use prost::Message;

/// Top-level P2P message envelope
#[derive(Clone, PartialEq, Message)]
pub struct PeerMessage {
    #[prost(oneof = "peer_message::Msg", tags = "1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21")]
    pub msg: Option<peer_message::Msg>,
}

pub mod peer_message {

    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Msg {
        #[prost(message, tag = "1")]
        Handshake(super::Handshake),
        #[prost(message, tag = "2")]
        Ping(super::Ping),
        #[prost(message, tag = "3")]
        Pong(super::Pong),
        #[prost(message, tag = "4")]
        ChunkPush(super::ChunkPush),
        #[prost(message, tag = "5")]
        ChunkPushAck(super::ChunkPushAck),
        #[prost(message, tag = "6")]
        ChunkRequest(super::ChunkRequest),
        #[prost(message, tag = "7")]
        ChunkResponse(super::ChunkResponse),
        #[prost(message, tag = "8")]
        ManifestPush(super::ManifestPush),
        #[prost(message, tag = "9")]
        ManifestPushAck(super::ManifestPushAck),
        #[prost(message, tag = "10")]
        ManifestRequest(super::ManifestRequest),
        #[prost(message, tag = "11")]
        ManifestResponse(super::ManifestResponse),
        #[prost(message, tag = "12")]
        TransferComplete(super::TransferComplete),
        #[prost(message, tag = "13")]
        TransferCompleteAck(super::TransferCompleteAck),
        // DHT messages
        #[prost(message, tag = "14")]
        FindNode(super::FindNode),
        #[prost(message, tag = "15")]
        FindNodeResponse(super::FindNodeResponse),
        #[prost(message, tag = "16")]
        FindValue(super::FindValue),
        #[prost(message, tag = "17")]
        FindValueResponse(super::FindValueResponse),
        #[prost(message, tag = "18")]
        StoreAnnounce(super::StoreAnnounce),
        #[prost(message, tag = "19")]
        StoreAnnounceAck(super::StoreAnnounceAck),
        #[prost(message, tag = "20")]
        Challenge(super::Challenge),
        #[prost(message, tag = "21")]
        ChallengeResponse(super::ChallengeResponse),
    }
}

#[derive(Clone, PartialEq, Message)]
pub struct Handshake {
    #[prost(bytes = "vec", tag = "1")]
    pub node_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "2")]
    pub public_key: Vec<u8>,
    #[prost(string, tag = "3")]
    pub version: String,
}

#[derive(Clone, PartialEq, Message)]
pub struct Ping {
    #[prost(uint64, tag = "1")]
    pub nonce: u64,
}

#[derive(Clone, PartialEq, Message)]
pub struct Pong {
    #[prost(uint64, tag = "1")]
    pub nonce: u64,
}

#[derive(Clone, PartialEq, Message)]
pub struct ChunkPush {
    #[prost(bytes = "vec", tag = "1")]
    pub hash: Vec<u8>,
    #[prost(bytes = "vec", tag = "2")]
    pub data: Vec<u8>,
    #[prost(uint32, tag = "3")]
    pub size: u32,
}

#[derive(Clone, PartialEq, Message)]
pub struct ChunkPushAck {
    #[prost(bytes = "vec", tag = "1")]
    pub hash: Vec<u8>,
    #[prost(bool, tag = "2")]
    pub success: bool,
    #[prost(string, tag = "3")]
    pub error: String,
}

#[derive(Clone, PartialEq, Message)]
pub struct ChunkRequest {
    #[prost(bytes = "vec", tag = "1")]
    pub hash: Vec<u8>,
}

#[derive(Clone, PartialEq, Message)]
pub struct ChunkResponse {
    #[prost(bytes = "vec", tag = "1")]
    pub hash: Vec<u8>,
    #[prost(bytes = "vec", tag = "2")]
    pub data: Vec<u8>,
    #[prost(bool, tag = "3")]
    pub found: bool,
    #[prost(string, tag = "4")]
    pub error: String,
}

#[derive(Clone, PartialEq, Message)]
pub struct ManifestPush {
    #[prost(bytes = "vec", tag = "1")]
    pub file_hash: Vec<u8>,
    #[prost(string, tag = "2")]
    pub file_name: String,
    #[prost(uint64, tag = "3")]
    pub file_size: u64,
    #[prost(message, repeated, tag = "4")]
    pub chunks: Vec<ChunkInfo>,
    /// Reed-Solomon data shards per group (0 = no erasure coding)
    #[prost(uint32, tag = "5")]
    pub erasure_data_shards: u32,
    /// Reed-Solomon parity shards per group
    #[prost(uint32, tag = "6")]
    pub erasure_parity_shards: u32,
}

#[derive(Clone, PartialEq, Message)]
pub struct ManifestPushAck {
    #[prost(bytes = "vec", tag = "1")]
    pub file_hash: Vec<u8>,
    #[prost(bool, tag = "2")]
    pub success: bool,
    #[prost(string, tag = "3")]
    pub error: String,
}

