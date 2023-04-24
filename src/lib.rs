#![doc = include_str!("../README.md")]

mod interface;
pub use interface::*;

#[cfg(feature = "use_std")]
#[cfg(any(feature = "use_tokio", feature = "use_futures"))]
compile_error!(
    "Can't use `std` and async runtime simultaneously. Consider adding `default-features = false` to depency declaration"
);

#[cfg(all(feature = "use_tokio", feature = "use_futures"))]
compile_error!(
    "Feature \"use_tokio\" and feature \"use_futures\" cannot be enabled at the same time"
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
    std::{collections::HashMap, io, io::Error, io::ErrorKind, net::Ipv6Addr},
};

pub type RequestResult<T> = io::Result<Result<T, String>>;

pub struct Endpoint<S> {
    socket: BufReader<S>,
}

impl<S: AsyncWrite + AsyncRead + Unpin> Endpoint<S> {
    pub fn attach(socket: S) -> Self {
        Self {
            socket: BufReader::new(socket),
        }
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
        self.request_args::<T>(request, HashMap::new()).await
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
                        format!("While parsing endpoint response: {err}"),
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
    const SOCKET_PATH: &str = "/var/run/yggdrasil/yggdrasil.sock";

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
        let mut e = Endpoint::attach(e);
        let err = e.request::<SelfEntry>("getself").await;
        assert!(!err.unwrap().unwrap().build_name.is_empty());
        e.get_peers().await.unwrap().unwrap();
        e.get_sessions().await.unwrap().unwrap();
        e.remove_peer("tcp://[::]:0".to_string(), None).await.ok();
        e.add_peer("tcp://[::]:0".to_string(), None)
            .await
            .unwrap()
            .unwrap();
        e.remove_peer("tcp://[::]:0".to_string(), None)
            .await
            .unwrap()
            .unwrap();
        e.get_self().await.unwrap().unwrap();
        e.get_paths().await.unwrap().unwrap();
        e.get_dht().await.unwrap().unwrap();
        e.get_node_info("".to_string()).await.unwrap().ok();
        e.get_multicast_interfaces().await.unwrap().unwrap();
        e.get_tun().await.unwrap().unwrap();
        e.list().await.unwrap().unwrap();
    }
}
