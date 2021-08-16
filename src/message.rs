use crate::error::LeaderElectError;
use derive_more::Display;
use std::str::FromStr;

#[derive(Display)]
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

#[derive(Display)]
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
