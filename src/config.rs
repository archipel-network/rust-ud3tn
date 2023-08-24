//! Bundle used for ud3tn contact configuration

use std::time::SystemTime;

/// ud3tn config bundle
#[derive(Debug, Clone)]
pub enum ConfigBundle {
    /// Add a new available contact
    AddContact {
        /// EID of the other node in contact
        eid: String,

        /// An integer number between 100 and 1000 and represent the expected likelihood that a future contact with the given node will be observed, divided by 10000
        reliability: Option<i32>,

        /// CLA address used in this contact
        /// Uses the same string representation as ud3tn and consists of the convergence layer adapter and the node address
        /// e.g., `(tcpclv3:127.0.0.1:1234)`
        cla_address: String,

        /// Reachable EID through this contact
        reaches_eid: Vec<String>,

        /// Future contact of this node
        contacts: Vec<Contact>,
    },

    /// Replace an existing contact
    ReplaceContact {
        /// EID of the other node in contact
        eid: String,

        /// An integer number between 100 and 1000 and represent the expected likelihood that a future contact with the given node will be observed, divided by 10000
        reliability: Option<i32>,

        /// CLA address used in this contact
        /// Uses the same string representation as ud3tn and consists of the convergence layer adapter and the node address
        /// e.g., `(tcpclv3:127.0.0.1:1234)`
        cla_address: Option<String>,

        /// Reachable EID through this contact
        reaches_eid: Vec<String>,

        /// Future contact of this node
        contacts: Vec<Contact>,
    },

    /// Delete an existing contact (Contact EID)
    DeleteContact(String),
}

