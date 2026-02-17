use ldrive_common::NodeId;
use rcgen::{CertificateParams, KeyPair};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};

/// Node cryptographic identity: Ed25519 keypair + self-signed certificate
pub struct NodeIdentity {
    pub node_id: NodeId,
    pub cert_der: CertificateDer<'static>,
    pub key_der: PrivateKeyDer<'static>,
    pub region: String,
}

impl NodeIdentity {
    /// Generate a new random identity with auto-detected region.
    pub async fn generate() -> anyhow::Result<Self> {
        let key_pair = KeyPair::generate_for(&rcgen::PKCS_ED25519)?;
        let public_key_der = key_pair.public_key_der();
        let node_id = NodeId::from_public_key(public_key_der.as_ref());

        let mut params = CertificateParams::new(vec!["ldrive.local".to_string()])?;
        params.distinguished_name.push(
            rcgen::DnType::CommonName,
            rcgen::DnValue::Utf8String(format!("ldrive-{}", node_id)),
        );

        let cert = params.self_signed(&key_pair)?;
        let cert_der = CertificateDer::from(cert.der().to_vec());
        let key_der = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key_pair.serialize_der()));

        let region = crate::region::detect_region().await.unwrap_or_else(|_| "default".to_string());

        Ok(Self {
            node_id,
            cert_der,
            key_der,
            region,
        })
    }
}

impl std::fmt::Debug for NodeIdentity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeIdentity")
            .field("node_id", &self.node_id)
            .finish()
    }
}