#[derive(Clone, PartialEq, Message)]
pub struct ManifestRequest {
    #[prost(bytes = "vec", tag = "1")]
    pub file_hash: Vec<u8>,
}

#[derive(Clone, PartialEq, Message)]
pub struct ManifestResponse {
    #[prost(bytes = "vec", tag = "1")]
    pub file_hash: Vec<u8>,
    #[prost(string, tag = "2")]
    pub file_name: String,
    #[prost(uint64, tag = "3")]
    pub file_size: u64,
    #[prost(message, repeated, tag = "4")]
    pub chunks: Vec<ChunkInfo>,
    #[prost(bool, tag = "5")]
    pub found: bool,
    #[prost(string, tag = "6")]
    pub error: String,
    #[prost(uint32, tag = "7")]
    pub erasure_data_shards: u32,
    #[prost(uint32, tag = "8")]
    pub erasure_parity_shards: u32,
}

#[derive(Clone, PartialEq, Message)]
pub struct ChunkInfo {
    #[prost(bytes = "vec", tag = "1")]
    pub hash: Vec<u8>,
    #[prost(uint32, tag = "2")]
    pub size: u32,
    #[prost(uint32, tag = "3")]
    pub index: u32,
    /// Erasure group index
    #[prost(uint32, tag = "4")]
    pub group_index: u32,
    /// Shard index within erasure group
    #[prost(uint32, tag = "5")]
    pub shard_index: u32,
    /// True if this is a parity shard
    #[prost(bool, tag = "6")]
    pub is_parity: bool,
    /// Original data size before padding
    #[prost(uint32, tag = "7")]
    pub original_size: u32,
}

#[derive(Clone, PartialEq, Message)]
pub struct TransferComplete {
    #[prost(bytes = "vec", tag = "1")]
    pub file_hash: Vec<u8>,
    #[prost(uint32, tag = "2")]
    pub total_chunks: u32,
}

#[derive(Clone, PartialEq, Message)]
pub struct TransferCompleteAck {
    #[prost(bytes = "vec", tag = "1")]
    pub file_hash: Vec<u8>,
    #[prost(bool, tag = "2")]
    pub success: bool,
    #[prost(string, tag = "3")]
    pub error: String,
}

// ─── DHT Messages ───

/// Peer contact info for DHT responses
#[derive(Clone, PartialEq, Message)]
pub struct PeerInfo {
    #[prost(bytes = "vec", tag = "1")]
    pub node_id: Vec<u8>,
    /// Serialized socket address "ip:port"
    #[prost(string, tag = "2")]
    pub addr: String,
    /// Region identifier (e.g., "cn-east", "us-west")
    #[prost(string, tag = "3")]
    pub region: String,
}

/// Find the K closest nodes to a target ID
#[derive(Clone, PartialEq, Message)]
pub struct FindNode {
    #[prost(bytes = "vec", tag = "1")]
    pub sender_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "2")]
    pub target: Vec<u8>,
}

#[derive(Clone, PartialEq, Message)]
pub struct FindNodeResponse {
    #[prost(message, repeated, tag = "1")]
    pub peers: Vec<PeerInfo>,
}

/// Find peers holding a content key (chunk hash)
#[derive(Clone, PartialEq, Message)]
pub struct FindValue {
    #[prost(bytes = "vec", tag = "1")]
    pub sender_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "2")]
    pub key: Vec<u8>,
}

#[derive(Clone, PartialEq, Message)]
pub struct FindValueResponse {
    #[prost(bool, tag = "1")]
    pub found: bool,
    #[prost(message, repeated, tag = "2")]
    pub holders: Vec<PeerInfo>,
    #[prost(message, repeated, tag = "3")]
    pub closer_peers: Vec<PeerInfo>,
}

/// Announce that this node holds a content key
#[derive(Clone, PartialEq, Message)]
pub struct StoreAnnounce {
    #[prost(bytes = "vec", tag = "1")]
    pub sender_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "2")]
    pub key: Vec<u8>,
    #[prost(message, optional, tag = "3")]
    pub sender_info: Option<PeerInfo>,
}

#[derive(Clone, PartialEq, Message)]
pub struct StoreAnnounceAck {
    #[prost(bool, tag = "1")]
    pub success: bool,
}

// ─── Proof of Storage ───

#[derive(Clone, PartialEq, Message)]
pub struct Challenge {
    #[prost(bytes = "vec", tag = "1")]
    pub chunk_hash: Vec<u8>,
    #[prost(uint64, tag = "2")]
    pub offset: u64,
    #[prost(uint32, tag = "3")]
    pub length: u32,
    #[prost(bytes = "vec", tag = "4")]
    pub nonce: Vec<u8>,
}

#[derive(Clone, PartialEq, Message)]
pub struct ChallengeResponse {
    #[prost(bytes = "vec", tag = "1")]
    pub chunk_hash: Vec<u8>,
    #[prost(bytes = "vec", tag = "2")]
    pub proof: Vec<u8>,
}
