use anyhow::{Context, Result};
use ldrive_common::NodeId;
use quinn::{Endpoint, ServerConfig, ClientConfig};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::info;

use crate::identity::NodeIdentity;

/// Wrapper around a quinn connection with peer identity info
pub struct Connection {
    pub inner: quinn::Connection,
    pub remote_addr: SocketAddr,
}

impl Connection {
    pub fn new(conn: quinn::Connection) -> Self {
        let remote_addr = conn.remote_address();
        Self {
            inner: conn,
            remote_addr,
        }
    }

    /// Open a new bidirectional stream for a request/response exchange.
    pub async fn open_bi(
        &self,
    ) -> Result<(quinn::SendStream, quinn::RecvStream)> {
        self.inner
            .open_bi()
            .await
            .context("opening bidirectional stream")
    }
}

/// QUIC server: listens for incoming connections
pub struct QuicServer {
    pub endpoint: Endpoint,
    pub identity: NodeIdentity,
}

impl QuicServer {
    /// Bind and listen on the given address with auto-detected region.
    pub async fn bind(addr: SocketAddr) -> Result<Self> {
        let identity = NodeIdentity::generate().await?;

        let server_config = configure_server(
            &identity.cert_der,
            &identity.key_der,
        )?;

        let endpoint = Endpoint::server(server_config, addr)
            .with_context(|| format!("binding QUIC server on {}", addr))?;

        info!(node_id = %identity.node_id, addr = %addr, "QUIC server listening");

        Ok(Self { endpoint, identity })
    }

    /// Accept the next incoming connection.
    pub async fn accept(&self) -> Result<Connection> {
        let incoming = self
            .endpoint
            .accept()
            .await
            .ok_or_else(|| anyhow::anyhow!("endpoint closed"))?;

        let conn = incoming.await.context("accepting connection")?;
        info!(remote = %conn.remote_address(), "accepted connection");

        Ok(Connection::new(conn))
    }

    pub fn local_addr(&self) -> Result<SocketAddr> {
        self.endpoint
            .local_addr()
            .context("getting local address")
    }

    pub fn node_id(&self) -> NodeId {
        self.identity.node_id
    }
}

/// QUIC client: connects to a remote peer
pub struct QuicClient {
    pub endpoint: Endpoint,
    pub identity: NodeIdentity,
}

impl QuicClient {
    /// Create a new client bound to an ephemeral port with auto-detected region.
    pub async fn new() -> Result<Self> {
        let identity = NodeIdentity::generate().await?;

        let client_config = configure_client()?;

        let mut endpoint = Endpoint::client("0.0.0.0:0".parse().unwrap())
            .context("creating client endpoint")?;
        endpoint.set_default_client_config(client_config);

        Ok(Self { endpoint, identity })
    }

    /// Connect to a remote peer.
    pub async fn connect(&self, addr: SocketAddr) -> Result<Connection> {
        let conn = self
            .endpoint
            .connect(addr, "ldrive.local")
            .context("initiating connection")?
            .await
            .with_context(|| format!("connecting to {}", addr))?;

        info!(remote = %addr, "connected to peer");

        Ok(Connection::new(conn))
    }

    pub fn node_id(&self) -> NodeId {
        self.identity.node_id
    }
}

fn configure_server(
    cert: &CertificateDer<'static>,
    key: &PrivateKeyDer<'static>,
) -> Result<ServerConfig> {
    let mut server_crypto = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![cert.clone()], key.clone_key())
        .context("configuring TLS server")?;

    server_crypto.alpn_protocols = vec![b"ldrive/0.1".to_vec()];

    let mut server_config = ServerConfig::with_crypto(Arc::new(
        quinn::crypto::rustls::QuicServerConfig::try_from(server_crypto)?,
    ));

    // Tune transport config
    let transport = Arc::get_mut(&mut server_config.transport).unwrap();
    transport.max_concurrent_bidi_streams(64u32.into());
    transport.max_idle_timeout(Some(quinn::IdleTimeout::from(
        quinn::VarInt::from_u32(30_000), // 30s idle timeout
    )));

    Ok(server_config)
}

fn configure_client() -> Result<ClientConfig> {
    // Accept any self-signed cert (P2P trust model, identity verified via handshake)
    let mut client_crypto = rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(SkipServerVerification))
        .with_no_client_auth();

    client_crypto.alpn_protocols = vec![b"ldrive/0.1".to_vec()];

    let client_config = ClientConfig::new(Arc::new(
        quinn::crypto::rustls::QuicClientConfig::try_from(client_crypto)?,
    ));

    Ok(client_config)
}

/// Skip TLS server certificate verification (P2P self-signed certs).
/// Identity is verified at the application layer via handshake.
#[derive(Debug)]
struct SkipServerVerification;

impl rustls::client::danger::ServerCertVerifier for SkipServerVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::ED25519,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::RSA_PSS_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA512,
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::RSA_PKCS1_SHA384,
            rustls::SignatureScheme::RSA_PKCS1_SHA512,
        ]
    }
}
