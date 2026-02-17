use crate::types::{Challenge, ChallengeResponse};
use ldrive_common::ChunkHash;
use anyhow::Result;

pub fn generate_challenge(chunk_hash: ChunkHash, chunk_size: u64) -> Challenge {
    let mut nonce = [0u8; 32];
    rand::Rng::fill(&mut rand::thread_rng(), &mut nonce);

    let offset = if chunk_size > 1024 {
        rand::random::<u64>() % (chunk_size - 1024)
    } else {
        0
    };

    let length = 256.min(chunk_size.saturating_sub(offset) as u32);

    Challenge {
        chunk_hash,
        offset,
        length,
        nonce,
    }
}

pub fn verify_response(
    challenge: &Challenge,
    response: &ChallengeResponse,
    chunk_data: &[u8],
) -> Result<bool> {
    if response.chunk_hash != challenge.chunk_hash {
        return Ok(false);
    }

    let start = challenge.offset as usize;
    let end = start + challenge.length as usize;

    if end > chunk_data.len() {
        return Ok(false);
    }

    let mut hasher = blake3::Hasher::new();
    hasher.update(&challenge.nonce);
    hasher.update(&chunk_data[start..end]);
    let expected = hasher.finalize();

    Ok(response.proof == *expected.as_bytes())
}
