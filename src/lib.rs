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

#[cfg(feature = "use_std")]
use std::io::{Read as AsyncRead, Write as AsyncWrite};

#[cfg(feature = "use_tokio")]
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

#[cfg(feature = "use_futures")]
use futures::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

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
    scratch: Vec<u8>,
    socket: S,
    router_version: RouterVersion,
}

impl<S: AsyncWrite + AsyncRead + Unpin> Endpoint<S> {
    #[maybe_async]
    pub async fn attach(socket: S) -> Self {
        // Assume router is of last known version
        let mut endpoint = Self {
            scratch: Vec::new(),
            socket,
            router_version: RouterVersion::v0_5_0__,
        };

        if let Ok(Ok(val)) = endpoint.request::<Value>("getself").await {
            // Routers before v0.4.5 expose ".self.<addr>.build_version"
            if val.get("self").is_some() {
                endpoint.router_version = RouterVersion::__v0_4_4;
                return endpoint;
            }

            // Routers from v0.4.5 expose ".build_version"
            if let Some(v) = val.get("build_version") {
                if let Some(v) = v.as_str() {
                    let v: Vec<i32> = v
                        .split(['.', '-'].as_slice())
                        .take(3)
                        .filter_map(|i| str::parse(i).ok())
                        .collect();
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
            scratch: Vec::new(),
            socket,
            router_version,
        }
    }

    pub fn get_version(&self) -> RouterVersion {
        self.router_version.clone()
    }

    pub fn into_inner(self) -> S {
        self.socket
    }

    pub fn get_ref(&self) -> &S {
        &self.socket
    }

    pub fn get_mut(&mut self) -> &mut S {
        &mut self.socket
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
            .write_all(serde_json::to_vec(&request)?.as_slice())
            .await?;

        let buf = protocol::read_response(&mut self.socket, &mut self.scratch).await?;

        let response: protocol::Response<T> = serde_json::from_slice(buf).map_err(|err| {
            Error::new(
                ErrorKind::InvalidData,
                format!(
                    "While parsing endpoint response for request {:?}: {err}",
                    request.request
                ),
            )
        })?;
        return Ok(match (response.status.as_str(), response.response) {
            ("success", Some(response)) => Ok(response),
            _ => Err(response.error.unwrap_or_else(|| "Unknown".to_string())),
        });
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

    /// Currently there's no well known json deserializer supporting async,
    /// so for the time being response separation is done using heuristics.
    ///
    /// This function assumes:
    ///   - response is strictly formatted (pretty or compact format).
    ///   - `reader` doesn't yield anything besides a single json object.
    #[maybe_async]
    pub async fn read_response<'a, R: AsyncRead + Unpin>(
        reader: &mut R,
        scratch: &'a mut Vec<u8>,
    ) -> io::Result<&'a [u8]> {
        if scratch.is_empty() {
            scratch.extend(std::iter::repeat_n(0, 8192));
        }
        let mut len = 0;
        loop {
            if len == scratch.len() {
                // Double the scratch buffer capacity
                scratch.extend(std::iter::repeat_n(0, len));
            }
            let cap = scratch.len();
            let read = reader.read(&mut scratch[len..cap]).await?;
            if read == 0 {
                // EOF
                break;
            }
            len += read;
            if len <= 2 {
                continue;
            }
            let buf = &scratch[0..len];
            if buf.starts_with(b"{\n") {
                // Pretty
                if buf.ends_with(b"\n}\n") {
                    break;
                }
            } else {
                // Compact
                if buf.ends_with(b"}\n") {
                    break;
                }
            }
        }
        Ok(&scratch[0..len])
    }
}

#[cfg(test)]
#[cfg(feature = "use_std")]
mod tests {
    use super::*;

    use std::io::{Cursor, Read, Write};

    macro_rules! mock_reader {
        ($($read_count:expr => $slice:expr $(,)?)*) => {{
            struct MockReader {
                read_counter: u32,
            }
            impl Write for MockReader {
                fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
                    Ok(buf.len())
                }
                fn flush(&mut self) -> io::Result<()> {
                    Ok(())
                }
            }
            impl Read for MockReader {
                fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
                    self.read_counter += 1;
                    match self.read_counter {
                        $($read_count => {
                            return Cursor::new(buf).write($slice);
                        }),*
                        _ => unreachable!(),
                    }
                }
            }
            impl Drop for MockReader {
                fn drop(&mut self) {
                    let read_expected = 0 $(.max($read_count))*;
                    if self.read_counter != read_expected {
                        panic!("Mocked socket seen {} reads, while expected {}", self.read_counter, read_expected)
                    }
                }
            }
            MockReader { read_counter: 0 }
        }};
    }

    #[test]
    fn simple() {
        let sock = mock_reader!(
            1 => &{
                let json = serde_json::json!({
                    "status": "success",
                    "request": {
                        "request": "test",
                        "arguments": {},
                    },
                    "response": {
                        "mock": 42,
                    }
                });
                let mut vec = serde_json::to_vec_pretty(&json).unwrap();
                vec.push(b'\n');
                vec
            }
        );
        let mut e = Endpoint::attach_version(sock, RouterVersion::v0_5_0__);
        #[derive(Debug, Deserialize, PartialEq, Eq)]
        struct MockResult {
            mock: u32,
        }
        let res: MockResult = Endpoint::request(&mut e, "test").unwrap().unwrap();
        assert_eq!(res, MockResult { mock: 42 });
    }

    #[test]
    fn read_response() {
        use super::protocol::read_response;
        let mut scratch = Vec::new();
        assert_eq!(
            read_response(&mut mock_reader!(1 => b"{ ... }", 2 => b""), &mut scratch).unwrap(),
            b"{ ... }"
        );
        assert_eq!(
            read_response(&mut mock_reader!(1 => b"{ ... }\n"), &mut scratch).unwrap(),
            b"{ ... }\n"
        );
        assert_eq!(
            read_response(&mut mock_reader!(1 => b"{\n ... \n}\n"), &mut scratch).unwrap(),
            b"{\n ... \n}\n"
        );
        assert_eq!(
            read_response(
                &mut mock_reader!(
                    1 => b"{",
                    2 => b" ... ",
                    3 => b"}\n",
                ),
                &mut scratch
            )
            .unwrap(),
            b"{ ... }\n"
        );

        let line: String = std::iter::repeat_n('a', 100).collect();
        let lines: String = std::iter::repeat_n(format!("\"{line}\",\n"), 1000).collect();
        let long = format!("{{\n{lines}\n}}\n");
        assert!(long.len() > 8192 << 2);
        assert_eq!(
            read_response(&mut Cursor::new(long.as_bytes()), &mut scratch).unwrap(),
            long.as_bytes(),
        );
    }
}

#[cfg(test)]
mod tests_live {
    use super::*;
    const SOCKET_PATH: &str = env!("YGGDRASIL_SOCKET");

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
