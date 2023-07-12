use std::{io::{Read, Write, BufRead, BufReader}};

use message::Message;
use thiserror::Error;

mod message;

#[derive(Debug)]
pub struct AAPConnection<S: Read + Write> {
    pub stream: BufReader<S>,
    pub agent_id: String,
    pub node_eid: String
}

impl<S: Read + Write> AAPConnection<S> {

    pub fn connect(stream: S, agent_id: String) -> Result<Self, Error> {
        let mut stream = BufReader::new(stream);

        match Self::recv_from(&mut stream)? {
            Message::WELCOME(node_eid) => {
                Self::send_message_to(&mut stream, Message::REGISTER(agent_id.clone()))?;
                Ok(Self { stream, agent_id, node_eid })
            },
            _ => Err(Error::UnexpectedMessage)
        }
    }

    fn send_message(&mut self, message: Message) -> Result<(), Error>{
        Self::send_message_to(&mut self.stream, message)
    }

    fn send_message_to(stream:&mut BufReader<S>, message: Message) -> Result<(), Error> {
        Self::send_message_unchecked_to(stream.get_mut(), message)?;
        match Self::recv_from(stream)? {
            Message::ACK => Ok(()),
            Message::NACK => Err(Error::FailedOperation),
            _ => Err(Error::UnexpectedMessage)
        }
    }

    fn send_message_unchecked_to<T: Write>(stream:&mut T, message: Message) -> Result<(), Error> {
        stream.write(&message.to_bytes())?;
        Ok(())
    }

    fn recv(&mut self) -> Result<Message, Error> {
        Self::recv_from(&mut self.stream)
    }

    fn recv_from<'a>(stream: &mut BufReader<S>) -> Result<Message<'a>, Error> {
        let mut message_buffer:Vec<u8> = Vec::new();

        loop {
            let mut buf = [0;255];
            let bytes_red = stream.read(&mut buf)?;
            message_buffer.append(&mut buf[0..bytes_red].into());

            if message_buffer.len() > 1 {
                match Message::parse(&message_buffer) {
                    Ok(m) => return Ok(m),
                    Err(_) => {},
                }
            }
        }
    }

}

#[derive(Debug, Error)]
pub enum Error {
    #[error("io Error")]
    IOError(#[from] std::io::Error),

    #[error("Unexpected message received")]
    UnexpectedMessage,

    #[error("Node responded with NACK")]
    FailedOperation
}