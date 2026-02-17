use anyhow::{Context, Result};
use ldrive_chunk::chunk_file;
use ldrive_dht::{DhtNode, VnodeRing, PhysicalNode};
use ldrive_net::{QuicClient, send_message, recv_message};
use ldrive_proto::{PeerMessage, peer_message::Msg};
use ldrive_store::ChunkStore;
use std::net::SocketAddr;
use std::path::PathBuf;
use tracing::{info, warn};

pub async fn run(
    file: PathBuf,
    listen: SocketAddr,
    storage_path: PathBuf,
    bootstrap: Vec<SocketAddr>,
) -> Result<()> {
    info!(file = %file.display(), "publishing file to DHT network");

    // Chunk the file
    let chunked = chunk_file(&file).context("chunking file")?;
    info!(
        chunks = chunked.pieces.len(),
        file_hash = %chunked.manifest.file_hash,
        file_size = chunked.manifest.file_size,
        erasure = chunked.manifest.erasure.is_some(),
        groups = chunked.manifest.group_count(),
        "file chunked with erasure coding"
    );

    // Store chunks locally
    let store = ChunkStore::open(&storage_path, u64::MAX)?;
    for piece in &chunked.pieces {
        store.put_chunk(&piece.hash, &piece.data)?;
    }
    store.put_manifest(&chunked.manifest)?;
    info!("chunks stored locally");

    // Start DHT node and bootstrap
    let dht = DhtNode::new(listen).await?;
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

    // Distribute shards using 3+3 strategy per erasure group
    let client = QuicClient::new().await?;
    let local_region = "default";
    let mut distributed = 0;

    if let Some(_erasure) = &chunked.manifest.erasure {
        let group_count = chunked.manifest.group_count();
        for group_idx in 0..group_count {
            let group_shards: Vec<_> = chunked.pieces.iter()
                .filter(|p| chunked.manifest.chunks[p.index as usize].group_index == group_idx)
                .collect();

            let group_hash = format!("group_{}", group_idx);
            let (in_region, out_region) = ring.find_nodes_with_affinity(
                group_hash.as_bytes(),
                local_region,
                3,
                3,
            );

            let target_nodes: Vec<_> = in_region.into_iter().chain(out_region).take(6).collect();

            for (shard, node) in group_shards.iter().zip(target_nodes.iter()) {
                match push_chunk(&client, node.addr, &shard.hash, &shard.data).await {
                    Ok(_) => {
                        distributed += 1;
                        info!(chunk = %shard.hash, node = %node.addr, "shard distributed");
                    }
                    Err(e) => {
                        warn!(chunk = %shard.hash, node = %node.addr, err = %e, "failed to push shard");
                    }
                }
            }
        }
    }

    info!(
        file_hash = %chunked.manifest.file_hash,
        chunks = chunked.pieces.len(),
        distributed = distributed,
        "file published with 3+3 distribution"
    );
    info!("file hash: {}", chunked.manifest.file_hash);
    info!("to fetch: ldrive-node fetch {} --bootstrap <addr>", chunked.manifest.file_hash);

    // Keep running to serve chunk requests
    info!("keeping node alive to serve chunks... press Ctrl+C to stop");
    tokio::signal::ctrl_c().await?;

    Ok(())
}

async fn push_chunk(
    client: &QuicClient,
    addr: SocketAddr,
    hash: &ldrive_common::ChunkHash,
    data: &[u8],
) -> Result<()> {
    let conn = client.connect(addr).await?;
    let (mut send, mut recv) = conn.open_bi().await?;

    let msg = PeerMessage {
        msg: Some(Msg::ChunkPush(ldrive_proto::ChunkPush {
            hash: hash.0.to_vec(),
            data: data.to_vec(),
            size: data.len() as u32,
        })),
    };

    send_message(&mut send, &msg).await?;
    send.finish()?;

    let ack = recv_message(&mut recv).await?;
    match ack.msg {
        Some(Msg::ChunkPushAck(ack)) if ack.success => Ok(()),
        Some(Msg::ChunkPushAck(ack)) => anyhow::bail!("push rejected: {}", ack.error),
        _ => anyhow::bail!("unexpected response"),
    }
}
