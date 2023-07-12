use std::os::unix::net::UnixStream;

use ud3tn::AAPConnection;

fn main(){
    let connection = AAPConnection::connect(
        UnixStream::connect("/home/epickiwi/Documents/DTN-research/archipel-core/ud3tn.socket").unwrap(),
        "my-agent".into()
    ).unwrap();
    println!("{:?}", connection)
}