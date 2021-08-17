use crate::error::{LeaderElectError, ThreadSafeResult};
use derive_more::Display;
use std::io::{BufRead, Write};
use std::str::FromStr;

#[derive(Display, Debug, PartialEq)]
enum MessageType {
    #[display(fmt = "HeartBeat")]
    HeartBeat = 0,
    #[display(fmt = "Elect")]
    Elect,
    #[display(fmt = "Acknowledge")]
    Acknowledge,
}

impl FromStr for MessageType {
    type Err = Box<dyn std::error::Error + Send + Sync>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "0" => Ok(MessageType::HeartBeat),
            "1" => Ok(MessageType::Elect),
            "2" => Ok(MessageType::Acknowledge),
            _ => Err(new_box_err!("fail to read message_type".to_owned())),
        }
    }
}

#[derive(Display, Debug, PartialEq)]
#[display(fmt = "[message_type: {}, sender_id: {}]", message_type, sender_id)]
pub struct Message {
    message_type: MessageType,
    sender_id: u8,
}

impl FromStr for Message {
    type Err = Box<dyn std::error::Error + Send + Sync>;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut id_type = s.split(':');
        Ok(Message {
            sender_id: id_type
                .next()
                .ok_or(new_box_err!("fail to read id".to_owned()))?
                .parse::<u8>()?,
            message_type: id_type
                .next()
                .ok_or(new_box_err!("fail to read type".to_owned()))?
                .parse::<MessageType>()?,
        })
    }
}

fn str_to_message(inp_str: &str) -> ThreadSafeResult<Message> {
    inp_str.trim().parse()
}

fn message_to_str(msg: Message) -> String {
    format!("{}:{}", msg.sender_id, msg.message_type as u8)
}

pub fn send_message<T: Write>(msg: Message, mut stream: T) -> ThreadSafeResult<()> {
    Ok(stream.write_all(message_to_str(msg).as_bytes())?)
}

pub fn receive_message<T: BufRead>(ref mut stream: T) -> ThreadSafeResult<Message> {
    let mut str_buf = String::new();
    let num_bytes = stream.read_line(&mut str_buf)?;
    if num_bytes == 0 {
        return Err(new_box_err!("0 bytes read".to_owned()));
    }
    str_to_message(&str_buf)
}

#[cfg(test)]
mod test {
    use super::Message;
    #[test]
    fn from_str() {
        let msg_str_1 = "1:0";
        let msg_str_2 = "2:1";
        assert_ne!(
            msg_str_1.parse::<Message>().unwrap(),
            msg_str_2.parse::<Message>().unwrap()
        );

        let msg_str_3 = "3:2";
        let msg_str_4 = "3:2";
        assert_eq!(
            msg_str_3.parse::<Message>().unwrap(),
            msg_str_4.parse::<Message>().unwrap()
        );
    }
}
