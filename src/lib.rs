#![doc = include_str!("../README.md")]

// Hash map macro. Taken from `https://stackoverflow.com/a/71541479`
macro_rules! hash_map{
    ( $($key:tt : $val:expr),* $(,)? ) =>{{
        #[allow(unused_mut)]
        let mut map = ::std::collections::HashMap::with_capacity(hash_map!(@count $($key),* ));
        $(
            #[allow(unused_parens)]
            let _ = map.insert($key, $val);
        )*
        map
    }};
    (@replace $_t:tt $e:expr ) => { $e };
    (@count $($t:tt)*) => { <[()]>::len(&[$( hash_map!(@replace $t ()) ),*]) }
}

mod interface;
pub use interface::*;

#[cfg(feature = "use_std")]
#[cfg(any(feature = "use_tokio", feature = "use_futures"))]
compile_error!(
    "Can't use `std` and async runtime simultaneously. Consider adding `default-features = false` to the `yggdrasilctl` flags"
);

#[cfg(all(feature = "use_tokio", feature = "use_futures"))]
compile_error!(
    "\"use_tokio\" and \"use_futures\" features can't be enabled at the same time. Consider choosing only one"
);

// std
#[cfg(feature = "use_std")]
use std::io::{BufRead as AsyncBufRead, BufReader, Read as AsyncRead, Write as AsyncWrite};
// tokio
#[cfg(feature = "use_tokio")]
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};
// futures
#[cfg(feature = "use_futures")]
use futures::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};

use {
    maybe_async::maybe_async,
    serde::{Deserialize, Serialize},
    serde_json::Value,
    std::{collections::HashMap, io, io::Error, io::ErrorKind, net::Ipv6Addr, time::Duration},
};

pub type RequestResult<T> = io::Result<Result<T, String>>;

#[derive(Clone, PartialEq, Debug)]
#[allow(non_camel_case_types)]
pub enum RouterVersion {
    __v0_4_4,
    v0_4_5__v0_4_7,
    v0_5_0__,
}

#[derive(Debug)]
pub struct Endpoint<S> {
    socket: BufReader<S>,
    router_version: RouterVersion,
}

impl<S: AsyncWrite + AsyncRead + Unpin> Endpoint<S> {
    #[maybe_async]
    pub async fn attach(socket: S) -> Self {
        // Assume router is of last known version
        let mut endpoint = Self {
            socket: BufReader::new(socket),
            router_version: RouterVersion::v0_5_0__,
        };

        if let Ok(Ok(val)) = endpoint.request::<Value>("getself").await {
            // Routers before v0.4.5 (response contains ".self.<addr>.build_version")
            if val.get("self").is_some() {
                endpoint.router_version = RouterVersion::__v0_4_4;
                return endpoint;
            }

            // Routers from v0.4.5 to v0.4.* (".build_version")
            if let Some(v) = val.get("build_version") {
                if let Some(v) = v.as_str() {
                    let v: Vec<i32> = v.split('.').filter_map(|i| str::parse(i).ok()).collect();
                    if v.len() == 3 && v[0] == 0 && v[1] == 4 {
                        endpoint.router_version = RouterVersion::v0_4_5__v0_4_7;
                        return endpoint;
                    }
                }
            }
        }

        endpoint
    }

    pub fn attach_version(socket: S, router_version: RouterVersion) -> Self {
        Self {
            socket: BufReader::new(socket),
            router_version,
        }
    }

    pub fn get_version(&self) -> RouterVersion {
        self.router_version.clone()
    }

    pub fn into_inner(self) -> S {
        self.socket.into_inner()
    }

    pub fn get_ref(&self) -> &S {
        self.socket.get_ref()
    }

    pub fn get_mut(&mut self) -> &mut S {
        self.socket.get_mut()
    }

    #[maybe_async]
    pub async fn request<T: for<'a> Deserialize<'a>>(&mut self, request: &str) -> RequestResult<T> {
        self.request_args::<T>(request, hash_map!()).await
    }

