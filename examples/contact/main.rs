use std::{os::unix::net::UnixStream, time::{SystemTime, Duration}};

use ud3tn_aap::{Agent, config::{ConfigBundle, AddContact, Contact, ContactDataRate}};

fn main(){
    let mut connection = Agent::connect(
        UnixStream::connect("/home/epickiwi/Documents/Dev/archipel-core/ud3tn.socket").unwrap(),
        "conf-agent".into()
    ).unwrap();
    
    let config = ConfigBundle::AddContact(AddContact {
        eid: "dtn://example.org/".into(),
        reliability: None,
        cla_address: "file:/home/epickiwi/Documents/Dev/archipel-core/data".into(),
        reaches_eid: Vec::new(),
        contacts: vec![
            Contact { 
                start: SystemTime::now(), 
                end: SystemTime::now() + Duration::from_secs(60), 
                data_rate: ContactDataRate::Unlimited
            }
        ],
    });

    println!("{}", config.to_string());

    connection.send_config(config).unwrap()
}