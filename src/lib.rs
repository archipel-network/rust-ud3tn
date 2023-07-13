use std::{io::{Read, Write, BufRead, BufReader}};

use message::{Message, ParseError};
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

    pub fn send_bundle(&mut self, destination_eid: String, payload:&[u8]) -> Result<u64, Error>{
        Self::send_message_unchecked_to(self.stream.get_mut(), Message::SENDBUNDLE(destination_eid, payload.into()))?;
        match Self::recv_from(&mut self.stream)? {
            Message::SENDCONFIRM(id) => Ok(id),
            _ => Err(Error::UnexpectedMessage)
        }
    }

    pub fn recv_bundle(&mut self) -> Result<(String, Vec<u8>), Error>{
        match Self::recv_from(&mut self.stream)? {
            Message::RECVBUNDLE(source, payload) => Ok((source, payload.into())),
            _ => Err(Error::UnexpectedMessage)
        }
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

    fn recv_from<'a>(stream: &mut BufReader<S>) -> Result<Message<'a>, Error> {
        loop {
            stream.fill_buf()?;
            let buffer = stream.buffer();
            let bytes_in_buffer = buffer.len();

            if bytes_in_buffer > 0 {
                match Message::parse_buffer(buffer) {
                    Ok((m, bytes_red)) => {
                        stream.consume(bytes_red);
                        return Ok(m);
                    },
                    Err(e) => match e {
                        message::ParseError::VersionNotSupported => {
                            stream.consume(bytes_in_buffer);
                            return Err(Error::MalformedMessage(e))
                        },
                        message::ParseError::UnknownType(_) => {
                            stream.consume(bytes_in_buffer);
                            return Err(Error::MalformedMessage(e))
                        },
                        _ => {}
                    },
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
    FailedOperation,

    #[error("Malformed message")]
    MalformedMessage(#[from] ParseError)
}