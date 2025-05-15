use std::{io::{stdin, stdout, Write}, path::{Path, PathBuf}, thread};

use inquire::Text;
use ud3tn_aap::{Agent, BaseAgent};

fn main(){

    // Establish connection to ud3tn

    let mut output_agent = Agent::connect_unix(
        &Path::new("/run/archipel-core/archipel-core.socket")
    ).expect("Can't create output aap")
        .register("chat/out".to_owned()).expect("Failed to register output agent");

    let mut input_agent = Agent::connect_unix(
        &PathBuf::from("/run/archipel-core/archipel-core.socket")
    ).expect("Can't create input aap")
        .register("chat/in".to_owned()).expect("Failed to register input agent");
    
    // Request user info

    let username = Text::new("Username").prompt().unwrap();
    let dest = Text::new("Destination EID")
                    .with_formatter(&|val| format!("dtn://{}/", val))
                    .prompt().unwrap();
    println!();

    println!("Welcome {} !", username);
    println!();
    println!("Your EID {}", output_agent.node_id());
    println!("Sending to EID dtn://{}/", dest);
    println!();

    let destination_eid = format!("dtn://{}/chat/in", dest);

    // Send messages

    let fallback_username = username.clone();
    thread::spawn(move || {
        loop {
            let bundle = input_agent.recv_bundle()
                .expect("Error receiving messages");
            let mess = String::from_utf8(bundle.payload).expect("Invalid utf8 message");
            println!("\r{: <50}", mess);
            print!("<{}> ", fallback_username);
            stdout().flush().unwrap();
        }
    });

    loop {
        let mut mess = String::new();
        print!("<{}> ", username);
        stdout().flush().unwrap();
        stdin().read_line(&mut mess).unwrap();
        mess = mess[0..mess.len()-1].to_string();

        output_agent.send_bundle(
            destination_eid.clone(), format!("<{}> {}", username, mess).as_bytes()
        ).expect("Unable to send message");
    }
}