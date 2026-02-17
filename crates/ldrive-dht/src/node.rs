use anyhow::{Context, Result};
use ldrive_common::NodeId;
use ldrive_net::{QuicClient, QuicServer, send_message, recv_message};
use ldrive_proto::{self, PeerMessage, peer_message::Msg};
use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{info, warn, debug};

use crate::key::Key;
use crate::routing::{RoutingTable, RoutingEntry, K};

/// Number of parallel lookups in iterative find
const ALPHA: usize = 3;
/// Timeout for a single RPC
const RPC_TIMEOUT: Duration = Duration::from_secs(5);

/// Content store: maps content keys to sets of peers that hold them
type ContentStore = HashMap<Key, HashSet<PeerEntry>>;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct PeerEntry {
    key: Key,
    addr: SocketAddr,
    region: String,
}

/// A running Kademlia DHT node
pub struct DhtNode {
    pub server: QuicServer,
    routing: Arc<RwLock<RoutingTable>>,
    content: Arc<RwLock<ContentStore>>,
    client: QuicClient,
}

impl DhtNode {
    /// Create a new DHT node listening on the given address with auto-detected region.
    pub async fn new(listen_addr: SocketAddr) -> Result<Self> {
        let server = QuicServer::bind(listen_addr).await?;
        let local_key = Key::from_node_id(&server.node_id());
        let routing = Arc::new(RwLock::new(RoutingTable::new(local_key)));
        let content = Arc::new(RwLock::new(HashMap::new()));
        let client = QuicClient::new().await?;

        Ok(Self {
            server,
            routing,
            content,
            client,
        })
    }

    pub fn node_id(&self) -> NodeId {
        self.server.node_id()
    }

    pub fn local_key(&self) -> Key {
        Key::from_node_id(&self.server.node_id())
    }

    pub fn local_addr(&self) -> Result<SocketAddr> {
        self.server.local_addr()
    }

    /// Bootstrap by connecting to known peers and performing a self-lookup.
    pub async fn bootstrap(&self, bootstrap_addrs: &[SocketAddr]) -> Result<()> {
        let local_key = self.local_key();
        info!(
            node_id = %self.node_id(),
            bootstrap_count = bootstrap_addrs.len(),
            "bootstrapping DHT"
        );

        // Contact each bootstrap peer
        for addr in bootstrap_addrs {
            match self.rpc_find_node(*addr, &local_key).await {
                Ok(peers) => {
                    info!(addr = %addr, found = peers.len(), "bootstrap peer responded");
                    let mut rt = self.routing.write().await;
                    for (key, peer_addr) in &peers {
                        rt.upsert(RoutingEntry::new(*key, *peer_addr, "unknown".to_string()));
                    }
                }
                Err(e) => {
                    warn!(addr = %addr, err = %e, "bootstrap peer unreachable");
                }
            }
        }

        // Iterative self-lookup to populate routing table
        let _ = self.iterative_find_node(&local_key).await;

        let rt = self.routing.read().await;
        info!(peers = rt.len(), "bootstrap complete");
        Ok(())
    }

    /// Iterative FIND_NODE: find the K closest nodes to a target.
    pub async fn iterative_find_node(&self, target: &Key) -> Vec<(Key, SocketAddr)> {
        let rt = self.routing.read().await;
        let initial = rt.closest(target, ALPHA);
        drop(rt);

        let mut contacted: HashSet<Key> = HashSet::new();
        let mut results: Vec<(Key, SocketAddr)> = initial
            .iter()
            .map(|e| (e.key, e.addr))
            .collect();
        let mut pending: Vec<(Key, SocketAddr)> = results.clone();

        loop {
            // Pick up to ALPHA uncontacted peers closest to target
            pending.sort_by_key(|(k, _)| target.distance(k));
            let to_query: Vec<(Key, SocketAddr)> = pending
                .iter()
                .filter(|(k, _)| !contacted.contains(k))
                .take(ALPHA)
                .cloned()
                .collect();

            if to_query.is_empty() {
                break;
            }

            // Query in parallel
            let mut handles = Vec::new();
            for (key, addr) in to_query {
                contacted.insert(key);
                let target = *target;
                let client_endpoint = self.client.endpoint.clone();
                let local_key = self.local_key();

                handles.push(tokio::spawn(async move {
                    let result = rpc_find_node_raw(&client_endpoint, addr, &local_key, &target).await;
                    (key, addr, result)
                }));
            }

            let mut improved = false;
            for handle in handles {
                if let Ok((queried_key, _queried_addr, Ok(new_peers))) = handle.await {
                    // Add responding peer to routing table
                    let mut rt = self.routing.write().await;
                    rt.upsert(RoutingEntry::new(queried_key, _queried_addr, "unknown".to_string()));
                    drop(rt);

                    for (peer_key, peer_addr) in new_peers {
                        if !results.iter().any(|(k, _)| k == &peer_key) {
                            results.push((peer_key, peer_addr));
                            pending.push((peer_key, peer_addr));
                            improved = true;
                        }
                    }
                }
            }

            if !improved {
                break;
            }
        }

        // Return K closest
        results.sort_by_key(|(k, _)| target.distance(k));
        results.truncate(K);
        results
    }

