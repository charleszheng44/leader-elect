use crate::error::{LeaderElectError, ThreadSafeResult};
use crate::message::{self, ElectResponse, Message, MessageType};
use clap::{AppSettings, Clap};
use derive_more::Display;
use log::{debug, error, info};
use std::collections::{BTreeMap, HashMap};
use std::io::{self, BufRead, BufReader, ErrorKind, Write};
use std::net::{SocketAddrV4, TcpListener, TcpStream};
use std::process;
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::{Duration, SystemTime};

const RETRY: u8 = 10;
const INIT_CONN_TIMEOUT: Duration = Duration::from_secs(10);
const ALIVE_TIMEOUT: Duration = Duration::from_secs(1);
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(2);
const LEADER_CHECK_INTERVAL: Duration = Duration::from_secs(3);

/// Run a node for leader election using the bully algorithm.
#[derive(Clap)]
#[clap(version = "1.0", author = "Charles Zheng. <charleszheng44@gmail.com>")]
#[clap(setting = AppSettings::ColoredHelp)]
pub struct Opts {
    /// ID of the current candidate
    #[clap(short, long)]
    id: u8,
    /// Peers' id, addresses pair e.g., --peers="1=0.0.0.0:1234,2=0.0.0.0:5678"
    #[clap(short, long)]
    peers: String,
    /// Address that can be visited by peers
    #[clap(short, long, default_value = "127.0.0.1:5678")]
    advertise_address: String,
    /// A level of verbosity, and can be used multiple times
    #[clap(short, long, default_value = "info")]
    pub log_level: String,
}

pub fn run(opts: &Opts) -> ThreadSafeResult<()> {
    // 1. initialize the node object
    let arc_rw_node = Arc::new(RwLock::new(Node::new(
        opts.id,
        &opts.peers,
        &opts.advertise_address,
    )?));
    debug!("node({}) initialized", opts.id);
    let mut handlers = HashMap::new();

    // 2. listen on the advertise address
    let ls_clone = Arc::clone(&arc_rw_node);
    handlers.insert(
        "message handler",
        thread::spawn(move || listen_and_serve(ls_clone)),
    );

    // 3. connect to peers
    {
        let mut node = arc_rw_node.write().unwrap();
        for (id, peer) in &mut node.peers.iter_mut() {
            (*peer).conn = Some(connect(peer.address)?);
            info!("peer({}) connected", id);
        }
    }

    // 4. send heartbeat if the node is the leader
    let hb_clone = Arc::clone(&arc_rw_node);
    handlers.insert("hearbeat handler", thread::spawn(|| heartbeat(hb_clone)));

    // 5. check if leader is alive
    let cl_clone = Arc::clone(&arc_rw_node);
    handlers.insert(
        "leader_checker handler",
        thread::spawn(|| check_leader(cl_clone)),
    );

    // 6. wait for all handlers to finish
    for (name, hdl) in handlers {
        if let Err(e) = hdl.join() {
            error!("{} failed: {:?}", name, e);
            process::exit(1);
        }
    }

    Ok(())
}

/// check_leader periodically checks if leader is malfunctioned
fn check_leader(locked_node: Arc<RwLock<Node>>) -> ThreadSafeResult<()> {
    loop {
        thread::sleep(LEADER_CHECK_INTERVAL);
        let mut node = locked_node.write().unwrap();
        let current_time = SystemTime::now();
        match node.last_leader_heartbeat {
            None => continue,
            Some(last_heartbeat) => {
                if current_time.duration_since(last_heartbeat)? > LEADER_CHECK_INTERVAL {
                    // the leader is melfunctioned, try to elect
                    node.leader = None;
                    node.last_leader_heartbeat = None;
                    if let ElectionResult::Win = elect(&mut node)? {
                        // won the election, announce self as the leader
                        node.leader = Some(node.id);
                        announce_victory(&mut node)?;
                    }
                }
            }
        }
    }
}

/// ElectionResult is the result of an election.
#[derive(Debug, Display)]
enum ElectionResult {
    #[display(fmt = "Win")]
    Win,
    #[display(fmt = "")]
    Fail,
}

/// announce_victory broadcasts `Victory` message to all peers with smaller id.
fn announce_victory(node: &mut Node) -> ThreadSafeResult<()> {
    for (_, peer) in node.peers.range_mut(..node.id) {
        send_message(node.id, peer, MessageType::Victory)?
    }
    Ok(())
}

/// elect tries to initiate an election.
fn elect(node: &mut Node) -> ThreadSafeResult<ElectionResult> {
    for (_, peer) in node.peers.range_mut(node.id + 1..) {
        // send Elect message to peers with larger id
        // TODO send elect to all peers concurrently?
        match send_elect_message(node.id, peer)? {
            ElectResponse::BuillerAlive => {
                // the builler is alive, abort the election.
                info!(
                    "node({}) fail to elect: the bullier({}) is alive",
                    node.id, peer.id
                );
                return Ok(ElectionResult::Fail);
            }
            // send elect message to the next builler
            ElectResponse::ResponseTimeOut => continue,
        }
    }
    info!(
        "all bullier are dead, node ({}) will be the leader",
        node.id
    );
    // if not receive Alive, announce self as the leader
    Ok(ElectionResult::Win)
}

