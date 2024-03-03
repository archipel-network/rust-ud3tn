use std::{time::Duration, path::PathBuf};

use ud3tn_aap::{Agent, config::{ConfigBundle, Contact, ContactDataRate}};

fn main(){
    let mut connection = Agent::connect_unix(
        &PathBuf::from("/run/archipel-core/archipel-core.socket"),
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