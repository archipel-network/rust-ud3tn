use std::path::Path;
use ud3tn_aap::{Agent, BaseAgent};

fn main(){
    let mut agent = Agent::connect_unix(Path::new("/run/archipel-core/archipel-core.socket"))
        .expect("Failed to connect to DTN node");

    agent.ping().expect("Failed to ping");

    let agent = agent.register("my-agent".to_owned())
        .expect("Failed to register");

    println!("Connected to {0} as {0}{1}", agent.node_id(), agent.agent_id())
}