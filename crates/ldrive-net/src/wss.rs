use anyhow::{Context, Result};
use bytes::{BufMut, BytesMut};
use futures_util::{SinkExt, StreamExt};
use ldrive_proto::PeerMessage;
use prost::Message;
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{
    accept_async, connect_async,
    tungstenite::protocol::Message as WsMessage,
};
use tracing::{info, debug};

/// Maximum message size for WebSocket transport
const MAX_MESSAGE_SIZE: usize = 4 * 1024 * 1024;

/// A WebSocket connection (wraps a tungstenite WebSocket stream)
pub struct WsConnection {
    stream: tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<TcpStream>>,
    pub remote_addr: SocketAddr,
}

/// Server-side WebSocket connection
pub struct WsServerConnection {
    stream: tokio_tungstenite::WebSocketStream<TcpStream>,
    pub remote_addr: SocketAddr,
}

/// WebSocket server for accepting connections
pub struct WsServer {
    listener: TcpListener,
}

impl WsServer {
    pub async fn bind(addr: SocketAddr) -> Result<Self> {
        let listener = TcpListener::bind(addr).await
            .with_context(|| format!("binding WSS server on {}", addr))?;
        info!(addr = %addr, "WebSocket server listening");
        Ok(Self { listener })
    }

    pub fn local_addr(&self) -> Result<SocketAddr> {
        self.listener.local_addr().context("getting local addr")
    }

    pub async fn accept(&self) -> Result<WsServerConnection> {
        let (tcp_stream, remote_addr) = self.listener.accept().await
            .context("accepting TCP connection")?;

        let ws_stream = accept_async(tcp_stream).await
            .context("WebSocket handshake")?;

        debug!(remote = %remote_addr, "WebSocket client connected");

        Ok(WsServerConnection {
            stream: ws_stream,
            remote_addr,
        })
    }
}

impl WsServerConnection {
    pub async fn send_message(&mut self, msg: &PeerMessage) -> Result<()> {
        let payload = msg.encode_to_vec();
        if payload.len() > MAX_MESSAGE_SIZE {
            anyhow::bail!("message too large: {} bytes", payload.len());
        }

        // Prepend 4-byte length (same framing as QUIC for consistency)
        let mut buf = BytesMut::with_capacity(4 + payload.len());
        buf.put_u32(payload.len() as u32);
        buf.put_slice(&payload);

        self.stream.send(WsMessage::Binary(buf.to_vec())).await
            .context("sending WSS message")?;
        Ok(())
    }

    pub async fn recv_message(&mut self) -> Result<PeerMessage> {
        loop {
            match self.stream.next().await {
                Some(Ok(WsMessage::Binary(data))) => {
                    if data.len() < 4 {
                        anyhow::bail!("WSS message too short");
                    }
                    let len = u32::from_be_bytes([data[0], data[1], data[2], data[3]]) as usize;
                    if data.len() < 4 + len {
                        anyhow::bail!("WSS message truncated");
                    }
                    let msg = PeerMessage::decode(&data[4..4 + len])
                        .context("decoding protobuf")?;
                    return Ok(msg);
                }
                Some(Ok(WsMessage::Ping(payload))) => {
                    self.stream.send(WsMessage::Pong(payload)).await?;
                    continue;
                }
                Some(Ok(WsMessage::Close(_))) | None => {
                    anyhow::bail!("WebSocket closed");
                }
                Some(Ok(_)) => continue, // ignore text, pong, etc.
                Some(Err(e)) => {
                    anyhow::bail!("WSS recv error: {}", e);
                }
            }
        }
    }
}

/// Connect to a remote peer via WebSocket.
pub async fn ws_connect(url: &str) -> Result<WsConnection> {
    let (ws_stream, _response) = connect_async(url).await
        .with_context(|| format!("connecting to {}", url))?;

    // Extract remote addr from URL (best effort)
    let remote_addr = url
        .replace("ws://", "")
        .replace("wss://", "")
        .parse()
        .unwrap_or_else(|_| "0.0.0.0:0".parse().unwrap());

    debug!(url = %url, "WebSocket connected");

    Ok(WsConnection {
        stream: ws_stream,
        remote_addr,
    })
}

impl WsConnection {
    pub async fn send_message(&mut self, msg: &PeerMessage) -> Result<()> {
        let payload = msg.encode_to_vec();
        if payload.len() > MAX_MESSAGE_SIZE {
            anyhow::bail!("message too large: {} bytes", payload.len());
        }

        let mut buf = BytesMut::with_capacity(4 + payload.len());
        buf.put_u32(payload.len() as u32);
        buf.put_slice(&payload);

        self.stream.send(WsMessage::Binary(buf.to_vec())).await
            .context("sending WSS message")?;
        Ok(())
    }

    pub async fn recv_message(&mut self) -> Result<PeerMessage> {
        loop {
            match self.stream.next().await {
                Some(Ok(WsMessage::Binary(data))) => {
                    if data.len() < 4 {
                        anyhow::bail!("WSS message too short");
                    }
                    let len = u32::from_be_bytes([data[0], data[1], data[2], data[3]]) as usize;
                    if data.len() < 4 + len {
                        anyhow::bail!("WSS message truncated");
                    }
                    let msg = PeerMessage::decode(&data[4..4 + len])
                        .context("decoding protobuf")?;
                    return Ok(msg);
                }
                Some(Ok(WsMessage::Ping(payload))) => {
                    self.stream.send(WsMessage::Pong(payload)).await?;
                    continue;
                }
                Some(Ok(WsMessage::Close(_))) | None => {
                    anyhow::bail!("WebSocket closed");
                }
                Some(Ok(_)) => continue,
                Some(Err(e)) => {
                    anyhow::bail!("WSS recv error: {}", e);
                }
            }
        }
    }
}
