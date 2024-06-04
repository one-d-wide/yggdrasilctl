use super::*;

fn parse_optional_duration_from_nanos<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Duration>, D::Error> {
    u64::deserialize(deserializer).map(|nanos| Some(Duration::from_nanos(nanos)))
}

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(test, serde(deny_unknown_fields))]
pub struct PeerEntry {
    pub address: Option<Ipv6Addr>,
    pub key: String,
    pub port: u64,
    pub priority: Option<u64>,
    pub remote: Option<String>,
    pub bytes_recvd: Option<u64>,
    pub bytes_sent: Option<u64>,
    pub uptime: Option<f64>,
    pub up: bool,
    pub inbound: bool,
    pub last_error: Option<String>,
    #[serde(default, deserialize_with = "parse_optional_duration_from_nanos")]
    pub last_error_time: Option<Duration>,
}

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(test, serde(deny_unknown_fields))]
pub struct SessionEntry {
    pub address: Ipv6Addr,
    pub key: String,
    pub bytes_recvd: Option<u64>,
    pub bytes_sent: Option<u64>,
    pub uptime: Option<f64>,
}
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(test, serde(deny_unknown_fields))]
pub struct SelfEntry {
    pub build_name: String,
    pub build_version: String,
    pub key: String,
    pub address: Ipv6Addr,
    pub subnet: String,
    pub routing_entries: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(test, serde(deny_unknown_fields))]
pub struct PathEntry {
    pub address: Ipv6Addr,
    pub key: String,
    pub path: Vec<u64>,
    pub sequence: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(test, serde(deny_unknown_fields))]
pub struct DHTEntry {
    pub address: Ipv6Addr,
    pub key: String,
    pub port: u64,
    pub rest: u64,
}

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(test, serde(deny_unknown_fields))]
pub struct TunEntry {
    pub enabled: bool,
    pub name: Option<String>,
    pub mtu: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(test, serde(deny_unknown_fields))]
pub struct TreeEntry {
    pub address: Ipv6Addr,
    pub key: String,
    pub parent: String,
    pub sequence: u64,
}

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(test, serde(deny_unknown_fields))]
pub struct ListEntry {
    pub command: String,
    pub description: String,
    pub fields: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(test, serde(deny_unknown_fields))]
pub struct Empty {}

