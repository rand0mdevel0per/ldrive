use anyhow::{Context, Result};
use ldrive_common::ChunkHash;
use ldrive_dht::DhtNode;
use ldrive_net::{recv_message, send_message};
use ldrive_proto::{PeerMessage, peer_message::Msg};
use ldrive_store::ChunkStore;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{info, warn};

pub async fn run(
    listen: SocketAddr,
    storage_path: PathBuf,
    quota: u64,
    bootstrap: Vec<SocketAddr>,
) -> Result<()> {
    let store = Arc::new(ChunkStore::open(&storage_path, quota).context("opening chunk store")?);
    let dht = Arc::new(DhtNode::new(listen).await?);

    info!(
        node_id = %dht.node_id(),
        addr = %dht.local_addr()?,
        storage = %storage_path.display(),
        quota_gb = quota / (1024 * 1024 * 1024),
        "storage node starting"
    );

    // Bootstrap DHT
    if !bootstrap.is_empty() {
        dht.bootstrap(&bootstrap).await?;
    }

    info!(peers = dht.peer_count().await, "DHT ready, accepting connections");

    loop {
        let conn = match dht.server.accept().await {
            Ok(c) => c,
            Err(e) => {
                warn!("accept error: {}", e);
                continue;
            }
        };

        let store = store.clone();
        let dht = dht.clone();
        let remote = conn.remote_addr;

        tokio::spawn(async move {
            info!(remote = %remote, "peer connected");

            loop {
                let (mut send, mut recv) = match conn.inner.accept_bi().await {
                    Ok(streams) => streams,
                    Err(quinn::ConnectionError::ApplicationClosed(_)) => {
                        info!(remote = %remote, "peer disconnected");
                        break;
                    }
                    Err(e) => {
                        warn!("stream error: {}", e);
                        break;
                    }
                };

                let msg = match recv_message(&mut recv).await {
                    Ok(m) => m,
                    Err(e) => {
                        warn!("message read error: {}", e);
                        continue;
                    }
                };

                // Try DHT handler first
                if let Some(response) = dht.handle_message(&msg, remote).await {
                    let _ = send_message(&mut send, &response).await;
                    continue;
                }

                // Handle storage messages
                match msg.msg {
                    Some(Msg::ChunkPush(push)) => {
                        let hash = match ChunkHash::from_bytes(&push.hash) {
                            Some(h) => h,
                            None => {
                                let ack = PeerMessage {
                                    msg: Some(Msg::ChunkPushAck(ldrive_proto::ChunkPushAck {
                                        hash: push.hash,
                                        success: false,
                                        error: "invalid hash".to_string(),
                                    })),
                                };
                                let _ = send_message(&mut send, &ack).await;
                                continue;
                            }
                        };

                        match store.put_chunk(&hash, &push.data) {
                            Ok(is_new) => {
                                info!(chunk = %hash, size = push.data.len(), new = is_new, "chunk stored");
                                let ack = PeerMessage {
                                    msg: Some(Msg::ChunkPushAck(ldrive_proto::ChunkPushAck {
                                        hash: push.hash,
                                        success: true,
                                        error: String::new(),
                                    })),
                                };
                                let _ = send_message(&mut send, &ack).await;
                            }
                            Err(e) => {
                                warn!("store error: {}", e);
                                let ack = PeerMessage {
                                    msg: Some(Msg::ChunkPushAck(ldrive_proto::ChunkPushAck {
                                        hash: push.hash,
                                        success: false,
                                        error: e.to_string(),
                                    })),
                                };
                                let _ = send_message(&mut send, &ack).await;
                            }
                        }
                    }

                    Some(Msg::ChunkRequest(req)) => {
                        let hash = match ChunkHash::from_bytes(&req.hash) {
                            Some(h) => h,
                            None => {
                                let resp = PeerMessage {
                                    msg: Some(Msg::ChunkResponse(ldrive_proto::ChunkResponse {
                                        hash: req.hash,
                                        data: vec![],
                                        found: false,
                                        error: "invalid hash".to_string(),
                                    })),
                                };
                                let _ = send_message(&mut send, &resp).await;
                                continue;
                            }
                        };

                        match store.get_chunk(&hash) {
                            Ok(Some(data)) => {
                                info!(chunk = %hash, size = data.len(), "chunk served");
                                let resp = PeerMessage {
                                    msg: Some(Msg::ChunkResponse(ldrive_proto::ChunkResponse {
                                        hash: req.hash,
                                        data,
                                        found: true,
                                        error: String::new(),
                                    })),
                                };
                                let _ = send_message(&mut send, &resp).await;
                            }
                            Ok(None) => {
                                let resp = PeerMessage {
                                    msg: Some(Msg::ChunkResponse(ldrive_proto::ChunkResponse {
                                        hash: req.hash,
                                        data: vec![],
                                        found: false,
                                        error: String::new(),
                                    })),
                                };
                                let _ = send_message(&mut send, &resp).await;
                            }
                            Err(e) => {
                                let resp = PeerMessage {
                                    msg: Some(Msg::ChunkResponse(ldrive_proto::ChunkResponse {
                                        hash: req.hash,
                                        data: vec![],
                                        found: false,
                                        error: e.to_string(),
                                    })),
                                };
                                let _ = send_message(&mut send, &resp).await;
                            }
                        }
                    }

                    Some(Msg::ManifestPush(push)) => {
                        if let Some(manifest) = ldrive_common::Manifest::from_proto_push(&push) {
                            info!(file = %manifest.file_name, hash = %manifest.file_hash, "manifest stored");
                            let _ = store.put_manifest(&manifest);
                            let ack = PeerMessage {
                                msg: Some(Msg::ManifestPushAck(ldrive_proto::ManifestPushAck {
                                    file_hash: push.file_hash,
                                    success: true,
                                    error: String::new(),
                                })),
                            };
                            let _ = send_message(&mut send, &ack).await;
                        }
                    }

                    Some(Msg::ManifestRequest(req)) => {
                        let file_hash = ldrive_common::FileHash::from_bytes(&req.file_hash);
                        if let Some(fh) = file_hash {
                            if let Ok(Some(manifest)) = store.get_manifest(&fh) {
                                let proto_push = manifest.to_proto_push();
                                let resp = PeerMessage {
                                    msg: Some(Msg::ManifestResponse(ldrive_proto::ManifestResponse {
                                        file_hash: manifest.file_hash.0.to_vec(),
                                        file_name: manifest.file_name.clone(),
                                        file_size: manifest.file_size,
                                        chunks: proto_push.chunks,
                                        found: true,
                                        error: String::new(),
                                        erasure_data_shards: proto_push.erasure_data_shards,
                                        erasure_parity_shards: proto_push.erasure_parity_shards,
                                    })),
                                };
                                let _ = send_message(&mut send, &resp).await;
                                continue;
                            }
                        }
                        let resp = PeerMessage {
                            msg: Some(Msg::ManifestResponse(ldrive_proto::ManifestResponse {
                                file_hash: req.file_hash,
                                file_name: String::new(),
                                file_size: 0,
                                chunks: vec![],
                                found: false,
                                error: String::new(),
                                erasure_data_shards: 0,
                                erasure_parity_shards: 0,
                            })),
                        };
                        let _ = send_message(&mut send, &resp).await;
                    }

                    Some(Msg::Ping(ping)) => {
                        let pong = PeerMessage {
                            msg: Some(Msg::Pong(ldrive_proto::Pong { nonce: ping.nonce })),
                        };
                        let _ = send_message(&mut send, &pong).await;
                    }

                    Some(Msg::Challenge(challenge)) => {
                        let hash = match ChunkHash::from_bytes(&challenge.chunk_hash) {
                            Some(h) => h,
                            None => continue,
                        };

                        let resp = match store.get_chunk(&hash) {
                            Ok(Some(data)) => {
                                let start = challenge.offset as usize;
                                let end = start + challenge.length as usize;

                                let proof = if end <= data.len() {
                                    let mut hasher = blake3::Hasher::new();
                                    hasher.update(&challenge.nonce);
                                    hasher.update(&data[start..end]);
                                    hasher.finalize().as_bytes().to_owned()
                                } else {
                                    [0u8; 32]
                                };

                                PeerMessage {
                                    msg: Some(Msg::ChallengeResponse(ldrive_proto::ChallengeResponse {
                                        chunk_hash: challenge.chunk_hash,
                                        proof: proof.to_vec(),
                                    })),
                                }
                            }
                            _ => continue,
                        };
                        let _ = send_message(&mut send, &resp).await;
                    }

                    other => {
                        warn!("unhandled message: {:?}", other);
                    }
                }
            }
        });
    }
}
