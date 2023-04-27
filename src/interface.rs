use super::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct PeerEntry {
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

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionEntry {
    pub address: Ipv6Addr,
    pub key: String,
    pub bytes_recvd: u64,
    pub bytes_sent: u64,
    pub uptime: f64,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct SelfEntry {
    pub build_name: String,
    pub build_version: String,
    pub key: String,
    pub address: Ipv6Addr,
    pub coords: Vec<u64>,
    pub subnet: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PathEntry {
    pub address: Ipv6Addr,
    pub key: String,
    pub path: Vec<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DHTEntry {
    pub address: Ipv6Addr,
    pub key: String,
    pub port: u64,
    pub rest: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TunEntry {
    pub enabled: bool,
    pub name: Option<String>,
    pub mtu: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListEntry {
    pub command: String,
    pub description: String,
    pub fields: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Empty {}

impl<S: AsyncWrite + AsyncRead + Unpin> Endpoint<S> {
    #[maybe_async]
    pub async fn get_peers(&mut self) -> RequestResult<Vec<PeerEntry>> {
        if self.old_router {
            #[derive(Debug, Deserialize)]
            struct Entry {
                port: u64,
                key: String,
                coords: Vec<u64>,
                remote: String,
                bytes_recvd: u64,
                bytes_sent: u64,
                uptime: f64,
            }
            #[derive(Debug, Deserialize)]
            struct Peers {
                peers: HashMap<Ipv6Addr, Entry>,
            }
            return match self.request::<Peers>("getpeers").await? {
                Ok(peers) => {
                    let mut vec: Vec<PeerEntry> = Vec::new();
                    for (k, v) in peers.peers {
                        vec.push(PeerEntry {
                            address: k,
                            key: v.key,
                            port: v.port,
                            coords: v.coords,
                            remote: v.remote,
                            uptime: v.uptime,
                            bytes_recvd: v.bytes_recvd,
                            bytes_sent: v.bytes_sent,
                            priority: 0,
                        });
                    }
                    Ok(Ok(vec))
                }
                Err(err) => Ok(Err(err)),
            };
        }
        #[derive(Debug, Deserialize)]
        struct Peers {
            peers: Vec<PeerEntry>,
        }
        self.request::<Peers>("getpeers")
            .await
            .map(|e| e.map(|e| e.peers))
    }
    #[maybe_async]
    pub async fn get_sessions(&mut self) -> RequestResult<Vec<SessionEntry>> {
        if self.old_router {
            #[derive(Debug, Deserialize)]
            struct Entry {
                key: String,
            }
            #[derive(Debug, Deserialize)]
            struct Sessions {
                sessions: HashMap<Ipv6Addr, Entry>,
            }
            return match self.request::<Sessions>("getsessions").await? {
                Ok(sessions) => {
                    let mut vec: Vec<SessionEntry> = Vec::new();
                    for (k, v) in sessions.sessions {
                        vec.push(SessionEntry {
                            address: k,
                            key: v.key,
                            bytes_recvd: 0,
                            bytes_sent: 0,
                            uptime: 0.0,
                        });
                    }
                    Ok(Ok(vec))
                }
                Err(err) => Ok(Err(err)),
            };
        }
        #[derive(Debug, Deserialize)]
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
        let mut args = HashMap::<String, Value>::new();
        args.insert("uri".to_string(), Value::from(uri));
        if let Some(interface) = interface {
            args.insert("interface".to_string(), Value::from(interface));
        }
        self.request_args("addpeer", args).await
    }
    #[maybe_async]
    pub async fn remove_peer(
        &mut self,
        uri: String,
        interface: Option<String>,
    ) -> RequestResult<Empty> {
        let mut args = HashMap::<String, Value>::new();
        args.insert("uri".to_string(), Value::from(uri));
        if let Some(interface) = interface {
            args.insert("interface".to_string(), Value::from(interface));
        }
        self.request_args("removepeer", args).await
    }
    #[maybe_async]
    pub async fn get_self(&mut self) -> RequestResult<SelfEntry> {
        if self.old_router {
            #[derive(Debug, Deserialize)]
            struct Entry {
                build_name: String,
                build_version: String,
                key: String,
                coords: Vec<u64>,
                subnet: String,
            }
            #[derive(Debug, Deserialize)]
            struct _SelfEntry {
                #[serde(alias = "self")]
                entry: HashMap<Ipv6Addr, Entry>,
            }
            return match self.request::<_SelfEntry>("getself").await? {
                Ok(entry) => {
                    for (k, v) in entry.entry {
                        return Ok(Ok(SelfEntry {
                            address: k,
                            key: v.key,
                            build_name: v.build_name,
                            build_version: v.build_version,
                            coords: v.coords,
                            subnet: v.subnet,
                        }));
                    }
                    Ok(Err("Unknown".to_string()))
                }
                Err(err) => Ok(Err(err)),
            };
        }
        self.request("getself").await
    }
    #[maybe_async]
    pub async fn get_paths(&mut self) -> RequestResult<Vec<PathEntry>> {
        if self.old_router {
            #[derive(Debug, Deserialize)]
            struct Entry {
                key: String,
                path: Vec<u64>,
            }
            #[derive(Debug, Deserialize)]
            struct Paths {
                paths: HashMap<Ipv6Addr, Entry>,
            }
            return match self.request::<Paths>("getpaths").await? {
                Ok(paths) => {
                    let mut vec: Vec<PathEntry> = Vec::new();
                    for (k, v) in paths.paths {
                        vec.push(PathEntry {
                            address: k,
                            key: v.key,
                            path: v.path,
                        });
                    }
                    Ok(Ok(vec))
                }
                Err(err) => Ok(Err(err)),
            };
        }
        #[derive(Debug, Deserialize)]
        struct Paths {
            paths: Vec<PathEntry>,
        }
        self.request::<Paths>("getpaths")
            .await
            .map(|e| e.map(|e| e.paths))
    }
    #[maybe_async]
    pub async fn get_dht(&mut self) -> RequestResult<Vec<DHTEntry>> {
        if self.old_router {
            #[derive(Debug, Deserialize)]
            struct Entry {
                key: String,
                pub port: u64,
                pub rest: u64,
            }
            #[derive(Debug, Deserialize)]
            struct DHT {
                dht: HashMap<Ipv6Addr, Entry>,
            }
            return match self.request::<DHT>("getdht").await? {
                Ok(dht) => {
                    let mut vec: Vec<DHTEntry> = Vec::new();
                    for (k, v) in dht.dht {
                        vec.push(DHTEntry {
                            address: k,
                            key: v.key,
                            port: v.port,
                            rest: v.rest,
                        });
                    }
                    Ok(Ok(vec))
                }
                Err(err) => Ok(Err(err)),
            };
        }
        #[derive(Debug, Deserialize)]
        struct DHT {
            dht: Vec<DHTEntry>,
        }
        self.request::<DHT>("getdht")
            .await
            .map(|e| e.map(|e| e.dht))
    }
    #[maybe_async]
    pub async fn get_node_info(&mut self, key: String) -> RequestResult<HashMap<String, Value>> {
        let mut args = HashMap::new();
        args.insert("key".to_string(), Value::from(key));
        self.request_args("getnodeinfo", args).await
    }
    #[maybe_async]
    pub async fn get_multicast_interfaces(&mut self) -> RequestResult<Vec<String>> {
        #[derive(Debug, Deserialize)]
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
    pub async fn list(&mut self) -> RequestResult<Vec<ListEntry>> {
        if self.old_router {
            #[derive(Debug, Deserialize)]
            struct Entry {
                fields: Vec<String>,
            }
            #[derive(Debug, Deserialize)]
            struct List {
                list: HashMap<String, Entry>,
            }
            return match self.request::<List>("list").await? {
                Ok(list) => {
                    let mut vec: Vec<ListEntry> = Vec::new();
                    for (k, v) in list.list {
                        vec.push(ListEntry {
                            command: k,
                            description: String::new(),
                            fields: Some(v.fields),
                        });
                    }
                    Ok(Ok(vec))
                }
                Err(err) => Ok(Err(err)),
            };
        }
        #[derive(Debug, Deserialize)]
        struct List {
            list: Vec<ListEntry>,
        }
        self.request::<List>("list")
            .await
            .map(|e| e.map(|e| e.list))
    }
}