/// heartbeat checks if the current node is the leader, if yes, it sends
/// heartbeat message to peers with smaller id.
fn heartbeat(locked_node: Arc<RwLock<Node>>) -> ThreadSafeResult<()> {
    loop {
        thread::sleep(HEARTBEAT_INTERVAL);
        let mut node = locked_node.write().unwrap();
        let node_id = node.id;
        if let Some(leader) = node.leader.as_ref() {
            if *leader != node_id {
                continue;
            }
            // the current node is the leader, send heartbeat to peers with
            // smaller id number.
            for (_, peer) in node.peers.range_mut(..node_id) {
                send_message(node_id, peer, MessageType::HeartBeat)?;
            }
        }
    }
}

/// send_message sends message with given `message_type` from `sender_id`
/// to `peer`.
fn send_message(sender_id: u8, peer: &mut Peer, message_type: MessageType) -> ThreadSafeResult<()> {
    let msg = Message::new(sender_id, message_type);
    debug!("send message {}", msg);
    if let Some(conn) = peer.conn.as_mut() {
        return Ok(conn.write_all(message::message_to_str(msg).as_bytes())?);
    }
    Err(new_box_err!(
        "try to send message through nonexist connection".to_owned()
    ))
}

/// send_elect_message sends `Elect` message to the given peer and waits for
/// reply from the peer. If a reply is received, the ElectResponse::BuillerAlive
/// will be returned. If no replies received within a designated time period,
/// the ElectResponse::ResponseTimeOut will be returned.
fn send_elect_message(sender_id: u8, peer: &mut Peer) -> ThreadSafeResult<ElectResponse> {
    send_message(sender_id, peer, MessageType::Elect)?;
    if let Some(mut conn) = peer.conn.as_mut() {
        conn.set_read_timeout(Some(ALIVE_TIMEOUT))?;
        let mut buf_rd = BufReader::new(&mut conn);
        let mut response = String::new();
        match buf_rd.read_line(&mut response) {
            Err(e) if e.kind() == ErrorKind::TimedOut => {
                conn.set_read_timeout(None)?;
                return Ok(ElectResponse::ResponseTimeOut);
            }
            Err(e) => {
                conn.set_read_timeout(None)?;
                return Err(Box::new(e));
            }
            Ok(num_bytes) => {
                if num_bytes == 0 {
                    return Err(new_box_err!(
                        "read zero bytes from the connection".to_owned()
                    ));
                }
                let rep_msg = message::str_to_message(&response)?;
                match rep_msg.get_message_type() {
                    MessageType::Alive => {
                        // receive acknowledge
                        return Ok(ElectResponse::BuillerAlive);
                    }
                    wrong_type @ _ => {
                        return Err(new_box_err!(format!(
                            "incorrect message type({})",
                            wrong_type
                        )));
                    }
                }
            }
        }
    }
    Err(new_box_err!(
        "try to send message through the nonexist connection".to_owned()
    ))
}

/// receive_message listens on `address` and passes received messages to
/// the channel
fn listen_and_serve(arc_rw_node: Arc<RwLock<Node>>) -> ThreadSafeResult<()> {
    let listener: TcpListener;
    {
        let adr = &arc_rw_node.read().unwrap().advertise_address;
        listener = TcpListener::bind(adr)?;
    }
    loop {
        let (conn, addr) = listener.accept()?;
        info!("accept connection from {}", addr);
        thread::spawn(move || handle_message(conn));
    }
}

/// handle_message keeps reading messages from the conn and handling
/// them accordingly.
fn handle_message(conn: TcpStream) -> ThreadSafeResult<()> {
    let mut buf_rd = BufReader::new(conn);
    loop {
        let _msg = message::receive_message(&mut buf_rd)?;
        // TODO handle message
    }
}

/// connect connects to the `address` and return a TcpStream on success.
fn connect(address: SocketAddrV4) -> ThreadSafeResult<TcpStream> {
    let mut count = RETRY;
    loop {
        match TcpStream::connect_timeout(&(address.into()), INIT_CONN_TIMEOUT) {
            Err(e) if io::ErrorKind::TimedOut == e.kind() && count > 0 => {
                count -= 1;
                continue;
            }
            Err(e) => {
                return Err(Box::new(e));
            }
            Ok(conn) => {
                return Ok(conn);
            }
        }
    }
}

#[derive(Debug)]
pub struct Node {
    id: u8,
    advertise_address: SocketAddrV4,
    peers: BTreeMap<u8, Peer>,
    leader: Option<u8>,
    last_leader_heartbeat: Option<SystemTime>,
}

#[derive(Debug)]
pub struct Peer {
    id: u8,
    address: SocketAddrV4,
    conn: Option<TcpStream>,
}

impl Node {
    pub fn new(id: u8, peer_str: &str, advertise_address: &str) -> ThreadSafeResult<Node> {
        Ok(Node {
            id,
            advertise_address: advertise_address.parse()?,
            peers: parse_peer_opt(peer_str.to_owned())?,
            leader: None,
            last_leader_heartbeat: None,
        })
    }
}

/// parse_peer_opt parses the value of the command line options `peers`
fn parse_peer_opt(peer_str: String) -> ThreadSafeResult<BTreeMap<u8, Peer>> {
    let mut peers = BTreeMap::new();
    for pair in peer_str.split(',') {
        let mut id_addr_pair = pair.split("=");
        let id = id_addr_pair
            .next()
            .ok_or(new_box_err!(peer_str.clone()))?
            .parse::<u8>()?;
        let address = id_addr_pair
            .next()
            .ok_or(new_box_err!(peer_str.clone()))?
            .parse::<SocketAddrV4>()?;
        peers.insert(
            id,
            Peer {
                id,
                address,
                conn: None,
            },
        );
    }
    Ok(peers)
}
