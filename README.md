# `ud3tn_aap`

> Rust AAP development for [ud3tn](https://gitlab.com/d3tn/ud3tn) 

## Getting started

You need a working [ud3tn](https://gitlab.com/d3tn/ud3tn)  node running on your machine.

Using `UnixStream` and socket file of ud3tn

```rust,no_run
use std::os::unix::net::UnixStream;
use ud3tn_aap::Agent;

let mut connection = Agent::connect(
    UnixStream::connect("archipel-core/ud3tn.socket").unwrap(),
    "my-agent".into()
).unwrap();
println!("Connected to {0} as {0}{1}", connection.node_eid, connection.agent_id);

connection.send_bundle("dtn://example.org/hello".into(), "Hello world !".as_bytes()).unwrap();
```

More examples in `examples` folder.