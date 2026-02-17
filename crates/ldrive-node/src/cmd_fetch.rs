use anyhow::{Context, Result};
use ldrive_chunk::reassemble;
use ldrive_common::{ChunkHash, FileHash, Manifest};
use ldrive_dht::{DhtNode, VnodeRing, PhysicalNode};
use ldrive_net::{QuicClient, send_message, recv_message};
use ldrive_proto::{PeerMessage, peer_message::Msg};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use tracing::{info, warn};

pub async fn run(
    hash_hex: String,
    output: PathBuf,
    bootstrap: Vec<SocketAddr>,
) -> Result<()> {
    let hash_bytes = hex::decode(&hash_hex).context("invalid hex hash")?;
    let file_hash = FileHash::from_bytes(&hash_bytes)
        .ok_or_else(|| anyhow::anyhow!("hash must be 32 bytes (64 hex chars)"))?;

    info!(file_hash = %file_hash, "fetching file from DHT network");

    // Start DHT node on ephemeral port
    let dht = DhtNode::new("0.0.0.0:0".parse().unwrap()).await?;
    if !bootstrap.is_empty() {
        dht.bootstrap(&bootstrap).await?;
    }

    // Build VnodeRing from known peers
    let peers = dht.known_peers().await;
    let mut ring = VnodeRing::new();
    for (key, addr) in peers {
        ring.add_node(PhysicalNode {
            node_id: ldrive_common::NodeId(key.0),
            addr,
            region: "unknown".to_string(),
        });
    }

    // Look up manifest holders via VnodeRing
    let manifest_hash = format!("manifest_{}", file_hash);
    let (in_region, out_region) = ring.find_nodes_with_affinity(
        manifest_hash.as_bytes(),
        "default",
        3,
        3,
    );
    let holders: Vec<_> = in_region.into_iter().chain(out_region).take(6).collect();

    if holders.is_empty() {
        anyhow::bail!("no peers found holding manifest for {}", file_hash);
    }

    info!(holders = holders.len(), "found manifest holders");

    // Fetch manifest from first available holder
    let client = QuicClient::new().await?;
    let mut manifest: Option<Manifest> = None;

    for node in &holders {
        match fetch_manifest(&client, node.addr, &file_hash).await {
            Ok(m) => {
                info!(
                    file = %m.file_name,
                    chunks = m.total_chunks(),
                    size = m.file_size,
                    erasure = m.erasure.is_some(),
                    "manifest retrieved"
                );
                manifest = Some(m);
                break;
            }
            Err(e) => {
                warn!(addr = %node.addr, err = %e, "failed to fetch manifest from peer");
            }
        }
    }

    let manifest = manifest.ok_or_else(|| anyhow::anyhow!("could not retrieve manifest from any peer"))?;

    // For each chunk/shard in the manifest, find holders via VnodeRing and fetch
    let mut chunks: HashMap<ChunkHash, Vec<u8>> = HashMap::new();
    let total = manifest.total_chunks();

    for (i, chunk_meta) in manifest.chunks.iter().enumerate() {
        let chunk_hash = format!("chunk_{}", chunk_meta.hash);
        let (in_region, out_region) = ring.find_nodes_with_affinity(
            chunk_hash.as_bytes(),
            "default",
            3,
            3,
        );
        let chunk_holders: Vec<_> = in_region.into_iter().chain(out_region).take(6).collect();

        let mut fetched = false;
        for node in &chunk_holders {
            match fetch_chunk(&client, node.addr, &chunk_meta.hash).await {
                Ok(data) => {
                    if chunk_meta.hash.verify(&data) {
                        chunks.insert(chunk_meta.hash, data);
                        fetched = true;
                        break;
                    } else {
                        warn!(chunk = %chunk_meta.hash, addr = %node.addr, "chunk hash mismatch");
                    }
                }
                Err(e) => {
                    warn!(chunk = %chunk_meta.hash, addr = %node.addr, err = %e, "failed to fetch chunk");
                }
            }
        }

        // If VnodeRing lookup didn't find holders, try the manifest holders directly
        if !fetched {
            for node in &holders {
                match fetch_chunk(&client, node.addr, &chunk_meta.hash).await {
                    Ok(data) => {
                        if chunk_meta.hash.verify(&data) {
                            chunks.insert(chunk_meta.hash, data);
                            fetched = true;
                            break;
                        }
                    }
                    Err(_) => {}
                }
            }
        }

        if fetched {
            info!(
                chunk = %chunk_meta.hash,
                shard_type = ?chunk_meta.shard_type,
                "shard fetched ({}/{})",
                i + 1,
                total
            );
        } else {
            // For erasure-coded files, missing shards may be recoverable
            if manifest.erasure.is_some() {
                warn!(
                    chunk = %chunk_meta.hash,
                    group = chunk_meta.group_index,
                    shard = chunk_meta.shard_index,
                    "shard unavailable, will attempt RS recovery"
                );
            } else {
                anyhow::bail!("could not fetch chunk {} from any peer", chunk_meta.hash);
            }
        }
    }

    // Reassemble (erasure decoding handles missing shards automatically)
    std::fs::create_dir_all(&output)?;
    let output_path = output.join(&manifest.file_name);
    reassemble(&manifest, &output_path, |hash| {
        chunks
            .get(hash)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("missing chunk {}", hash))
    })?;

    info!(
        path = %output_path.display(),
        size = manifest.file_size,
        "file downloaded successfully"
    );

    Ok(())
}

async fn fetch_manifest(
    client: &QuicClient,
    addr: SocketAddr,
    file_hash: &FileHash,
) -> Result<Manifest> {
    let conn = client.connect(addr).await?;
    let (mut send, mut recv) = conn.open_bi().await?;

    let msg = PeerMessage {
        msg: Some(Msg::ManifestRequest(ldrive_proto::ManifestRequest {
            file_hash: file_hash.0.to_vec(),
        })),
    };
    send_message(&mut send, &msg).await?;
    send.finish()?;

    let resp = recv_message(&mut recv).await?;
    match resp.msg {
        Some(Msg::ManifestResponse(resp)) if resp.found => {
            let push = ldrive_proto::ManifestPush {
                file_hash: resp.file_hash,
                file_name: resp.file_name,
                file_size: resp.file_size,
                chunks: resp.chunks,
                erasure_data_shards: resp.erasure_data_shards,
                erasure_parity_shards: resp.erasure_parity_shards,
            };
            Manifest::from_proto_push(&push)
                .ok_or_else(|| anyhow::anyhow!("invalid manifest data"))
        }
        Some(Msg::ManifestResponse(_)) => {
            anyhow::bail!("manifest not found on peer")
        }
        _ => anyhow::bail!("unexpected response"),
    }
}

async fn fetch_chunk(
    client: &QuicClient,
    addr: SocketAddr,
    chunk_hash: &ChunkHash,
) -> Result<Vec<u8>> {
    let conn = client.connect(addr).await?;
    let (mut send, mut recv) = conn.open_bi().await?;

    let msg = PeerMessage {
        msg: Some(Msg::ChunkRequest(ldrive_proto::ChunkRequest {
            hash: chunk_hash.0.to_vec(),
        })),
    };
    send_message(&mut send, &msg).await?;
    send.finish()?;

    let resp = recv_message(&mut recv).await?;
    match resp.msg {
        Some(Msg::ChunkResponse(resp)) if resp.found => Ok(resp.data),
        Some(Msg::ChunkResponse(_)) => anyhow::bail!("chunk not found"),
        _ => anyhow::bail!("unexpected response"),
    }
}
