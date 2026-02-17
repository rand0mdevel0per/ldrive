use anyhow::{Context, Result};
use std::net::SocketAddr;
use tokio::net::UdpSocket;
use tracing::{info, debug};

/// Result of a STUN probe
#[derive(Debug, Clone)]
pub struct StunResult {
    pub nat_type: NatType,
    /// Our public address as seen by the STUN server (if available)
    pub public_addr: Option<SocketAddr>,
    /// Our local address
    pub local_addr: SocketAddr,
}

/// Detected NAT type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NatType {
    /// Direct public IP or full cone NAT — easiest for P2P
    OpenOrFullCone,
    /// Restricted cone NAT — can hole-punch with coordination
    RestrictedCone,
    /// Symmetric NAT — requires relay
    Symmetric,
    /// Could not determine (STUN failed)
    Unknown,
}

/// Lightweight STUN binding request (RFC 5389 minimal).
///
/// Sends a STUN Binding Request and parses the XOR-MAPPED-ADDRESS from the response.
/// This is a simplified implementation — enough to discover our public IP:port.
pub async fn stun_probe(stun_server: &str) -> Result<StunResult> {
    let socket = UdpSocket::bind("0.0.0.0:0")
        .await
        .context("binding UDP for STUN")?;
    let local_addr = socket.local_addr()?;

    let stun_addr: SocketAddr = tokio::net::lookup_host(stun_server)
        .await
        .context("resolving STUN server")?
        .next()
        .ok_or_else(|| anyhow::anyhow!("no address for STUN server"))?;

    // Build minimal STUN Binding Request (RFC 5389)
    // Header: type(2) + length(2) + magic_cookie(4) + transaction_id(12) = 20 bytes
    let mut req = [0u8; 20];
    // Binding Request type = 0x0001
    req[0] = 0x00;
    req[1] = 0x01;
    // Message length = 0 (no attributes)
    req[2] = 0x00;
    req[3] = 0x00;
    // Magic cookie = 0x2112A442
    req[4] = 0x21;
    req[5] = 0x12;
    req[6] = 0xA4;
    req[7] = 0x42;
    // Transaction ID = random 12 bytes
    let tx_id: [u8; 12] = rand::random();
    req[8..20].copy_from_slice(&tx_id);

    socket.send_to(&req, stun_addr).await?;

    let mut buf = [0u8; 512];
    let timeout = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        socket.recv_from(&mut buf),
    )
    .await;

    match timeout {
        Ok(Ok((n, _from))) => {
            if n < 20 {
                return Ok(StunResult {
                    nat_type: NatType::Unknown,
                    public_addr: None,
                    local_addr,
                });
            }

            // Verify it's a Binding Response (0x0101)
            if buf[0] != 0x01 || buf[1] != 0x01 {
                return Ok(StunResult {
                    nat_type: NatType::Unknown,
                    public_addr: None,
                    local_addr,
                });
            }

            // Parse attributes looking for XOR-MAPPED-ADDRESS (0x0020) or MAPPED-ADDRESS (0x0001)
            let msg_len = u16::from_be_bytes([buf[2], buf[3]]) as usize;
            let public_addr = parse_xor_mapped_address(&buf[20..20 + msg_len.min(n - 20)], &tx_id);

            if let Some(addr) = public_addr {
                info!(local = %local_addr, public = %addr, "STUN probe succeeded");

                // Simple NAT type heuristic:
                // If public port == local port, likely open/full cone
                // If public IP == local IP, we have a public IP
                let nat_type = if addr.ip() == local_addr.ip() || addr.port() == local_addr.port() {
                    NatType::OpenOrFullCone
                } else {
                    // Could be restricted or symmetric — need a second STUN server to tell
                    NatType::RestrictedCone
                };

                Ok(StunResult {
                    nat_type,
                    public_addr: Some(addr),
                    local_addr,
                })
            } else {
                Ok(StunResult {
                    nat_type: NatType::Unknown,
                    public_addr: None,
                    local_addr,
                })
            }
        }
        _ => {
            debug!("STUN probe timed out or failed");
            Ok(StunResult {
                nat_type: NatType::Unknown,
                public_addr: None,
                local_addr,
            })
        }
    }
}

/// Parse XOR-MAPPED-ADDRESS (0x0020) from STUN response attributes.
fn parse_xor_mapped_address(attrs: &[u8], _tx_id: &[u8; 12]) -> Option<SocketAddr> {
    let magic_cookie: u32 = 0x2112A442;
    let mut offset = 0;

    while offset + 4 <= attrs.len() {
        let attr_type = u16::from_be_bytes([attrs[offset], attrs[offset + 1]]);
        let attr_len = u16::from_be_bytes([attrs[offset + 2], attrs[offset + 3]]) as usize;
        offset += 4;

        if offset + attr_len > attrs.len() {
            break;
        }

        match attr_type {
            0x0020 => {
                // XOR-MAPPED-ADDRESS
                if attr_len >= 8 {
                    let family = attrs[offset + 1];
                    let xor_port =
                        u16::from_be_bytes([attrs[offset + 2], attrs[offset + 3]]) ^ (magic_cookie >> 16) as u16;

                    if family == 0x01 && attr_len >= 8 {
                        // IPv4
                        let xor_ip = u32::from_be_bytes([
                            attrs[offset + 4],
                            attrs[offset + 5],
                            attrs[offset + 6],
                            attrs[offset + 7],
                        ]) ^ magic_cookie;
                        let ip = std::net::Ipv4Addr::from(xor_ip);
                        return Some(SocketAddr::new(ip.into(), xor_port));
                    }
                }
            }
            0x0001 => {
                // MAPPED-ADDRESS (fallback)
                if attr_len >= 8 {
                    let family = attrs[offset + 1];
                    let port = u16::from_be_bytes([attrs[offset + 2], attrs[offset + 3]]);

                    if family == 0x01 {
                        let ip = std::net::Ipv4Addr::new(
                            attrs[offset + 4],
                            attrs[offset + 5],
                            attrs[offset + 6],
                            attrs[offset + 7],
                        );
                        return Some(SocketAddr::new(ip.into(), port));
                    }
                }
            }
            _ => {}
        }

        // Attributes are padded to 4-byte boundaries
        offset += (attr_len + 3) & !3;
    }

    None
}
