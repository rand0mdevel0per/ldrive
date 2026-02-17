use ldrive_common::NodeId;
use std::fmt;

/// 160-bit key used in the Kademlia DHT (same size as NodeId).
/// Used for both node IDs and content keys.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Key(pub [u8; 20]);

impl Key {
    pub const ZERO: Key = Key([0u8; 20]);
    pub const BITS: usize = 160;

    pub fn from_node_id(id: &NodeId) -> Self {
        Self(id.0)
    }

    /// Create a key from a 32-byte content hash (truncate to 20 bytes).
    pub fn from_content_hash(hash: &[u8; 32]) -> Self {
        let mut k = [0u8; 20];
        k.copy_from_slice(&hash[..20]);
        Self(k)
    }

    /// XOR distance between two keys.
    pub fn distance(&self, other: &Key) -> Key {
        let mut d = [0u8; 20];
        for i in 0..20 {
            d[i] = self.0[i] ^ other.0[i];
        }
        Key(d)
    }

    /// Return the index of the most significant bit (0-159), or None if zero.
    /// This determines which K-Bucket a node belongs to.
    pub fn leading_zeros(&self) -> usize {
        for i in 0..20 {
            if self.0[i] != 0 {
                return i * 8 + self.0[i].leading_zeros() as usize;
            }
        }
        160
    }

    /// Bucket index for a peer relative to our own key.
    /// Returns 0..159 (bucket 0 = farthest, bucket 159 = closest).
    pub fn bucket_index(&self, other: &Key) -> usize {
        let dist = self.distance(other);
        let lz = dist.leading_zeros();
        if lz >= Self::BITS {
            Self::BITS - 1 // same key, put in closest bucket
        } else {
            Self::BITS - 1 - lz
        }
    }

    pub fn as_bytes(&self) -> &[u8; 20] {
        &self.0
    }
}

impl fmt::Debug for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Key({}..)", &hex::encode(self.0)[..8])
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

impl PartialOrd for Key {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Key {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xor_distance_self_is_zero() {
        let k = Key([0xAB; 20]);
        assert_eq!(k.distance(&k), Key::ZERO);
    }

    #[test]
    fn bucket_index_computation() {
        let a = Key([0; 20]);
        let mut b = [0u8; 20];
        b[19] = 1; // distance = 1, leading zeros = 159
        let b = Key(b);
        assert_eq!(a.bucket_index(&b), 0); // closest non-self bucket

        let mut c = [0u8; 20];
        c[0] = 0x80; // distance has MSB set, leading zeros = 0
        let c = Key(c);
        assert_eq!(a.bucket_index(&c), 159); // farthest bucket
    }

    #[test]
    fn distance_symmetry() {
        let a = Key([0x12; 20]);
        let b = Key([0x34; 20]);
        assert_eq!(a.distance(&b), b.distance(&a));
    }
}
