use thiserror::Error;

#[derive(Debug, Error)]
pub enum LdriveError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("chunk not found: {hash}")]
    ChunkNotFound { hash: String },

    #[error("manifest not found: {hash}")]
    ManifestNotFound { hash: String },

    #[error("hash mismatch: expected {expected}, got {actual}")]
    HashMismatch { expected: String, actual: String },

    #[error("quota exceeded: used {used} bytes, limit {limit} bytes")]
    QuotaExceeded { used: u64, limit: u64 },

    #[error("storage error: {0}")]
    Storage(String),

    #[error("network error: {0}")]
    Network(String),

    #[error("protocol error: {0}")]
    Protocol(String),
}
