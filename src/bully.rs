use crate::error::{LeaderElectError, ThreadSafeResult};
use crate::message::{self, Message};
use clap::{AppSettings, Clap};
use log::{debug, info};
use std::io;
use std::io::BufReader;
use std::net::{SocketAddrV4, TcpListener, TcpStream};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

const RETRY: u8 = 10;
const TIMEOUT: Duration = Duration::from_secs(10);

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
    let mut node = Node::new(opts.id, &opts.peers, &opts.advertise_address)?;
    debug!("node({}) initialized", node.id);

    // 2. listen on the advertise address
    let (sender, receiver) = mpsc::channel();
    let addr = node.advertise_address.clone();
    let message_receiver = thread::spawn(move || receive_messages(addr, sender));

    // 3. connect to peers
    for peer in &mut node.peers {
        (*peer).conn = Some(connect(peer.address)?);
        info!("peer({}) connected", peer.id);
    }

    // 4. process messages
    let message_handler = thread::spawn(|| process_message(receiver));

    // 5. wait for listener and message_handler to finish
    message_receiver
        .join()
        .expect("fail to wait for the listener thread")?;
    message_handler
        .join()
        .expect("fail to wait for the message handler")?;

    Ok(())
}

/// receive_message listens on `address` and passes received messages to
/// the channel
fn receive_messages(address: SocketAddrV4, sender: mpsc::Sender<Message>) -> ThreadSafeResult<()> {
    let listener = TcpListener::bind(address)?;
    loop {
        let (conn, addr) = listener.accept()?;
        info!("accept connection from {}", addr);
        let tmp_sender = sender.clone();
        thread::spawn(move || read_message(conn, tmp_sender));
    }
}

/// process_message reads messages from the channel and changes the node's
/// state accordingly
/// TODO not implement yet
fn process_message(receiver: mpsc::Receiver<Message>) -> ThreadSafeResult<()> {
    loop {
        let msg = receiver.recv()?;
        debug!("processing message {}", msg);
    }
}

/// pass_message keeps reading messages from the conn and passing them to
/// the channel
fn read_message(conn: TcpStream, sender: mpsc::Sender<Message>) -> ThreadSafeResult<()> {
    let mut buf_rd = BufReader::new(conn);
    loop {
        let msg = message::receive_message(&mut buf_rd)?;
        sender.send(msg)?;
    }
}

/// connect connects to the `address` and return a TcpStream on success.
fn connect(address: SocketAddrV4) -> ThreadSafeResult<TcpStream> {
    let mut count = RETRY;
    loop {
        match TcpStream::connect_timeout(&(address.into()), TIMEOUT) {
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
    peers: Vec<Peer>,
    leader: Option<Peer>,
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
        })
    }
}

/// parse_peer_opt parses the value of the command line options `peers`
fn parse_peer_opt(peer_str: String) -> ThreadSafeResult<Vec<Peer>> {
    let mut peers = vec![];
    for pair in peer_str.split(',') {
        let mut id_addr_pair = pair.split("=");
        peers.push(Peer {
            id: id_addr_pair
                .next()
                .ok_or(new_box_err!(peer_str.clone()))?
                .parse::<u8>()?,
            address: id_addr_pair
                .next()
                .ok_or(new_box_err!(peer_str.clone()))?
                .parse::<SocketAddrV4>()?,
            conn: None,
        });
    }
    Ok(peers)
}
