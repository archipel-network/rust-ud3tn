use std::os::unix::net::UnixStream;

use ud3tn::AAPConnection;

fn main(){
    let connection = AAPConnection::connect(
        UnixStream::connect("/home/epickiwi/Documents/Dev/archipel-core/ud3tn.socket").unwrap(),
        "my-agent".into()
    ).unwrap();
    println!("Connected to {0} as {0}{1}", connection.node_eid, connection.agent_id)
}