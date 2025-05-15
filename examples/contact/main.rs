use std::{path::Path, time::Duration};

use ud3tn_aap::{config::{ConfigBundle, Contact, ContactDataRate}, Agent};

fn main(){
    let mut agent = Agent::connect_unix(
        &Path::new("/run/archipel-core/archipel-core.socket")
    ).expect("Failed to connect to node")
    .register("contact-agent".to_owned()).expect("Failed to register agent");
    
    agent.send_config(ConfigBundle::AddContact{
        eid: "dtn://example.org/".into(),
        reliability: None,
        cla_address: "file:/home/epickiwi/Documents/DTN-research/data/".into(),
        reaches_eid: Vec::new(),
        contacts: vec![
            Contact::from_now_during(Duration::from_secs(60), ContactDataRate::Unlimited)
        ],
    }).expect("Failed to sent contact config bundle");

    println!("Contact added")
}