    #[maybe_async]
    pub async fn request_args<T: for<'a> Deserialize<'a>>(
        &mut self,
        request: &str,
        arguments: HashMap<String, serde_json::Value>,
    ) -> RequestResult<T> {
        let request = protocol::Request {
            request,
            arguments,
            keepalive: true,
        };
        self.socket
            .get_mut()
            .write_all(serde_json::to_vec(&request)?.as_slice())
            .await?;

        let mut last_len = 0usize;
        loop {
            let frame = self.socket.fill_buf().await?;
            // Handle EOF
            if last_len == frame.len() {
                return Err(Error::new(
                    ErrorKind::UnexpectedEof,
                    "Socket closed while awaiting data",
                ));
            }
            last_len = frame.len();

            // Deserialize stream
            let mut stream =
                serde_json::Deserializer::from_slice(frame).into_iter::<protocol::Response<T>>();
            if let Some(result) = stream.next() {
                let parsed = result.map_err(|err| {
                    Error::new(
                        ErrorKind::InvalidData,
                        format!(
                            "While parsing endpoint response for request '{}': {err}",
                            request.request
                        ),
                    )
                })?;
                let consumed = stream.byte_offset() + 1;
                #[cfg(feature = "use_futures")]
                self.socket.consume_unpin(consumed);
                #[cfg(not(feature = "use_futures"))]
                self.socket.consume(consumed);
                return Ok(match (parsed.status.as_str(), parsed.response) {
                    ("success", Some(response)) => Ok(response),
                    _ => Err(parsed.error.unwrap_or("Unknown".to_string())),
                });
            }
        }
    }
}

mod protocol {
    use super::*;
    #[derive(Serialize)]
    pub struct Request<'a> {
        pub request: &'a str,
        pub keepalive: bool,
        pub arguments: HashMap<String, serde_json::Value>,
    }
    #[derive(Deserialize)]
    pub struct Response<T> {
        pub status: String,
        pub error: Option<String>,
        pub response: Option<T>,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const SOCKET_PATH: &str = "/run/yggdrasil/yggdrasil.sock";

    #[cfg(feature = "use_std")]
    #[test]
    fn test_request() {
        let e = std::os::unix::net::UnixStream::connect(SOCKET_PATH).unwrap();
        request(e);
    }

    #[cfg(feature = "use_tokio")]
    #[tokio::test]
    async fn test_request() {
        let e = tokio::net::UnixStream::connect(SOCKET_PATH).await.unwrap();
        request(e).await;
    }

    #[cfg(feature = "use_futures")]
    #[test]
    fn test_request() {
        let e = futures::io::AllowStdIo::new(
            std::os::unix::net::UnixStream::connect(SOCKET_PATH).unwrap(),
        );
        futures::executor::block_on(request(e));
    }

    #[maybe_async]
    async fn request<S: AsyncWrite + AsyncRead + Unpin>(e: S) {
        let mut e = Endpoint::attach(e).await;

        if let RouterVersion::v0_4_5__v0_4_7 = e.get_version() {
            e.get_dht().await.unwrap().unwrap();
        }

        if let RouterVersion::v0_4_5__v0_4_7 | RouterVersion::v0_5_0__ = e.get_version() {
            #[derive(Debug, Deserialize)]
            struct _SelfEntry {
                build_name: String,
            }
            let err = e.request::<_SelfEntry>("getself").await;
            assert!(!err.unwrap().unwrap().build_name.is_empty());

            e.remove_peer("tcp://[::]:0".to_string(), None).await.ok();
            e.add_peer("tcp://[::]:0".to_string(), None)
                .await
                .unwrap()
                .unwrap();
            e.remove_peer("tcp://[::]:0".to_string(), None)
                .await
                .unwrap()
                .unwrap();
            e.get_tun().await.unwrap().unwrap();
        }

        if let RouterVersion::v0_5_0__ = e.get_version() {
            e.get_tree().await.unwrap().unwrap();
        }

        e.get_peers().await.unwrap().unwrap();
        e.get_sessions().await.unwrap().unwrap();
        e.get_self().await.unwrap().unwrap();
        e.get_paths().await.unwrap().unwrap();
        e.get_node_info("".to_string()).await.unwrap().ok();
        e.get_multicast_interfaces().await.unwrap().unwrap();
        e.list().await.unwrap().unwrap();
    }
}
