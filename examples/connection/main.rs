use std::os::unix::net::UnixStream;

use ud3tn_aap::Agent;

fn main(){
    let connection = Agent::connect(
        UnixStream::connect("/run/archipel-core/archipel-core.socket").unwrap(),
        "my-agent".into()
    ).unwrap();
    println!("Connected to {0} as {0}{1}", connection.node_eid, connection.agent_id)
}