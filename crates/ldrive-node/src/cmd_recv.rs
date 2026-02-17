use anyhow::{Context, Result};
use ldrive_chunk::reassemble;
use ldrive_common::{ChunkHash, FileHash, Manifest};
use ldrive_net::{QuicServer, recv_message, send_message};
use ldrive_proto::{PeerMessage, peer_message::Msg};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use tracing::{info, warn};

pub async fn run(listen: SocketAddr, output: PathBuf) -> Result<()> {
    std::fs::create_dir_all(&output)?;

    let server = QuicServer::bind(listen).await?;
    info!(
        node_id = %server.node_id(),
        addr = %server.local_addr()?,
        "receiver listening"
    );

    // Accept one connection and receive one file
    let conn = server.accept().await?;
    info!(remote = %conn.remote_addr, "sender connected");

    let mut chunks: HashMap<ChunkHash, Vec<u8>> = HashMap::new();
    let mut manifest: Option<Manifest> = None;

    loop {
        // Accept bidirectional streams from the sender
        let (mut send, mut recv) = match conn.inner.accept_bi().await {
            Ok(streams) => streams,
            Err(quinn::ConnectionError::ApplicationClosed(_)) => {
                info!("connection closed by sender");
                break;
            }
            Err(e) => {
                warn!("connection error: {}", e);
                break;
            }
        };

        let msg = recv_message(&mut recv).await?;

        match msg.msg {
            Some(Msg::ChunkPush(push)) => {
                let hash = ChunkHash::from_bytes(&push.hash)
                    .ok_or_else(|| anyhow::anyhow!("invalid chunk hash"))?;

                // Verify hash
                let computed = ChunkHash::compute(&push.data);
                if computed != hash {
                    let ack = PeerMessage {
                        msg: Some(Msg::ChunkPushAck(ldrive_proto::ChunkPushAck {
                            hash: push.hash,
                            success: false,
                            error: "hash mismatch".to_string(),
                        })),
                    };
                    send_message(&mut send, &ack).await?;
                    continue;
                }

                info!(chunk = %hash, size = push.data.len(), "chunk received");
                chunks.insert(hash, push.data);

                let ack = PeerMessage {
                    msg: Some(Msg::ChunkPushAck(ldrive_proto::ChunkPushAck {
                        hash: push.hash,
                        success: true,
                        error: String::new(),
                    })),
                };
                send_message(&mut send, &ack).await?;
            }

            Some(Msg::ManifestPush(push)) => {
                let m = Manifest::from_proto_push(&push)
                    .ok_or_else(|| anyhow::anyhow!("invalid manifest"))?;

                info!(
                    file = %m.file_name,
                    hash = %m.file_hash,
                    chunks = m.total_chunks(),
                    "manifest received"
                );

                manifest = Some(m);

                let ack = PeerMessage {
                    msg: Some(Msg::ManifestPushAck(ldrive_proto::ManifestPushAck {
                        file_hash: push.file_hash,
                        success: true,
                        error: String::new(),
                    })),
                };
                send_message(&mut send, &ack).await?;
            }

            Some(Msg::TransferComplete(tc)) => {
                let file_hash = FileHash::from_bytes(&tc.file_hash)
                    .ok_or_else(|| anyhow::anyhow!("invalid file hash"))?;

                info!(
                    file_hash = %file_hash,
                    total_chunks = tc.total_chunks,
                    received_chunks = chunks.len(),
                    "transfer complete signal received"
                );

                // Reassemble the file
                if let Some(ref m) = manifest {
                    let output_path = output.join(&m.file_name);
                    reassemble(m, &output_path, |hash| {
                        chunks
                            .get(hash)
                            .cloned()
                            .ok_or_else(|| anyhow::anyhow!("missing chunk {}", hash))
                    })
                    .context("reassembling file")?;

                    info!(path = %output_path.display(), "file reassembled successfully");

                    let ack = PeerMessage {
                        msg: Some(Msg::TransferCompleteAck(
                            ldrive_proto::TransferCompleteAck {
                                file_hash: tc.file_hash,
                                success: true,
                                error: String::new(),
                            },
                        )),
                    };
                    send_message(&mut send, &ack).await?;
                } else {
                    let ack = PeerMessage {
                        msg: Some(Msg::TransferCompleteAck(
                            ldrive_proto::TransferCompleteAck {
                                file_hash: tc.file_hash,
                                success: false,
                                error: "no manifest received".to_string(),
                            },
                        )),
                    };
                    send_message(&mut send, &ack).await?;
                }

                break;
            }

            other => {
                warn!("unexpected message: {:?}", other);
            }
        }
    }

    info!("receiver done");
    Ok(())
}