    /// Iterative FIND_VALUE: find peers that hold a content key.
    pub async fn iterative_find_value(&self, content_key: &Key) -> Vec<(Key, SocketAddr)> {
        // First check local content store
        {
            let cs = self.content.read().await;
            if let Some(holders) = cs.get(content_key) {
                if !holders.is_empty() {
                    return holders
                        .iter()
                        .map(|p| (p.key, p.addr))
                        .collect();
                }
            }
        }

        let rt = self.routing.read().await;
        let initial = rt.closest(content_key, ALPHA);
        drop(rt);

        let mut contacted: HashSet<Key> = HashSet::new();
        let mut pending: Vec<(Key, SocketAddr)> = initial
            .iter()
            .map(|e| (e.key, e.addr))
            .collect();

        loop {
            pending.sort_by_key(|(k, _)| content_key.distance(k));
            let to_query: Vec<(Key, SocketAddr)> = pending
                .iter()
                .filter(|(k, _)| !contacted.contains(k))
                .take(ALPHA)
                .cloned()
                .collect();

            if to_query.is_empty() {
                break;
            }

            let mut handles = Vec::new();
            for (key, addr) in to_query {
                contacted.insert(key);
                let ck = *content_key;
                let client_endpoint = self.client.endpoint.clone();
                let local_key = self.local_key();

                handles.push(tokio::spawn(async move {
                    let result = rpc_find_value_raw(&client_endpoint, addr, &local_key, &ck).await;
                    (key, addr, result)
                }));
            }

            for handle in handles {
                match handle.await {
                    Ok((_queried_key, _queried_addr, Ok(FindValueResult::Found(holders)))) => {
                        let mut rt = self.routing.write().await;
                        rt.upsert(RoutingEntry::new(_queried_key, _queried_addr, "unknown".to_string()));
                        drop(rt);
                        return holders;
                    }
                    Ok((_queried_key, _queried_addr, Ok(FindValueResult::CloserPeers(peers)))) => {
                        let mut rt = self.routing.write().await;
                        rt.upsert(RoutingEntry::new(_queried_key, _queried_addr, "unknown".to_string()));
                        drop(rt);

                        for (peer_key, peer_addr) in peers {
                            if !pending.iter().any(|(k, _)| k == &peer_key) {
                                pending.push((peer_key, peer_addr));
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        Vec::new()
    }

    /// Announce that we hold a content key to the K closest nodes.
    pub async fn announce(&self, content_key: &Key) -> Result<usize> {
        let closest = self.iterative_find_node(content_key).await;
        let local_key = self.local_key();
        let local_addr = self.local_addr()?;

        let mut success_count = 0;
        for (_, peer_addr) in &closest {
            match self
                .rpc_store_announce(*peer_addr, content_key, &local_key, local_addr)
                .await
            {
                Ok(true) => success_count += 1,
                Ok(false) => {}
                Err(e) => {
                    debug!(addr = %peer_addr, err = %e, "store announce failed");
                }
            }
        }

        info!(key = %content_key, announced_to = success_count, "content announced");
        Ok(success_count)
    }

    /// Handle incoming DHT messages on a connection. Run this in a loop for the server.
    pub async fn handle_message(
        &self,
        msg: &PeerMessage,
        sender_addr: SocketAddr,
    ) -> Option<PeerMessage> {
        match &msg.msg {
            Some(Msg::FindNode(find)) => {
                if let Some(sender_key) = parse_key_20(&find.sender_id) {
                    let mut rt = self.routing.write().await;
                    rt.upsert(RoutingEntry::new(sender_key, sender_addr, "unknown".to_string()));
                    drop(rt);
                }

                let target = match parse_key_20(&find.target) {
                    Some(k) => k,
                    None => return None,
                };

                let rt = self.routing.read().await;
                let closest = rt.closest(&target, K);

                Some(PeerMessage {
                    msg: Some(Msg::FindNodeResponse(ldrive_proto::FindNodeResponse {
                        peers: closest
                            .iter()
                            .map(|e| ldrive_proto::PeerInfo {
                                node_id: e.key.as_bytes().to_vec(),
                                addr: e.addr.to_string(),
                                region: e.region.clone(),
                            })
                            .collect(),
                    })),
                })
            }

            Some(Msg::FindValue(find)) => {
                if let Some(sender_key) = parse_key_20(&find.sender_id) {
                    let mut rt = self.routing.write().await;
                    rt.upsert(RoutingEntry::new(sender_key, sender_addr, "unknown".to_string()));
                    drop(rt);
                }

                let content_key = match parse_key_20(&find.key) {
                    Some(k) => k,
                    None => return None,
                };

                let cs = self.content.read().await;
                if let Some(holders) = cs.get(&content_key) {
                    if !holders.is_empty() {
                        return Some(PeerMessage {
                            msg: Some(Msg::FindValueResponse(
                                ldrive_proto::FindValueResponse {
                                    found: true,
                                    holders: holders
                                        .iter()
                                        .map(|p| ldrive_proto::PeerInfo {
                                            node_id: p.key.as_bytes().to_vec(),
                                            addr: p.addr.to_string(),
                                            region: p.region.clone(),
                                        })
                                        .collect(),
                                    closer_peers: vec![],
                                },
                            )),
                        });
                    }
                }
                drop(cs);

                // Not found - return closest peers
                let rt = self.routing.read().await;
                let closest = rt.closest(&content_key, K);

                Some(PeerMessage {
                    msg: Some(Msg::FindValueResponse(ldrive_proto::FindValueResponse {
                        found: false,
                        holders: vec![],
                        closer_peers: closest
                            .iter()
                            .map(|e| ldrive_proto::PeerInfo {
                                node_id: e.key.as_bytes().to_vec(),
                                addr: e.addr.to_string(),
                                region: e.region.clone(),
                            })
                            .collect(),
                    })),
                })
            }

            Some(Msg::StoreAnnounce(store)) => {
                if let Some(sender_key) = parse_key_20(&store.sender_id) {
                    let mut rt = self.routing.write().await;
                    rt.upsert(RoutingEntry::new(sender_key, sender_addr, "unknown".to_string()));
                    drop(rt);
                }

                let content_key = match parse_key_20(&store.key) {
                    Some(k) => k,
                    None => {
                        return Some(PeerMessage {
                            msg: Some(Msg::StoreAnnounceAck(ldrive_proto::StoreAnnounceAck {
                                success: false,
                            })),
                        });
                    }
                };

                // Extract the announced peer info
                let peer = if let Some(info) = &store.sender_info {
                    if let (Some(key), Ok(addr)) =
                        (parse_key_20(&info.node_id), info.addr.parse::<SocketAddr>())
                    {
                        PeerEntry { key, addr, region: "unknown".to_string() }
                    } else {
                        return Some(PeerMessage {
                            msg: Some(Msg::StoreAnnounceAck(ldrive_proto::StoreAnnounceAck {
                                success: false,
                            })),
                        });
                    }
                } else {
                    // Use sender's info
                    let sender_key = match parse_key_20(&store.sender_id) {
                        Some(k) => k,
                        None => return None,
                    };
                    PeerEntry {
                        key: sender_key,
                        addr: sender_addr,
                        region: "unknown".to_string(),
                    }
                };

                let mut cs = self.content.write().await;
                cs.entry(content_key).or_default().insert(peer);

                debug!(key = %content_key, "content announcement stored");

                Some(PeerMessage {
                    msg: Some(Msg::StoreAnnounceAck(ldrive_proto::StoreAnnounceAck {
                        success: true,
                    })),
                })
            }

            _ => None, // Not a DHT message
        }
    }

    /// Get routing table peer count
    pub async fn peer_count(&self) -> usize {
        self.routing.read().await.len()
    }

    /// Get all known peers
    pub async fn known_peers(&self) -> Vec<(Key, SocketAddr)> {
        self.routing
            .read()
            .await
            .all_peers()
            .into_iter()
            .map(|e| (e.key, e.addr))
            .collect()
    }

    // ─── RPC helpers ───

    async fn rpc_find_node(
        &self,
        addr: SocketAddr,
        target: &Key,
    ) -> Result<Vec<(Key, SocketAddr)>> {
        rpc_find_node_raw(&self.client.endpoint, addr, &self.local_key(), target).await
    }

    async fn rpc_store_announce(
        &self,
        addr: SocketAddr,
        content_key: &Key,
        local_key: &Key,
        local_addr: SocketAddr,
    ) -> Result<bool> {
        let connecting = self.client.endpoint
            .connect(addr, "ldrive.local")
            .context("connect")?;
        let conn = tokio::time::timeout(RPC_TIMEOUT, connecting)
            .await
            .context("connect timeout")?
            .context("handshake")?;

        let (mut send, mut recv) = conn.open_bi().await?;

        let msg = PeerMessage {
            msg: Some(Msg::StoreAnnounce(ldrive_proto::StoreAnnounce {
                sender_id: local_key.as_bytes().to_vec(),
                key: content_key.as_bytes().to_vec(),
                sender_info: Some(ldrive_proto::PeerInfo {
                    node_id: local_key.as_bytes().to_vec(),
                    addr: local_addr.to_string(),
                    region: self.server.identity.region.clone(),
                }),
            })),
        };
        send_message(&mut send, &msg).await?;
        send.finish()?;

        let resp = tokio::time::timeout(RPC_TIMEOUT, recv_message(&mut recv))
            .await
            .context("recv timeout")?
            .context("recv")?;

        match resp.msg {
            Some(Msg::StoreAnnounceAck(ack)) => Ok(ack.success),
            _ => Ok(false),
        }
    }
}

// ─── Standalone RPC functions (usable from spawned tasks) ───

async fn rpc_find_node_raw(
    endpoint: &quinn::Endpoint,
    addr: SocketAddr,
    local_key: &Key,
    target: &Key,
) -> Result<Vec<(Key, SocketAddr)>> {
    let connecting = endpoint
        .connect(addr, "ldrive.local")
        .context("connect")?;
    let conn = tokio::time::timeout(RPC_TIMEOUT, connecting)
        .await
        .context("connect timeout")?
        .context("handshake")?;

    let (mut send, mut recv) = conn.open_bi().await?;

    let msg = PeerMessage {
        msg: Some(Msg::FindNode(ldrive_proto::FindNode {
            sender_id: local_key.as_bytes().to_vec(),
            target: target.as_bytes().to_vec(),
        })),
    };
    send_message(&mut send, &msg).await?;
    send.finish()?;

    let resp = tokio::time::timeout(RPC_TIMEOUT, recv_message(&mut recv))
        .await
        .context("recv timeout")?
        .context("recv")?;

    match resp.msg {
        Some(Msg::FindNodeResponse(resp)) => Ok(parse_peer_list(&resp.peers)),
        _ => anyhow::bail!("unexpected response to FIND_NODE"),
    }
}

enum FindValueResult {
    Found(Vec<(Key, SocketAddr)>),
    CloserPeers(Vec<(Key, SocketAddr)>),
}

async fn rpc_find_value_raw(
    endpoint: &quinn::Endpoint,
    addr: SocketAddr,
    local_key: &Key,
    content_key: &Key,
) -> Result<FindValueResult> {
    let connecting = endpoint
        .connect(addr, "ldrive.local")
        .context("connect")?;
    let conn = tokio::time::timeout(RPC_TIMEOUT, connecting)
        .await
        .context("connect timeout")?
        .context("handshake")?;

    let (mut send, mut recv) = conn.open_bi().await?;

    let msg = PeerMessage {
        msg: Some(Msg::FindValue(ldrive_proto::FindValue {
            sender_id: local_key.as_bytes().to_vec(),
            key: content_key.as_bytes().to_vec(),
        })),
    };
    send_message(&mut send, &msg).await?;
    send.finish()?;

    let resp = tokio::time::timeout(RPC_TIMEOUT, recv_message(&mut recv))
        .await
        .context("recv timeout")?
        .context("recv")?;

    match resp.msg {
        Some(Msg::FindValueResponse(resp)) => {
            if resp.found {
                Ok(FindValueResult::Found(parse_peer_list(&resp.holders)))
            } else {
                Ok(FindValueResult::CloserPeers(parse_peer_list(
                    &resp.closer_peers,
                )))
            }
        }
        _ => anyhow::bail!("unexpected response to FIND_VALUE"),
    }
}

fn parse_key_20(bytes: &[u8]) -> Option<Key> {
    if bytes.len() == 20 {
        let mut k = [0u8; 20];
        k.copy_from_slice(bytes);
        Some(Key(k))
    } else {
        None
    }
}

fn parse_peer_list(peers: &[ldrive_proto::PeerInfo]) -> Vec<(Key, SocketAddr)> {
    peers
        .iter()
        .filter_map(|p| {
            let key = parse_key_20(&p.node_id)?;
            let addr = p.addr.parse().ok()?;
            Some((key, addr))
        })
        .collect()
}