impl ConfigBundle {
    /// Serialize this config bundle as string
    pub fn to_string(&self) -> String {
        let result: String = match self {
            ConfigBundle::AddContact {
                eid,
                reliability,
                cla_address,
                reaches_eid,
                contacts,
            } => {
                // Command
                let mut result = format!("1({0})", eid);

                // Reliability
                result = match reliability {
                    Some(r) => result + &format!(",{}", r),
                    None => result,
                };

                // CLA
                result = result + &format!(":({})", cla_address);

                result = if reaches_eid.len() > 0 {
                    let reaches: Vec<String> =
                        reaches_eid.iter().map(|it| format!("({})", it)).collect();

                    result + ":" + &format!("[{0}]", reaches.join(","))
                } else {
                    result + ":"
                };

                result = if contacts.len() > 0 {
                    let contacts: Vec<String> = contacts
                        .iter()
                        .map(|it| {
                            let reaches: Vec<String> = it
                                .reaches_eid
                                .iter()
                                .map(|it| format!("({})", it))
                                .collect();

                            format!(
                                "{{{},{},{},[{}]}}",
                                it.start
                                    .duration_since(SystemTime::UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs(),
                                it.end
                                    .duration_since(SystemTime::UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs(),
                                match it.data_rate {
                                    ContactDataRate::Limited(i) => format!("{}", i),
                                    ContactDataRate::Unlimited => format!("{}", 4_294_967_200_i64),
                                },
                                reaches.join(",")
                            )
                        })
                        .collect();

                    result + &format!(":[{}]", contacts.join(","))
                } else {
                    result
                };

                // EOL
                result + ";"
            }
            ConfigBundle::ReplaceContact {
                eid,
                reliability,
                cla_address,
                reaches_eid,
                contacts,
            } => {
                // Command
                let mut result = format!("2({0})", eid);

                // Reliability
                result = match reliability {
                    Some(r) => result + &format!(",{}", r),
                    None => result,
                };

                // CLA
                result = match &cla_address {
                    Some(cla) => result + &format!(":({})", cla),
                    None => result + ":",
                };

                result = if reaches_eid.len() > 0 {
                    let reaches: Vec<String> =
                        reaches_eid.iter().map(|it| format!("({})", it)).collect();

                    result + ":" + &format!("[{0}]", reaches.join(","))
                } else {
                    result + ":"
                };

                result = if contacts.len() > 0 {
                    let contacts: Vec<String> = contacts
                        .iter()
                        .map(|it| {
                            let reaches: Vec<String> = it
                                .reaches_eid
                                .iter()
                                .map(|it| format!("({})", it))
                                .collect();

                            format!(
                                "{{{},{},{},[{}]}}",
                                it.start
                                    .duration_since(SystemTime::UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs(),
                                it.end
                                    .duration_since(SystemTime::UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs(),
                                match it.data_rate {
                                    ContactDataRate::Limited(i) => format!("{}", i),
                                    ContactDataRate::Unlimited => format!("{}", 4_294_967_200_i64),
                                },
                                reaches.join(",")
                            )
                        })
                        .collect();

                    result + &format!(":[{}]", contacts.join(","))
                } else {
                    result
                };

                // EOL
                result + ";"
            }
            ConfigBundle::DeleteContact(eid) => {
                format!("3({0});", eid)
            }
        };

        result
    }

    /// Serialize this config bundle as bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        Vec::from(self.to_string())
    }
}

/// Describes when a contact is available
#[derive(Debug, Clone)]
pub struct Contact {
    /// When this contact will start
    pub start: SystemTime,

    /// When this contact will end
    pub end: SystemTime,

    /// Expected transmission rate
    pub data_rate: ContactDataRate,

    /// Reachable EID through this contact
    pub reaches_eid: Vec<String>,
}

/// Contact expected transmission rate
#[derive(Debug, Clone)]
pub enum ContactDataRate {
    /// Unlimited transmission rate
    Unlimited,

    /// Limited transmission rate in bytes per second
    Limited(i32),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn ts(timestamp: u64) -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(timestamp)
    }

    #[test]
    fn serialize_add() {
        let config_1 = ConfigBundle::AddContact{
            eid: "dtn://ud3tn2.dtn/".into(),
            reliability: None,
            cla_address: "mtcp:127.0.0.1:4223".into(),
            reaches_eid: Vec::new(),
            contacts: vec![
                Contact {
                    start: ts(1401519306972),
                    end: ts(1401519316972),
                    data_rate: ContactDataRate::Limited(1200),
                    reaches_eid: vec!["dtn://89326/".into(), "dtn://12349/".into()],
                },
                Contact {
                    start: ts(1401519506972),
                    end: ts(1401519516972),
                    data_rate: ContactDataRate::Limited(1200),
                    reaches_eid: vec!["dtn://89326/".into(), "dtn://12349/".into()],
                },
            ],
        };

        assert_eq!(config_1.to_string(), "1(dtn://ud3tn2.dtn/):(mtcp:127.0.0.1:4223)::[{1401519306972,1401519316972,1200,[(dtn://89326/),(dtn://12349/)]},{1401519506972,1401519516972,1200,[(dtn://89326/),(dtn://12349/)]}];");

        let config_2 = ConfigBundle::AddContact{
            eid: "dtn://13714/".into(),
            reliability: Some(333),
            cla_address: "tcpspp:".into(),
            reaches_eid: vec!["dtn://18471/".into(), "dtn://81491/".into()],
            contacts: Vec::new(),
        };

        assert_eq!(
            config_2.to_string(),
            "1(dtn://13714/),333:(tcpspp:):[(dtn://18471/),(dtn://81491/)];"
        );
    }

    #[test]
    fn serialize_replace() {
        let config_1 = ConfigBundle::ReplaceContact{
            eid: "dtn://ud3tn2.dtn/".into(),
            reliability: None,
            cla_address: Some("mtcp:127.0.0.1:4223".into()),
            reaches_eid: vec!["dtn://89326/".into(), "dtn://12349/".into()],
            contacts: Vec::new(),
        };

        assert_eq!(
            config_1.to_string(),
            "2(dtn://ud3tn2.dtn/):(mtcp:127.0.0.1:4223):[(dtn://89326/),(dtn://12349/)];"
        );
    }

    #[test]
    fn serialize_delete() {
        let config_1 = ConfigBundle::DeleteContact("dtn://ud3tn2.dtn/".into());
        assert_eq!(config_1.to_string(), "3(dtn://ud3tn2.dtn/);");
    }
}
