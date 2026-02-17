use ldrive_common::ChunkHash;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Challenge {
    pub chunk_hash: ChunkHash,
    pub offset: u64,
    pub length: u32,
    pub nonce: [u8; 32],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChallengeResponse {
    pub chunk_hash: ChunkHash,
    pub proof: [u8; 32],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreditRecord {
    pub node_id: String,
    pub storage_gb: f64,
    pub uptime_hours: f64,
    pub challenge_pass_rate: f64,
    pub credits_earned: f64,
}
