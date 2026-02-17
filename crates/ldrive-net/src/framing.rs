use anyhow::{Context, Result};
use bytes::{BufMut, BytesMut};
use ldrive_proto::PeerMessage;
use prost::Message;
use quinn::{RecvStream, SendStream};

/// Maximum message size: 4 MB (256KB max chunk + overhead)
const MAX_MESSAGE_SIZE: u32 = 4 * 1024 * 1024;

/// Send a protobuf message over a QUIC send stream.
/// Wire format: [4-byte big-endian length][protobuf payload]
pub async fn send_message(stream: &mut SendStream, msg: &PeerMessage) -> Result<()> {
    let payload = msg.encode_to_vec();
    let len = payload.len() as u32;

    if len > MAX_MESSAGE_SIZE {
        anyhow::bail!("message too large: {} bytes (max {})", len, MAX_MESSAGE_SIZE);
    }

    let mut buf = BytesMut::with_capacity(4 + payload.len());
    buf.put_u32(len);
    buf.put_slice(&payload);

    stream
        .write_all(&buf)
        .await
        .context("sending message")?;

    Ok(())
}

/// Receive a protobuf message from a QUIC recv stream.
pub async fn recv_message(stream: &mut RecvStream) -> Result<PeerMessage> {
    // Read 4-byte length prefix
    let mut len_buf = [0u8; 4];
    stream
        .read_exact(&mut len_buf)
        .await
        .context("reading message length")?;
    let len = u32::from_be_bytes(len_buf);

    if len > MAX_MESSAGE_SIZE {
        anyhow::bail!(
            "incoming message too large: {} bytes (max {})",
            len,
            MAX_MESSAGE_SIZE
        );
    }

    // Read payload
    let mut payload = vec![0u8; len as usize];
    stream
        .read_exact(&mut payload)
        .await
        .context("reading message payload")?;

    let msg = PeerMessage::decode(&payload[..]).context("decoding protobuf message")?;
    Ok(msg)
}
