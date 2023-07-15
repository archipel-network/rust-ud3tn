use std::{os::unix::net::UnixStream, time::{SystemTime, Duration}};

use ud3tn_aap::{Agent, config::{ConfigBundle, AddContact, Contact, ContactDataRate}};

fn main(){
    let mut connection = Agent::connect(
        UnixStream::connect("/home/epickiwi/Documents/DTN-research/archipel-core/ud3tn.socket").unwrap(),
        "my-agent".into()
    ).unwrap();
    
    connection.send_config(ConfigBundle::AddContact(AddContact {
        eid: "dtn://example.org/".into(),
        reliability: None,
        cla_address: "tcpspp:".into(),
        reaches_eid: Vec::new(),
        contacts: vec![
            Contact { 
                start: SystemTime::now(), 
                end: SystemTime::now() + Duration::from_secs(60), 
                data_rate: ContactDataRate::Unlimited, 
                reaches_eid: Vec::new()
            }
        ],
    })).unwrap()
}