//! Message parsing and serializing from ud3tn

use std::{array::TryFromSliceError, borrow::Cow, string::FromUtf8Error, time::{Duration, SystemTime}};
use thiserror::Error;

/// An ud3tn message received or sent to node
#[derive(PartialEq, Debug, Clone)]
#[non_exhaustive]
pub enum Message<'a> {
    /// Cositive acknowledgment
    Ack,

    /// Negative acknowledgment
    Nack,

    /// EID registration request
    /// (Agent identifier)
    Register(String),

    /// Connection establishment notice
    /// (Node EID)
    Welcome(String),

    /// Bundle transmission request
    /// (Destination EID, Payload data)
    SendBundle(String, Cow<'a, [u8]>),

    /// Bundle reception message
    /// (Destination EID, Payload data)
    RecvBundle(String, Cow<'a, [u8]>),

    /// Bundle transmission confirmation
    SendConfirm(BundleIdentifier),

    /// Bundle cancellation request
    CancelBundle(BundleIdentifier),

    /// Connection liveliness check
    Ping,

    /// Unimplemented - BIBE Bundle transmission request
    SendBIBE(String, Cow<'a, [u8]>),

    /// Unimplmented - BIBE Bundle reception message
    RecvBIBE(String, Cow<'a, [u8]>),
}

/// Identifier of a prevously sent bundle
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct BundleIdentifier(pub [u8;8]);

impl BundleIdentifier {
    fn is_v2_format(&self) -> bool {
        self.0[0] & 0b10000000 == 0b10000000
    }

    fn contains_creation_time(&self) -> bool {
        self.is_v2_format() && (self.0[0] & 0b010000000 == 0b010000000)
    }

    /// Bundle creation time
    /// 
    /// [None] if identifier is in older format or not included
    pub fn creation_time(&self) -> Option<DtnTime> {
        if !self.contains_creation_time() {
            return None
        }

        let time: u64 = u64::from_be_bytes([
            0,
            0,
            self.0[0] & 0b00111111,
            self.0[1],
            self.0[2],
            self.0[3],
            self.0[4],
            self.0[5],
        ]);

        Some(DtnTime(time))
    }

    /// Bundle sequence number
    /// 
    /// [None] if bundle identifier is in older format
    pub fn sequence_number(&self) -> Option<u64> {
        if !self.is_v2_format() {
            return None
        }

        if self.contains_creation_time() {
            let seq: u16 = u16::from_be_bytes([
                    self.0[6],
                    self.0[7]
                ]);
            Some(seq as u64)
        } else {
            let seq: u64 = u64::from_be_bytes([
                    self.0[0] & 0b00111111,
                    self.0[1],
                    self.0[2],
                    self.0[3],
                    self.0[4],
                    self.0[5],
                    self.0[6],
                    self.0[7]
                ]);
            Some(seq)
        }

    }
}

impl From<u64> for BundleIdentifier {
    fn from(value: u64) -> Self {
        Self::from(value.to_be_bytes())
    }
}

impl From<[u8;8]> for BundleIdentifier {
    fn from(value: [u8;8]) -> Self {
        BundleIdentifier(value)
    }
}

/// A timestamp relative to DTN EPOCH
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct DtnTime(u64);

impl DtnTime {
    /// Get system time from this DTN Time
    pub fn as_system_time(&self) -> SystemTime {
        SystemTime::from(*self)
    }
}

#[cfg(feature = "chrono")]
impl DtnTime {
    /// Get chrono DateTime from this DTN Time
    pub fn as_datetime(&self) -> chrono::NaiveDateTime {
        chrono::NaiveDateTime::from(*self)
    }
}

#[cfg(feature = "chrono")]
impl From<DtnTime> for chrono::NaiveDateTime {
    fn from(value: DtnTime) -> Self {
        chrono::DateTime::UNIX_EPOCH.naive_utc() +
            Duration::from_secs(946681200) // Number of seconds elapsed between UNIX EPOCH (1970-01-01T00:00:00)
                                           // and DTN EPOCH (2000-01-01T00:00:00)
            + Duration::from_millis(value.0)
    }
}

impl From<u64> for DtnTime {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl From<DtnTime> for SystemTime {
    fn from(value: DtnTime) -> Self {
        SystemTime::UNIX_EPOCH +
            Duration::from_secs(946681200) // Number of seconds elapsed between UNIX EPOCH (1970-01-01T00:00:00)
                                           // and DTN EPOCH (2000-01-01T00:00:00)
            + Duration::from_millis(value.0)
    }
}

impl<'a> Message<'a> {
    
    /// Serialize this message to bytes ready to be sended to ud3tn
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut result = vec![0x1 << 4];

        result[0] |= match self {
            Message::Ack => 0x0,
            Message::Nack => 0x1,
            Message::Register(_) => 0x2,
            Message::SendBundle(_, _) => 0x3,
            Message::RecvBundle(_, _) => 0x4,
            Message::SendConfirm(_) => 0x5,
            Message::CancelBundle(_) => 0x6,
            Message::Welcome(_) => 0x7,
            Message::Ping => 0x8,
            Message::SendBIBE(_, _) => todo!("BIBE not implemented"),
            Message::RecvBIBE(_, _) => todo!("BIBE not implemented"),
        };

        match self {
            Message::Register(agent_id) => {
                append_string(&mut result, agent_id);
            },
            Message::Welcome(node_eid) => {
                append_string(&mut result, node_eid);
            },
            Message::SendBundle(destination_eid, payload) => {
                append_string(&mut result, destination_eid);
                append_bytes(&mut result, &payload)
            },
            Message::RecvBundle(source_eid, payload) => {
                append_string(&mut result, source_eid);
                append_bytes(&mut result, &payload)
            },
            Message::SendConfirm(bundle_id) => 
                result.append(&mut Vec::from((bundle_id).0)),
            Message::CancelBundle(bundle_id) =>  
                result.append(&mut Vec::from((bundle_id).0)),
            _ => {}
        };

        return result;
    }

    /// Parse an array of bytes to a message
    pub fn parse(bytes: &[u8]) -> Result<Self, ParseError> {
        Self::parse_buffer(bytes).map(|it| it.0)
    }

    /// Parse an array of bytes to a message and return consumed bytes
    /// 
    /// Returns a tuple of (Parsed message, number of bytes consumed in buffer)
    pub fn parse_buffer(bytes: &[u8]) -> Result<(Self, usize), ParseError> {
        let version = (bytes[0] & 0b11110000) >> 4;

        if version != 0x1 {
            return Err(ParseError::VersionNotSupported);
        }

        let message_type = bytes[0] & 0b00001111;
        let mut offset = 1;

        let message = match message_type {
            0x0 => Self::Ack,
            0x1 => Self::Nack,
            0x2 => {
                let eid_length = u16::from_be_bytes(bytes[offset..offset+2].try_into()?) as usize;
                offset += 2;

                let eid = String::from_utf8(bytes[offset..offset+eid_length].into())?;
                offset += eid_length;

                Message::Register(eid)
            }
            0x3 => {
                let eid_length = u16::from_be_bytes(bytes[offset..offset+2].try_into()?) as usize;
                offset += 2;

                let dest_eid = String::from_utf8(bytes[offset..offset+eid_length].into())?;
                offset += eid_length;

                let payload_length = u64::from_be_bytes(bytes[offset..offset+8].try_into()?) as usize;
                offset += 8;

                if bytes.len() < offset+payload_length {
                    return Err(ParseError::UnexpectedEnd)
                }

                let payload = Cow::from(Vec::from(&bytes[offset..offset+payload_length]));
                offset += payload_length;

                Message::SendBundle(dest_eid, payload)
            }
            0x4 => {
                let eid_length = u16::from_be_bytes(bytes[offset..offset+2].try_into()?) as usize;
                offset += 2;

                let source_eid = String::from_utf8(bytes[offset..offset+eid_length].into())?;
                offset += eid_length;

                let payload_length = u64::from_be_bytes(bytes[offset..offset+8].try_into()?) as usize;
                offset += 8;

                if bytes.len() < offset+payload_length {
                    return Err(ParseError::UnexpectedEnd)
                }

                let payload = Cow::from(Vec::from(&bytes[offset..offset+payload_length]));
                offset += payload_length;

                Message::RecvBundle(source_eid, payload)
            }
            0x5 => {
                let bundle_id:[u8;8] = bytes[offset..offset+8].try_into()?;
                offset += 8;

                Message::SendConfirm(BundleIdentifier(bundle_id))
            }
            0x6 => {
                let bundle_id:[u8;8] = bytes[offset..offset+8].try_into()?;
                offset += 8;

                Message::CancelBundle(BundleIdentifier(bundle_id))
            }
            0x7 => {
                let eid_length:usize = u16::from_be_bytes(bytes[offset..offset+2].try_into()?) as usize;
                offset += 2;

                let eid = String::from_utf8(bytes[offset..offset+eid_length].into())?;
                offset += eid_length;

                Message::Welcome(eid)
            }
            0x8 => Self::Ping,
            0x9 => return Err(ParseError::UnknownType(0x9)), //todo BIBE not implemented
            0xA => return Err(ParseError::UnknownType(0xA)), //todo BIBE not implemented
            _ => return Err(ParseError::UnknownType(message_type))
        };

        Ok((message, offset))
    }
}

/// Append a string to a buffer including its length before it
fn append_string(target: &mut Vec<u8>, str: &String){
    target.append(&mut Vec::from((str.len() as u16).to_be_bytes()));
    target.append(&mut Vec::from(str.as_bytes()));
}

/// Append a byte array to a buffer including its length before it
fn append_bytes(target: &mut Vec<u8>, bytes: &[u8]){
    target.append(&mut Vec::from((bytes.len() as u64).to_be_bytes()));
    target.append(&mut bytes.iter().cloned().collect());
}

impl<'a> Into<Vec<u8>> for Message<'a> {
    fn into(self) -> Vec<u8> {
        self.to_bytes()
    }
}

impl<'a> TryFrom<Vec<u8>> for Message<'a> {
    type Error = ParseError;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        Self::parse(&value)
    }
}

/// Parsing error of message
#[derive(Debug, Error, Clone)]
pub enum ParseError {
    /// Parsed message protocol version isn't supported (or provided message is not a valid ud3tn message)
    #[error("Version byte not supported")]
    VersionNotSupported,

    /// Message type isn't supported (or provided message is not a valid ud3tn message)
    #[error("Unknown message type {0}")]
    UnknownType(u8),

    /// No more bytes to read but message wasn't finished
    #[error("Unexpected end of message")]
    UnexpectedEnd,

    /// A parsed string in message isn't a valid utf8 string
    #[error("Invalid utf8 string {0}")]
    Utf8Error(#[from] FromUtf8Error),
}

impl From<TryFromSliceError> for ParseError {
    fn from(_: TryFromSliceError) -> Self {
        Self::UnexpectedEnd
    }
}

/// A bundle received from node
#[derive(Debug)]
pub struct ReceivedBundle {
    /// Source endpoint ID of this bundle
    pub source: Option<String>,
    /// Content of this bundle
    pub payload: Vec<u8>
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;
    use crate::message::{BundleIdentifier, Message};

    #[test]
    fn test_ack_to_bytes(){
        assert_eq!(Message::Ack.to_bytes(), vec![0b00010000])
    }

    #[test]
    fn test_ack_parse(){
        assert_eq!(Message::parse(&vec![0b00010000]).unwrap(), Message::Ack)
    }

    #[test]
    fn test_nack_to_bytes(){
        assert_eq!(Message::Nack.to_bytes(), vec![0b00010001])
    }

    #[test]
    fn test_nack_parse(){
        assert_eq!(Message::parse(&vec![0b00010001]).unwrap(), Message::Nack)
    }

    #[test]
    fn test_register_to_bytes(){
        assert_eq!(
            Message::Register("rust_test".into()).to_bytes(),
            vec![0b00010010, // Declaration
                0, 9, // Length
                0b01110010,0b01110101,0b01110011,0b01110100,0b01011111,0b01110100,0b01100101,0b01110011,0b01110100 // EID
                ])
    }

    #[test]
    fn test_register_parse(){
        assert_eq!(
            Message::parse(&vec![0b00010010, // Declaration
                0, 9, // Length
                0b01110010,0b01110101,0b01110011,0b01110100,0b01011111,0b01110100,0b01100101,0b01110011,0b01110100 // EID
                ]).unwrap(),
            Message::Register("rust_test".into()))
    }

    #[test]
    fn test_send_bundle_to_bytes(){
        let payload:Vec<u8> = "Hello world !".into();

        assert_eq!(
            Message::SendBundle(
                "dtn://rust-lang.org/rust_test".into(),
                Cow::from(&payload)
            ).to_bytes(), 
            vec![0b00010011, // Declaration
                0, 29, // Length
                0b01100100,0b01110100,0b01101110,0b00111010,0b00101111,0b00101111,0b01110010,0b01110101,0b01110011,0b01110100,0b00101101,0b01101100,0b01100001,0b01101110,0b01100111,0b00101110,0b01101111,0b01110010,0b01100111,0b00101111,0b01110010,0b01110101,0b01110011,0b01110100,0b01011111,0b01110100,0b01100101,0b01110011,0b01110100, // Destination EID
                0, 0, 0, 0, 0, 0, 0, 13, // Payload length
                0b01001000,0b01100101,0b01101100,0b01101100,0b01101111,0b00100000,0b01110111,0b01101111,0b01110010,0b01101100,0b01100100,0b00100000,0b00100001 // Payload
                ])
    }

    #[test]
    fn test_send_bundle_parse(){
        let payload:Vec<u8> = "Hello world !".into();

        assert_eq!(
            Message::parse(&vec![0b00010011, // Declaration
                0, 29, // Length
                0b01100100,0b01110100,0b01101110,0b00111010,0b00101111,0b00101111,0b01110010,0b01110101,0b01110011,0b01110100,0b00101101,0b01101100,0b01100001,0b01101110,0b01100111,0b00101110,0b01101111,0b01110010,0b01100111,0b00101111,0b01110010,0b01110101,0b01110011,0b01110100,0b01011111,0b01110100,0b01100101,0b01110011,0b01110100, // Destination EID
                0, 0, 0, 0, 0, 0, 0, 13, // Payload length
                0b01001000,0b01100101,0b01101100,0b01101100,0b01101111,0b00100000,0b01110111,0b01101111,0b01110010,0b01101100,0b01100100,0b00100000,0b00100001 // Payload
                ]).unwrap(),
            Message::SendBundle(
                "dtn://rust-lang.org/rust_test".into(),
                Cow::from(&payload)
            ))
    }

    #[test]
    fn test_recv_bundle_to_bytes(){
        let payload:Vec<u8> = "Hello world !".into();

        assert_eq!(
            Message::RecvBundle(
                "dtn://rust-lang.org/rust_test".into(),
                (&payload).into()
            ).to_bytes(), 
            vec![0b00010100, // Declaration
                0, 29, // Length
                0b01100100,0b01110100,0b01101110,0b00111010,0b00101111,0b00101111,0b01110010,0b01110101,0b01110011,0b01110100,0b00101101,0b01101100,0b01100001,0b01101110,0b01100111,0b00101110,0b01101111,0b01110010,0b01100111,0b00101111,0b01110010,0b01110101,0b01110011,0b01110100,0b01011111,0b01110100,0b01100101,0b01110011,0b01110100, // Source EID
                0, 0, 0, 0, 0, 0, 0, 13, // Payload length
                0b01001000,0b01100101,0b01101100,0b01101100,0b01101111,0b00100000,0b01110111,0b01101111,0b01110010,0b01101100,0b01100100,0b00100000,0b00100001 // Payload
                ])
    }

    #[test]
    fn test_recv_bundle_parse(){
        let payload:Vec<u8> = "Hello world !".into();

        assert_eq!(
            Message::parse(&vec![0b00010100, // Declaration
                0, 29, // Length
                0b01100100,0b01110100,0b01101110,0b00111010,0b00101111,0b00101111,0b01110010,0b01110101,0b01110011,0b01110100,0b00101101,0b01101100,0b01100001,0b01101110,0b01100111,0b00101110,0b01101111,0b01110010,0b01100111,0b00101111,0b01110010,0b01110101,0b01110011,0b01110100,0b01011111,0b01110100,0b01100101,0b01110011,0b01110100, // Source EID
                0, 0, 0, 0, 0, 0, 0, 13, // Payload length
                0b01001000,0b01100101,0b01101100,0b01101100,0b01101111,0b00100000,0b01110111,0b01101111,0b01110010,0b01101100,0b01100100,0b00100000,0b00100001 // Payload
                ]).unwrap(),
            Message::RecvBundle(
                "dtn://rust-lang.org/rust_test".into(),
                (&payload).into()
            ))
    }

    #[test]
    fn test_sendconfirm_to_bytes(){
        assert_eq!(
            Message::SendConfirm(BundleIdentifier(735469895_u64.to_be_bytes())).to_bytes(),
            vec![0b00010101, // Declaration
            0, 0, 0, 0, 0b00101011, 0b11010110, 0b01100001, 0b01000111 //Bundle ID
            ])
    }

    #[test]
    fn test_sendconfirm_parse(){
        assert_eq!(
            Message::parse(&vec![0b00010101, 0, 0, 0, 0, 0b00101011, 0b11010110, 0b01100001, 0b01000111]).unwrap(),
            Message::SendConfirm(BundleIdentifier(735469895_u64.to_be_bytes())))
    }

    #[test]
    fn test_bundle_cancelled_to_bytes(){
        assert_eq!(
            Message::CancelBundle(BundleIdentifier(735469895_u64.to_be_bytes())).to_bytes(),
            vec![0b00010110, // Declaration
            0, 0, 0, 0, 0, 0b00011010, 0b01000010, 0b00111101, //Bundle ID
            ])
    }

    #[test]
    fn test_bundle_cancelled_parse(){
        assert_eq!(
            Message::parse(&vec![0b00010110, 0, 0, 0, 0, 0, 0b00011010, 0b01000010, 0b00111101]).unwrap(),
            Message::CancelBundle(BundleIdentifier(735469895_u64.to_be_bytes())))
    }

    #[test]
    fn test_welcome_to_bytes(){
        assert_eq!(
            Message::Welcome("dtn://rust-lang.org/".into()).to_bytes(), 
            vec![0b00010111, // Declaration
                0, 20, // Length
                0b01100100,0b01110100,0b01101110,0b00111010,0b00101111,0b00101111,0b01110010,0b01110101,0b01110011,0b01110100,0b00101101,0b01101100,0b01100001,0b01101110,0b01100111,0b00101110,0b01101111,0b01110010,0b01100111,0b00101111, // Node EID
                ])
    }

    #[test]
    fn test_welcome_parse(){
        assert_eq!(
            Message::parse(&vec![0b00010111, // Declaration
                0, 20, // Length
                0b01100100,0b01110100,0b01101110,0b00111010,0b00101111,0b00101111,0b01110010,0b01110101,0b01110011,0b01110100,0b00101101,0b01101100,0b01100001,0b01101110,0b01100111,0b00101110,0b01101111,0b01110010,0b01100111,0b00101111, // Node EID
                ]).unwrap(),
            Message::Welcome("dtn://rust-lang.org/".into()))
    }

    #[test]
    fn test_ping_to_bytes(){
        assert_eq!(Message::Ping.to_bytes(), vec![0b00011000])
    }

    #[test]
    fn test_ping_parse(){
        assert_eq!(Message::parse(&vec![0b00011000]).unwrap(), Message::Ping)
    }

}