#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

use std::io::{Read, Write, BufRead, BufReader};

use message::{Message, ParseError};
use thiserror::Error;

pub mod message;
pub mod config;

/// Agent creating an AAP on a ud3tn node through a stream
/// 
/// Represents an AAP on a ud3tn node. An AAP expose en endpoint under node's current EID and a defined `agent_id`.
/// 
/// 
/// Using a socket file with [`std::os::unix::net::UnixStream`] to expose en endpoint on `dtn://[your-node-eid]/my-agent`
/// ```rust,ignore
/// let connection = Agent::connect(
///     UnixStream::connect("archipel-core/ud3tn.socket").unwrap(),
///     "my-agent".into()
/// ).unwrap();
/// println!("Connected to {0} as {0}{1}", connection.node_eid, connection.agent_id)
/// ```
/// 
/// Using a [`std::net::TcpStream`] to expose en endpoint on `dtn://[your-node-eid]/my-agent`
/// ```rust,ignore
/// let connection = Agent::connect(
///     TcpStream::connect("127.0.0.1:34254").unwrap(),
///     "my-agent".into()
/// ).unwrap();
/// println!("Connected to {0} as {0}{1}", connection.node_eid, connection.agent_id)
/// ```
#[derive(Debug)]
pub struct Agent<S: Read + Write> {
    /// Stream used for communication with ud3tn
    pub stream: BufReader<S>,

    /// Current registered Agent ID
    pub agent_id: String,

    /// EID of currently connected node
    pub node_eid: String
}

impl<S: Read + Write> Agent<S> {

    /// Connect to ud3tn with provided stream using the the given `agent_id`. Blocks until a sucessful connection or Error.
    /// 
    /// Will establish a communication with ud3tn, wait for WELCOME message and will register agent ID
    /// This operation is blocking until the connection is available and working
    pub fn connect(stream: S, agent_id: String) -> Result<Self, Error> {
        let mut stream = BufReader::new(stream);

        match Self::recv_from(&mut stream)? {
            Message::Welcome(node_eid) => {
                Self::send_message_to(&mut stream, Message::Register(agent_id.clone()))?;
                Ok(Self { stream, agent_id, node_eid })
            },
            _ => Err(Error::UnexpectedMessage)
        }
    }

    /// Send a bundle to ud3tn node to route it
    /// 
    /// Bundle is sent with this agent as source.
    /// 
    /// Returns bundle identifier as [`u64`]
    pub fn send_bundle(&mut self, destination_eid: String, payload:&[u8]) -> Result<u64, Error>{
        Self::send_message_unchecked_to(self.stream.get_mut(), Message::SendBundle(destination_eid, payload.into()))?;
        match Self::recv_from(&mut self.stream)? {
            Message::SendConfirm(id) => Ok(id),
            _ => Err(Error::UnexpectedMessage)
        }
    }

    /// Block until a bundle is received from ud3tn node adressed to this agent
    /// 
    /// If something other than a bundle is received [`Err(Error::UnexpectedMessage)`] is returned
    pub fn recv_bundle(&mut self) -> Result<(String, Vec<u8>), Error>{
        match Self::recv_from(&mut self.stream)? {
            Message::RecvBundle(source, payload) => Ok((source, payload.into())),
            _ => Err(Error::UnexpectedMessage)
        }
    }

    /// Send an AAP message to a stream and wait of a [`Message::ACK`] answer
    fn send_message_to(stream:&mut BufReader<S>, message: Message) -> Result<(), Error> {
        Self::send_message_unchecked_to(stream.get_mut(), message)?;
        match Self::recv_from(stream)? {
            Message::Ack => Ok(()),
            Message::Nack => Err(Error::FailedOperation),
            _ => Err(Error::UnexpectedMessage)
        }
    }

    /// Send an AAP message to a stream
    fn send_message_unchecked_to<T: Write>(stream:&mut T, message: Message) -> Result<(), Error> {
        stream.write(&message.to_bytes())?;
        Ok(())
    }

    /// Blocks until an AAP message is received from a stream
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

/// An error during communication with ud3tn node
#[derive(Debug, Error)]
pub enum Error {
    /// IO Error during sending or receiving of data on provided stream
    #[error("io Error")]
    IOError(#[from] std::io::Error),

    /// Message received wasn't expected
    /// 
    /// Waiting for [`Message::Ack`] message but something else came
    #[error("Unexpected message received")]
    UnexpectedMessage,

    /// Asked operation failed with a [`Message::Nack`] from ud3tn node
    #[error("Node responded with NACK")]
    FailedOperation,

    /// An error occured during message parsing
    #[error("Malformed message")]
    MalformedMessage(#[from] ParseError)
}