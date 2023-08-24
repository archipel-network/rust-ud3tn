use std::{os::unix::net::UnixStream, time::Duration};

use ud3tn_aap::{Agent, config::{ConfigBundle, Contact, ContactDataRate}};

fn main(){
    let mut connection = Agent::connect(
        UnixStream::connect("/run/archipel-core/archipel-core.socket").unwrap(),
        "contact-agent".into()
    ).unwrap();
    
    connection.send_config(ConfigBundle::AddContact{
        eid: "dtn://example.org/".into(),
        reliability: None,
        cla_address: "file:/home/epickiwi/Documents/DTN-research/data/".into(),
        reaches_eid: Vec::new(),
        contacts: vec![
            Contact::from_now_during(Duration::from_secs(60), ContactDataRate::Unlimited)
        ],
    }).unwrap()
}