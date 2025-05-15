use std::path::Path;
use ud3tn_aap::{Agent, BaseAgent};

fn main(){
    let mut agent = Agent::connect_unix(Path::new("/run/archipel-core/archipel-core.socket"))
            .expect("Failed to connect to DTN node")
        .register("donteatcat".to_owned())
            .expect("Failed to register agent");

    const DESTINATION: &str = "dtn://bob.dtn/donteatcat";

    let bundle_id = agent.send_bundle(DESTINATION.to_owned(), b"Hello world")
        .expect("Failed to send bundle");

    println!("Sent bundle from {}{} to {}", agent.node_id(), agent.agent_id(), DESTINATION);
    println!("Bundle {:?}", bundle_id);
    if let Some(time) = bundle_id.creation_time() {
        println!(" creation time: {}", time.as_datetime().format("%Y-%m-%d %H:%M:%S"))
    }
    if let Some(seq) = bundle_id.sequence_number() {
        println!(" sequence number: {}", seq)
    }
}