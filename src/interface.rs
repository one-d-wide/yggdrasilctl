use super::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct PeerEntry {
    pub address: Ipv6Addr,
    pub key: String,
    pub port: u16,
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
        self.request("getself").await
    }
    #[maybe_async]
    pub async fn get_paths(&mut self) -> RequestResult<Vec<PathEntry>> {
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
        #[derive(Debug, Deserialize)]
        struct List {
            list: Vec<ListEntry>,
        }
        self.request::<List>("list")
            .await
            .map(|e| e.map(|e| e.list))
    }
}
