use std::{thread, io::{stdin, stdout, Write}, path::PathBuf};

use inquire::Text;
use ud3tn_aap::Agent;

fn main(){

    // Establish connection to ud3tn

    let mut output_connection = Agent::connect_unix(
        &PathBuf::from("/run/user/1000/archipel-core/archipel-core.socket"),
        "chat/out".into()
    ).expect("Can't create output aap");

    let mut input_connection = Agent::connect_unix(
        &PathBuf::from("/run/user/1000/archipel-core/archipel-core.socket"),
        "chat/in".into()
    ).expect("Can't create input aap");
    
    // Request user info

    let username = Text::new("Username").prompt().unwrap();
    let dest = Text::new("Destination EID")
                    .with_formatter(&|val| format!("dtn://{}/", val))
                    .prompt().unwrap();
    println!();

    println!("Welcome {} !", username);
    println!();
    println!("Your EID {}", output_connection.node_eid);
    println!("Sending to EID dtn://{}/", dest);
    println!();

    let destination_eid = format!("dtn://{}/chat/in", dest);

    // Send messages

    let fallback_username = username.clone();
    thread::spawn(move || {
        loop {
            let (_, payload) = input_connection.recv_bundle()
                .expect("Error receiving messages");
            let mess = String::from_utf8(payload).expect("Invalid utf8 message");
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

        output_connection.send_bundle(
            destination_eid.clone(), format!("<{}> {}", username, mess).as_bytes()
        ).expect("Unable to send message");
    }
}