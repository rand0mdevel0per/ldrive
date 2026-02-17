use anyhow::Result;
use chacha20::cipher::{KeyIvInit, StreamCipher};
use chacha20::ChaCha20;
use fips203::{ml_kem_768, traits::{KeyGen as KemKeyGen, SerDes as KemSerDes}};
use fips204::{ml_dsa_65, traits::{KeyGen as DsaKeyGen, SerDes as DsaSerDes}};

pub struct PqcKeypair {
    pub kem_pk: Vec<u8>,
    pub kem_sk: Vec<u8>,
    pub sig_pk: Vec<u8>,
    pub sig_sk: Vec<u8>,
}

pub fn generate_keypair() -> Result<PqcKeypair> {
    let mut rng = rand::thread_rng();

    let (kem_pk, kem_sk) = ml_kem_768::KG::try_keygen_with_rng(&mut rng)
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    let (sig_pk, sig_sk) = ml_dsa_65::KG::try_keygen_with_rng(&mut rng)
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    Ok(PqcKeypair {
        kem_pk: kem_pk.into_bytes().to_vec(),
        kem_sk: kem_sk.into_bytes().to_vec(),
        sig_pk: sig_pk.into_bytes().to_vec(),
        sig_sk: sig_sk.into_bytes().to_vec(),
    })
}

pub fn encrypt(data: &[u8], key: &[u8; 32], nonce: &[u8; 12]) -> Vec<u8> {
    let mut cipher = ChaCha20::new(key.into(), nonce.into());
    let mut buffer = data.to_vec();
    cipher.apply_keystream(&mut buffer);
    buffer
}

pub fn decrypt(data: &[u8], key: &[u8; 32], nonce: &[u8; 12]) -> Vec<u8> {
    encrypt(data, key, nonce)
}
