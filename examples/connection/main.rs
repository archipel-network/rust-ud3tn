use std::path::PathBuf;

use ud3tn_aap::Agent;

fn main(){
    let connection = Agent::connect_unix(
        &PathBuf::from("/home/epickiwi/Documents/Dev/archipel-core/ud3tn.socket"),
        "my-agent".into()
    ).unwrap();
    println!("Connected to {0} as {0}{1}", connection.node_eid, connection.agent_id)
}