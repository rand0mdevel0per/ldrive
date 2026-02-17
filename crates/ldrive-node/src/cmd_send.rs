use anyhow::{Context, Result};
use ldrive_chunk::chunk_file_simple;
use ldrive_net::{QuicClient, send_message};
use ldrive_proto::{PeerMessage, peer_message::Msg};
use std::net::SocketAddr;
use std::path::PathBuf;
use tracing::info;

pub async fn run(file: PathBuf, peer: SocketAddr) -> Result<()> {
    info!(file = %file.display(), peer = %peer, "sending file");

    // Chunk the file
    let chunked = chunk_file_simple(&file).context("chunking file")?;
    info!(
        chunks = chunked.pieces.len(),
        file_hash = %chunked.manifest.file_hash,
        "file chunked"
    );

    // Connect to peer
    let client = QuicClient::new().await?;
    let conn = client.connect(peer).await?;

    // Send each chunk
    for piece in &chunked.pieces {
        let (mut send, mut recv) = conn.open_bi().await?;

        let msg = PeerMessage {
            msg: Some(Msg::ChunkPush(ldrive_proto::ChunkPush {
                hash: piece.hash.0.to_vec(),
                data: piece.data.clone(),
                size: piece.data.len() as u32,
            })),
        };

        send_message(&mut send, &msg).await?;
        send.finish()?;

        // Wait for ack
        let ack = ldrive_net::recv_message(&mut recv).await?;
        match ack.msg {
            Some(Msg::ChunkPushAck(ack)) => {
                if !ack.success {
                    anyhow::bail!("chunk push rejected: {}", ack.error);
                }
            }
            _ => anyhow::bail!("unexpected response to chunk push"),
        }

        info!(
            chunk = %piece.hash,
            index = piece.index,
            size = piece.data.len(),
            "chunk sent"
        );
    }

    // Send manifest
    {
        let (mut send, mut recv) = conn.open_bi().await?;

        let msg = PeerMessage {
            msg: Some(Msg::ManifestPush(chunked.manifest.to_proto_push())),
        };

        send_message(&mut send, &msg).await?;
        send.finish()?;

        let ack = ldrive_net::recv_message(&mut recv).await?;
        match ack.msg {
            Some(Msg::ManifestPushAck(ack)) => {
                if !ack.success {
                    anyhow::bail!("manifest push rejected: {}", ack.error);
                }
            }
            _ => anyhow::bail!("unexpected response to manifest push"),
        }

        info!(file_hash = %chunked.manifest.file_hash, "manifest sent");
    }

    // Send transfer complete
    {
        let (mut send, mut recv) = conn.open_bi().await?;

        let msg = PeerMessage {
            msg: Some(Msg::TransferComplete(ldrive_proto::TransferComplete {
                file_hash: chunked.manifest.file_hash.0.to_vec(),
                total_chunks: chunked.pieces.len() as u32,
            })),
        };

        send_message(&mut send, &msg).await?;
        send.finish()?;

        let ack = ldrive_net::recv_message(&mut recv).await?;
        match ack.msg {
            Some(Msg::TransferCompleteAck(ack)) => {
                if ack.success {
                    info!("transfer complete, file hash: {}", chunked.manifest.file_hash);
                } else {
                    anyhow::bail!("transfer complete rejected: {}", ack.error);
                }
            }
            _ => anyhow::bail!("unexpected response to transfer complete"),
        }
    }

    conn.inner.close(0u32.into(), b"done");
    client.endpoint.wait_idle().await;

    info!("file sent successfully");
    Ok(())
}