impl<S: AsyncWrite + AsyncRead + Unpin> Endpoint<S> {
    #[maybe_async]
    pub async fn get_peers(&mut self) -> RequestResult<Vec<PeerEntry>> {
        match self.router_version {
            RouterVersion::__v0_4_4 => {
                #[derive(Debug, Deserialize)]
                #[cfg_attr(test, serde(deny_unknown_fields))]
                struct Entry {
                    port: u64,
                    key: String,
                    #[allow(dead_code)]
                    coords: Vec<u64>,
                    remote: String,
                    bytes_recvd: u64,
                    bytes_sent: u64,
                    uptime: f64,
                }
                #[derive(Debug, Deserialize)]
                #[cfg_attr(test, serde(deny_unknown_fields))]
                struct Peers {
                    peers: HashMap<Ipv6Addr, Entry>,
                }
                match self.request::<Peers>("getpeers").await? {
                    Ok(peers) => {
                        let vec = peers
                            .peers
                            .into_iter()
                            .map(|(k, v)| PeerEntry {
                                address: Some(k),
                                key: v.key,
                                port: v.port,
                                remote: Some(v.remote),
                                uptime: Some(v.uptime),
                                bytes_recvd: Some(v.bytes_recvd),
                                bytes_sent: Some(v.bytes_sent),
                                priority: None,
                                up: true,
                                inbound: false,
                                last_error: None,
                                last_error_time: None,
                            })
                            .collect();
                        Ok(Ok(vec))
                    }
                    Err(err) => Ok(Err(err)),
                }
            }
            RouterVersion::v0_4_5__v0_4_7 => {
                #[derive(Debug, Serialize, Deserialize)]
                #[cfg_attr(test, serde(deny_unknown_fields))]
                pub struct Entry {
                    pub address: Ipv6Addr,
                    pub key: String,
                    pub port: u64,
                    pub priority: u64,
                    pub coords: Vec<u64>,
                    pub remote: String,
                    pub bytes_recvd: u64,
                    pub bytes_sent: u64,
                    pub uptime: f64,
                }
                #[derive(Debug, Deserialize)]
                #[cfg_attr(test, serde(deny_unknown_fields))]
                struct Peers {
                    peers: Vec<Entry>,
                }
                match self.request::<Peers>("getpeers").await? {
                    Ok(peers) => {
                        let vec = peers
                            .peers
                            .into_iter()
                            .map(|v| PeerEntry {
                                address: Some(v.address),
                                key: v.key,
                                port: v.port,
                                remote: Some(v.remote),
                                uptime: Some(v.uptime),
                                bytes_recvd: Some(v.bytes_recvd),
                                bytes_sent: Some(v.bytes_sent),
                                priority: None,
                                up: true,
                                inbound: false,
                                last_error: None,
                                last_error_time: None,
                            })
                            .collect();
                        Ok(Ok(vec))
                    }
                    Err(err) => Ok(Err(err)),
                }
            }
            RouterVersion::v0_5_0__ => {
                #[derive(Debug, Deserialize)]
                #[cfg_attr(test, serde(deny_unknown_fields))]
                struct Peers {
                    peers: Vec<PeerEntry>,
                }
                self.request::<Peers>("getpeers")
                    .await
                    .map(|e| e.map(|e| e.peers))
            }
        }
    }
    #[maybe_async]
    pub async fn get_sessions(&mut self) -> RequestResult<Vec<SessionEntry>> {
        if let RouterVersion::__v0_4_4 = self.router_version {
            #[derive(Debug, Deserialize)]
            #[cfg_attr(test, serde(deny_unknown_fields))]
            struct Entry {
                key: String,
            }
            #[derive(Debug, Deserialize)]
            #[cfg_attr(test, serde(deny_unknown_fields))]
            struct Sessions {
                sessions: HashMap<Ipv6Addr, Entry>,
            }
            return match self.request::<Sessions>("getsessions").await? {
                Ok(sessions) => {
                    let vec = sessions
                        .sessions
                        .into_iter()
                        .map(|(k, v)| SessionEntry {
                            address: k,
                            key: v.key,
                            bytes_recvd: None,
                            bytes_sent: None,
                            uptime: None,
                        })
                        .collect();
                    Ok(Ok(vec))
                }
                Err(err) => Ok(Err(err)),
            };
        }
        #[derive(Debug, Deserialize)]
        #[cfg_attr(test, serde(deny_unknown_fields))]
        struct Sessions {
            sessions: Vec<SessionEntry>,
        }
        self.request::<Sessions>("getsessions")
            .await
            .map(|e| e.map(|e| e.sessions))
    }
    #[maybe_async]
    pub async fn add_peer(
        &mut self,
        uri: String,
        interface: Option<String>,
    ) -> RequestResult<Empty> {
        let mut args = hash_map! {
            ("uri".into()): uri.into()
        };
        if let Some(interface) = interface {
            args.insert("interface".into(), interface.into());
        }
        self.request_args("addpeer", args).await
    }
    #[maybe_async]
    pub async fn remove_peer(
        &mut self,
        uri: String,
        interface: Option<String>,
    ) -> RequestResult<Empty> {
        let mut args = hash_map! {
            ("uri".into()): uri.into()
        };
        if let Some(interface) = interface {
            args.insert("interface".into(), interface.into());
        }
        self.request_args("removepeer", args).await
    }
    #[maybe_async]
    pub async fn get_self(&mut self) -> RequestResult<SelfEntry> {
        match self.router_version {
            RouterVersion::__v0_4_4 => {
                #[derive(Debug, Deserialize)]
                #[cfg_attr(test, serde(deny_unknown_fields))]
                struct Entry {
                    build_name: String,
                    build_version: String,
                    key: String,
                    #[allow(dead_code)]
                    coords: Vec<u64>,
                    subnet: String,
                }
                #[derive(Debug, Deserialize)]
                #[cfg_attr(test, serde(deny_unknown_fields))]
                struct _SelfEntry {
                    #[serde(alias = "self")]
                    entry: HashMap<Ipv6Addr, Entry>,
                }
                match self.request::<_SelfEntry>("getself").await? {
                    Ok(entry) => match entry.entry.into_iter().next() {
                        Some((k, v)) => Ok(Ok(SelfEntry {
                            address: k,
                            key: v.key,
                            build_name: v.build_name,
                            build_version: v.build_version,
                            subnet: v.subnet,
                            routing_entries: None,
                        })),
                        None => Ok(Err("Unknown".to_string())),
                    },
                    Err(err) => Ok(Err(err)),
                }
            }
            RouterVersion::v0_4_5__v0_4_7 => {
                #[derive(Debug, Serialize, Deserialize)]
                #[cfg_attr(test, serde(deny_unknown_fields))]
                pub struct Entry {
                    pub build_name: String,
                    pub build_version: String,
                    pub key: String,
                    pub address: Ipv6Addr,
                    #[allow(dead_code)]
                    pub coords: Vec<u64>,
                    pub subnet: String,
                }
                match self.request::<Entry>("getself").await? {
                    Ok(v) => Ok(Ok(SelfEntry {
                        address: v.address,
                        key: v.key,
                        build_name: v.build_name,
                        build_version: v.build_version,
                        subnet: v.subnet,
                        routing_entries: None,
                    })),
                    Err(v) => Ok(Err(v)),
                }
            }
            RouterVersion::v0_5_0__ => self.request("getself").await,
        }
    }
    #[maybe_async]
    pub async fn get_paths(&mut self) -> RequestResult<Vec<PathEntry>> {
        match self.router_version {
            RouterVersion::__v0_4_4 => {
                #[derive(Debug, Deserialize)]
                #[cfg_attr(test, serde(deny_unknown_fields))]
                struct Entry {
                    key: String,
                    path: Vec<u64>,
                }
                #[derive(Debug, Deserialize)]
                #[cfg_attr(test, serde(deny_unknown_fields))]
                struct Paths {
                    paths: HashMap<Ipv6Addr, Entry>,
                }
                match self.request::<Paths>("getpaths").await? {
                    Ok(paths) => {
                        let vec = paths
                            .paths
                            .into_iter()
                            .map(|(k, v)| PathEntry {
                                address: k,
                                key: v.key,
                                path: v.path,
                                sequence: None,
                            })
                            .collect();
                        Ok(Ok(vec))
                    }
                    Err(err) => Ok(Err(err)),
                }
            }
            RouterVersion::v0_4_5__v0_4_7 | RouterVersion::v0_5_0__ => {
                #[derive(Debug, Deserialize)]
                #[cfg_attr(test, serde(deny_unknown_fields))]
                struct Paths {
                    paths: Vec<PathEntry>,
                }
                self.request::<Paths>("getpaths")
                    .await
                    .map(|e| e.map(|e| e.paths))
            }
        }
    }
    #[maybe_async]
    pub async fn get_dht(&mut self) -> RequestResult<Vec<DHTEntry>> {
        match self.router_version {
            RouterVersion::__v0_4_4 => {
                #[derive(Debug, Deserialize)]
                #[cfg_attr(test, serde(deny_unknown_fields))]
                struct Entry {
                    key: String,
                    pub port: u64,
                    pub rest: u64,
                }
                #[derive(Debug, Deserialize)]
                #[cfg_attr(test, serde(deny_unknown_fields))]
                struct Dht {
                    dht: HashMap<Ipv6Addr, Entry>,
                }
                match self.request::<Dht>("getdht").await? {
                    Ok(dht) => {
                        let vec = dht
                            .dht
                            .into_iter()
                            .map(|(k, v)| DHTEntry {
                                address: k,
                                key: v.key,
                                port: v.port,
                                rest: v.rest,
                            })
                            .collect();
                        Ok(Ok(vec))
                    }
                    Err(err) => Ok(Err(err)),
                }
            }
            // Not implemented in the router after v0.5.0
            RouterVersion::v0_4_5__v0_4_7 | RouterVersion::v0_5_0__ => {
                #[derive(Debug, Deserialize)]
                #[cfg_attr(test, serde(deny_unknown_fields))]
                struct Dht {
                    dht: Vec<DHTEntry>,
                }
                self.request::<Dht>("getdht")
                    .await
                    .map(|e| e.map(|e| e.dht))
            }
        }
    }
    #[maybe_async]
    pub async fn get_node_info(&mut self, key: String) -> RequestResult<HashMap<String, Value>> {
        let args = hash_map! {
            ("key".into()): key.into()
        };
        self.request_args("getnodeinfo", args).await
    }
    #[maybe_async]
    pub async fn get_multicast_interfaces(&mut self) -> RequestResult<Vec<String>> {
        #[derive(Debug, Deserialize)]
        #[cfg_attr(test, serde(deny_unknown_fields))]
        struct MulticastInterfaces {
            multicast_interfaces: Vec<String>,
        }
        self.request::<MulticastInterfaces>("getmulticastinterfaces")
            .await
            .map(|e| e.map(|e| e.multicast_interfaces))
    }
    #[maybe_async]
    pub async fn get_tun(&mut self) -> RequestResult<TunEntry> {
        self.request("gettun").await
    }
    #[maybe_async]
    pub async fn get_tree(&mut self) -> RequestResult<Vec<TreeEntry>> {
        #[derive(Debug, Deserialize)]
        #[cfg_attr(test, serde(deny_unknown_fields))]
        struct Tree {
            tree: Vec<TreeEntry>,
        }
        self.request::<Tree>("gettree")
            .await
            .map(|t| t.map(|r| r.tree))
    }
    #[maybe_async]
    pub async fn list(&mut self) -> RequestResult<Vec<ListEntry>> {
        if let RouterVersion::__v0_4_4 = self.router_version {
            #[derive(Debug, Deserialize)]
            #[cfg_attr(test, serde(deny_unknown_fields))]
            struct Entry {
                fields: Vec<String>,
            }
            #[derive(Debug, Deserialize)]
            #[cfg_attr(test, serde(deny_unknown_fields))]
            struct List {
                list: HashMap<String, Entry>,
            }
            return match self.request::<List>("list").await? {
                Ok(list) => {
                    let vec = list
                        .list
                        .into_iter()
                        .map(|(k, v)| ListEntry {
                            command: k,
                            description: String::new(),
                            fields: Some(v.fields),
                        })
                        .collect();
                    Ok(Ok(vec))
                }
                Err(err) => Ok(Err(err)),
            };
        }
        #[derive(Debug, Deserialize)]
        #[cfg_attr(test, serde(deny_unknown_fields))]
        struct List {
            list: Vec<ListEntry>,
        }
        self.request::<List>("list")
            .await
            .map(|e| e.map(|e| e.list))
    }
}
