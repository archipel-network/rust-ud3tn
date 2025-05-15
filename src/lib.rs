#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

use std::{fmt::Debug, io::{Read, Write}, os::unix::net::UnixStream};
use std::path::Path;

use config::ConfigBundle;
use message::ParseError;
pub use message::{ReceivedBundle, BundleIdentifier, Message, DtnTime};
use thiserror::Error;

pub mod message;
pub mod config;

/// Any stream matching requirements to be used as an ud3tn aap source
/// 
/// You shouldn't use it directly. Use [Agent::connect_unix] to connect to a unix stream.
pub trait AapStream: Read + Write + Send {}

impl<T: Read + Write + Send> AapStream for T {}

/// Generic function available in all agents
pub trait BaseAgent<S: AapStream> {
    /// Send a single [Message::Ping] message a await a ACK response
    fn ping(&mut self) -> Result<(), Error>;

    /// Get node id this agent is connected to
    fn node_id(&self) -> &str;
}

/// An unregistered agent that can communicate with ud3tn/Archipel
#[derive(Debug)]
pub struct Agent<S: AapStream> {
    /// Stream used for communication with ud3tn
    stream: S,

    /// EID of currently connected node
    node_eid: String,

    recv_buffer: Vec<u8>
}

impl Agent<UnixStream> {
    /// Connect to ud3tn using a unix socket and an `agent_id`.
    /// Blocks until a sucessful connection or Error.
    /// 
    /// Will establish a communication with ud3tn, wait for WELCOME message and will register agent ID
    /// This operation is blocking until the connection is available and working
    #[cfg(unix)]
    pub fn connect_unix(unix_sock_path: &Path) -> Result<Self, Error> {
        let stream = UnixStream::connect(unix_sock_path)?;
        Self::new(stream)
    }
}

impl<S: AapStream> Agent<S> {

    /// Connect to ud3tn with provided stream using the the given `agent_id`. Blocks until a sucessful connection or Error.
    /// 
    /// Will establish a communication with ud3tn, wait for WELCOME message and will register agent ID
    /// This operation is blocking until the connection is available and working
    pub fn new(
        stream: S
    ) -> Result<Self, Error> {
        let mut new_self = Self {
            stream,
            node_eid: String::new(),
            recv_buffer: Vec::new()
        };

        match new_self.recv_message()? {
            Message::Welcome(node_eid) => {
                new_self.node_eid = node_eid
            }
            _ => return Err(Error::UnexpectedMessage)
        }

        Ok(new_self)
    }

    /// Register this agent to send and receive bundles
    pub fn register(mut self, agent_id: String) -> Result<RegisteredAgent<S>, Error>{
        self.send_request(Message::Register(agent_id.clone()))?;
        Ok(RegisteredAgent {
            inner: self,
            agent_id
        })
    }

    /// Send a message and await a [Message::Ack] or [Message::Nack]
    fn send_request(&mut self, request_msg: Message<'_>) -> Result<(), Error> {
        self.stream.write_all(request_msg.to_bytes().as_slice())?;
        let message = self.recv_message()?;
        match message {
            Message::Ack => Ok(()),
            Message::Nack => Err(Error::FailedOperation),
            _ => Err(Error::UnexpectedMessage)
        }
    }

    /// Receive a single message
    fn recv_message(&mut self) -> Result<Message, Error> {
        let mut buffer = [0;1024];
        loop {
            let byte_red = self.stream.read(&mut buffer)?;

            if byte_red > 0 {
                self.recv_buffer.extend_from_slice(&buffer[0..byte_red]);
            }

            let (mess, consumed_bytes) = match Message::parse_buffer(&self.recv_buffer) {
                Ok(it) => it,
                Err(message::ParseError::UnexpectedEnd) => {
                    if byte_red == 0 {
                        return Err(Error::UnexpectedEnd)
                    } else {
                        continue;
                    }
                },
                Err(e) => return Err(Error::MalformedMessage(e))
            };

            let remaning_buffer_len = self.recv_buffer[consumed_bytes..].len();
            self.recv_buffer.copy_within(consumed_bytes.., 0);
            self.recv_buffer.resize(remaning_buffer_len, 0);

            return Ok(mess)
        }
    }
}

impl<S:AapStream> BaseAgent<S> for Agent<S> {
    fn ping(&mut self) -> Result<(), Error> {
        self.send_request(Message::Ping)
    }

    fn node_id(&self) -> &str {
        &self.node_eid
    }
}

/// AAn agent that was registered and abto to send and receive bundles
pub struct RegisteredAgent<S: AapStream> {
    inner: Agent<S>,
    agent_id: String
}

impl<S: AapStream> RegisteredAgent<S> {

    /// Get currently registered agent id
    pub fn agent_id(&self) -> &str {
        &self.agent_id
    }

    /// Send a bundle to ud3tn node to route it
    /// 
    /// Bundle is sent with this agent as source.
    /// 
    /// Returns bundle identifier as [`u64`]
    pub fn send_bundle(&mut self, destination_eid: String, payload:&[u8]) -> Result<BundleIdentifier, Error>{
        let message = Message::SendBundle(destination_eid, std::borrow::Cow::Borrowed(payload));
        self.inner.stream.write_all(&message.to_bytes())?;
        match self.inner.recv_message()? {
            Message::SendConfirm(identifier) => Ok(identifier),
            _ => Err(Error::UnexpectedMessage)
        }
    }

    /// Block until a bundle is received from ud3tn node adressed to this agent
    /// 
    /// If something other than a bundle is received [`Err(Error::UnexpectedMessage)`] is returned
    pub fn recv_bundle(&mut self) -> Result<ReceivedBundle, Error> {
        match self.inner.recv_message()? {
            Message::RecvBundle(source, content) => Ok(ReceivedBundle {
                source: Some(source),
                payload: content.into_owned()
            }),
            _ => Err(Error::UnexpectedMessage)
        }
    }

    /// Try to receive a bundle from ud3tn node adressed to this agent
    /// 
    /// If something other than a bundle is received [`Err(Error::UnexpectedMessage)`] is returned
    /// If no bundle is pending, return [`Err(Error:NoMessage)`]
    pub fn try_recv_bundle(&mut self) -> Result<ReceivedBundle, Error>{
        todo!()
    }

    /// Send a configuration bundle to ud3tn node
    pub fn send_config(&mut self, config:ConfigBundle) -> Result<(), Error> {
        match self.send_bundle(format!("{0}config", self.inner.node_eid), &config.to_bytes()) {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }
}

impl<S:AapStream> BaseAgent<S> for RegisteredAgent<S> {
    fn ping(&mut self) -> Result<(), Error> {
        self.inner.ping()
    }

    fn node_id(&self) -> &str {
        self.inner.node_id()
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

    /// Stream ended before a message was fully received
    #[error("Unexpected end")]
    UnexpectedEnd
}