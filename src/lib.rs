#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

use std::{io::{Read, Write, BufReader, BufRead}, os::unix::net::UnixStream, fmt::Debug, thread, time::Duration};
use std::path::Path;

use config::ConfigBundle;
use message::{Message, ParseError};
use thiserror::Error;

pub mod message;
pub mod config;

/// Any stream matching requirements to be used as an ud3tn aap source
/// 
/// You shouldn't use it directly. Use [Agent::connect_unix] to connect to a unix stream.
pub trait Ud3tnAapStream: Read + Write + Debug + Send {}

impl<T: Read + Write + Debug + Send> Ud3tnAapStream for T {}

/// Agent creating an AAP on a ud3tn node through a stream
/// 
/// Represents an AAP on a ud3tn node. An AAP expose en endpoint under node's current EID and a defined `agent_id`.
/// 
/// # Examples
/// 
/// Using a socket file with [`std::os::unix::net::UnixStream`] to expose en endpoint on `dtn://[your-node-eid]/my-agent`
/// ```rust,no_run
/// use std::os::unix::net::UnixStream;
/// use ud3tn_aap::Agent;
/// 
/// let connection = Agent::connect(
///     UnixStream::connect("archipel-core/ud3tn.socket").unwrap(),
///     "my-agent".into()
/// ).unwrap();
/// println!("Connected to {0} as {0}{1}", connection.node_eid, connection.agent_id)
/// ```
/// 
/// Using a [`std::net::TcpStream`] to expose en endpoint on `dtn://[your-node-eid]/my-agent`
/// ```rust,no_run
/// use std::net::TcpStream;
/// use ud3tn_aap::Agent;
/// 
/// let connection = Agent::connect(
///     TcpStream::connect("127.0.0.1:34254").unwrap(),
///     "my-agent".into()
/// ).unwrap();
/// println!("Connected to {0} as {0}{1}", connection.node_eid, connection.agent_id)
/// ```
#[derive(Debug)]
pub struct Agent {
    /// Stream used for communication with ud3tn
    pub stream: BufReader<Box<dyn Ud3tnAapStream>>,

    /// Current registered Agent ID
    pub agent_id: String,

    /// EID of currently connected node
    pub node_eid: String
}

impl Agent {

    /// Connect to ud3tn using a unix socket and an `agent_id`.
    /// Blocks until a sucessful connection or Error.
    /// 
    /// Will establish a communication with ud3tn, wait for WELCOME message and will register agent ID
    /// This operation is blocking until the connection is available and working
    #[cfg(unix)]
    pub fn connect_unix(unix_sock_path: &Path, agent_id: String) -> Result<Self, Error> {
        let stream = UnixStream::connect(unix_sock_path)?;
        stream.set_nonblocking(true)?;
        Self::connect_stream(Box::new(stream), agent_id)
    }

    // TODO TCP Connect

    /// Connect to ud3tn with provided stream using the the given `agent_id`. Blocks until a sucessful connection or Error.
    /// 
    /// Will establish a communication with ud3tn, wait for WELCOME message and will register agent ID
    /// This operation is blocking until the connection is available and working
    pub fn connect_stream(
        stream: Box<dyn Ud3tnAapStream>,
        agent_id: String
    ) -> Result<Self, Error> {

        let mut stream = BufReader::new(stream);

        match Self::recv_from(&mut stream, true)? {
            Message::Welcome(node_eid) => {
                Self::send_message_to(
                    &mut stream,
                    Message::Register(agent_id.clone())
                )?;
                Ok(Self {
                    stream,
                    agent_id,
                    node_eid
                })
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
        Self::send_message_unchecked_to(&mut self.stream.get_mut(), Message::SendBundle(destination_eid, payload.into()))?;
        match Self::recv_from(&mut self.stream, true)? {
            Message::SendConfirm(id) => Ok(id),
            _ => Err(Error::UnexpectedMessage)
        }
    }

    /// Block until a bundle is received from ud3tn node adressed to this agent
    /// 
    /// If something other than a bundle is received [`Err(Error::UnexpectedMessage)`] is returned
    pub fn recv_bundle(&mut self) -> Result<(String, Vec<u8>), Error>{
        match Self::recv_from(&mut self.stream, true)? {
            Message::RecvBundle(source, payload) => Ok((source, payload.into())),
            _ => Err(Error::UnexpectedMessage)
        }
    }

    /// Try to receive a bundle from ud3tn node adressed to this agent
    /// 
    /// If something other than a bundle is received [`Err(Error::UnexpectedMessage)`] is returned
    /// If no bundle is pending, return [`Err(Error:NoMessage)`]
    pub fn try_recv_bundle(&mut self) -> Result<(String, Vec<u8>), Error>{
        match Self::recv_from(&mut self.stream, false)? {
            Message::RecvBundle(source, payload) => Ok((source, payload.into())),
            _ => Err(Error::UnexpectedMessage)
        }
    }

    /// Send an AAP message to a stream and wait of a [`Message::ACK`] answer
    fn send_message_to<S: Ud3tnAapStream>(stream: &mut BufReader<S>, message: Message) -> Result<(), Error> {
        Self::send_message_unchecked_to(stream.get_mut(), message)?;
        match Self::recv_from(stream, true)? {
            Message::Ack => Ok(()),
            Message::Nack => Err(Error::FailedOperation),
            _ => Err(Error::UnexpectedMessage)
        }
    }

    /// Send an AAP message to a stream
    fn send_message_unchecked_to<T: Write>(stream: &mut T, message: Message) -> Result<(), Error> {
        stream.write_all(&message.to_bytes())?;
        Ok(())
    }

    /// Blocks until an AAP message is received from a stream
    fn recv_from<'b, S: BufRead>(
        stream: &mut S,
        block: bool
    ) -> Result<Message<'b>, Error> {
        loop {

            let buffer = {
                let mut result = None;
                while result.is_none() {
                    result  = match stream.fill_buf() {
                        Ok(b) => Ok(Some(b)),
                        Err(e) => match e.kind() {
                            std::io::ErrorKind::WouldBlock => {
                                if block {
                                    thread::sleep(Duration::from_millis(100));
                                    Ok(None)
                                } else {
                                    Err(Error::NoMessage)
                                }
                            },
                            _ => Err(Error::IOError(e))
                        },
                    }?;
                }
                result.unwrap()
            };

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

    /// Send a configuration bundle to ud3tn node
    pub fn send_config(&mut self, config:ConfigBundle) -> Result<(), Error> {
        match self.send_bundle(format!("{0}config", self.node_eid), &config.to_bytes()) {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
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
    MalformedMessage(#[from] ParseError),

    /// Nothing to receive
    #[error("Nothing to receive")]
    NoMessage
}