use anyhow::Result;
use crate::QuicServer;
use tracing::{info, warn, debug};

/// A relay server that forwards messages between peers.
/// Gateway nodes run this to help NAT'd peers communicate.
pub struct RelayServer {
    server: QuicServer,
}

impl RelayServer {
    pub fn new(server: QuicServer) -> Self {
        Self { server }
    }

    /// Run the relay server. Accepts connections and forwards bidirectional streams.
    /// This is a simple TURN-like relay: peer A connects, sends target addr,
    /// relay connects to B and pipes the streams together.
    pub async fn run(&self) -> Result<()> {
        let addr = self.server.local_addr()?;
        info!(addr = %addr, "relay server running");

        loop {
            let conn = match self.server.accept().await {
                Ok(c) => c,
                Err(e) => {
                    warn!("relay accept error: {}", e);
                    continue;
                }
            };

            let remote = conn.remote_addr;
            debug!(remote = %remote, "relay: peer connected");

            // Handle relay connections in a spawned task
            tokio::spawn(async move {
                // For now, relay just accepts and holds the connection.
                // Full relay implementation will:
                // 1. Accept a "relay request" with target peer addr
                // 2. Connect to target peer
                // 3. Pipe bidirectional streams between source and target
                //
                // This will be fully implemented when we have the relay protocol messages.
                loop {
                    match conn.inner.accept_bi().await {
                        Ok((_send, _recv)) => {
                            debug!(remote = %remote, "relay: stream opened (relay forwarding TBD)");
                        }
                        Err(quinn::ConnectionError::ApplicationClosed(_)) => {
                            debug!(remote = %remote, "relay: peer disconnected");
                            break;
                        }
                        Err(e) => {
                            warn!(remote = %remote, err = %e, "relay: stream error");
                            break;
                        }
                    }
                }
            });
        }
    }
}
