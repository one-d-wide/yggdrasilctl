# yggdrasilctl

A library for accessing [Admin API] of [Yggdrasil network router].

It supports both sync and async environment. All you need is to provide
socket that implements either `Read` and `Write` from `std` for synchronous
operation, or `AsyncRead` and `AsyncWrite` from any async runtime.
Currently supported runtimes: `tokio` and `futures`. If your favourite
runtime is not in the list, consider filing an issue or pull request.

[Admin API]: https://yggdrasil-network.github.io/admin.html
[Yggdrasil network router]: https://github.com/yggdrasil-network/yggdrasil-go

# Basic usage

Add either line to your dependencies in `Cargo.toml`

```toml
# Use `std` (synchronous)
yggdrasilctl = "1"
# Use async runtime
# Availible features: "use_tokio" or "use_futures"
yggdrasilctl = { version = "1", default-features = false, features = [ "use_tokio" ] }
```

Next:

```rust
use yggdrasilctl::Endpoint;
use std::os::unix::net::UnixStream;

// Connect socket using your favourite runtime
let socket = UnixStream::connect("/var/run/yggdrasil/yggdrasil.sock")/*.await*/.unwrap();

// Attach endpoint to a socket
let mut endpoint = Endpoint::attach(socket);

// First you can get I/O or protocol parsing error
let maybe_error = endpoint.get_self()/*.await*/.unwrap();

// Then Admin API can return error (string) to your request
match maybe_error {
    Ok(response) => println!("Yggdrasil address: {}", response.address),
    Err(error) => println!("Admin API returned error: {error}"),
}
```

# Advanced usage

You may also want to perform `debug_*` requests, which are deliberately unimplemented
in this library. For this case `yggdrasilctl` allows you to provide response structure
you want to receive.

First, add crates `serde` and `serde_json` to your dependecies

```toml
# Import derive macros for `Deserialize` trait
serde = { version = "1", features = [ "derive" ] }
# Import enum `Value` that represents any possible json value
serde_json = "1"
```

Next:

```rust
use yggdrasilctl::Endpoint;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::net::Ipv6Addr;

// Connect endpoint
use std::os::unix::net::UnixStream;
let socket = UnixStream::connect("/var/run/yggdrasil/yggdrasil.sock")/*.await*/.unwrap();
let mut endpoint = Endpoint::attach(socket);
let get_self = endpoint.get_self()/*.await*/.unwrap().unwrap();

// Declare a struct you want to receive
#[derive(Deserialize)]
struct DebugRemoteGetSelfEntry {
    coords: String,
    key: String,
}
type DebugRemoteGetSelf = HashMap<Ipv6Addr, DebugRemoteGetSelfEntry>;

// Pass arguments to your request
let mut args = HashMap::<String, Value>::new();
args.insert("key".to_string(), Value::from(get_self.key.as_str()));

// Perform request
let maybe_error = endpoint.request_args::<DebugRemoteGetSelf>("debug_remotegetself", args)/*.await*/.unwrap();

// Parse request
match maybe_error {
    Ok(response) =>
        println!(
            "Yggdrasil node coordinates: {:?}",
            response[&get_self.address].coords
        ),
    Err(error) => println!("Admin API returned error: {error}"),
}
```