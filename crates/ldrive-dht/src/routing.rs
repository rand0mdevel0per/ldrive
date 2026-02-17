use crate::key::Key;
use std::collections::VecDeque;
use std::net::SocketAddr;
use std::time::Instant;

/// Kademlia parameters
pub const K: usize = 20;
pub const NUM_BUCKETS: usize = 160;

/// A single entry in the routing table
#[derive(Debug, Clone)]
pub struct RoutingEntry {
    pub key: Key,
    pub addr: SocketAddr,
    pub region: String,
    pub last_seen: Instant,
}

impl RoutingEntry {
    pub fn new(key: Key, addr: SocketAddr, region: String) -> Self {
        Self {
            key,
            addr,
            region,
            last_seen: Instant::now(),
        }
    }

    pub fn touch(&mut self) {
        self.last_seen = Instant::now();
    }
}

/// K-Bucket: holds up to K entries, ordered by last-seen (front = least recent)
struct KBucket {
    entries: VecDeque<RoutingEntry>,
}

impl KBucket {
    fn new() -> Self {
        Self {
            entries: VecDeque::with_capacity(K),
        }
    }

    /// Insert or update an entry. Returns the eviction candidate if bucket is full.
    fn upsert(&mut self, entry: RoutingEntry) -> UpsertResult {
        // If already present, move to back (most recently seen)
        if let Some(pos) = self.entries.iter().position(|e| e.key == entry.key) {
            self.entries[pos].addr = entry.addr;
            self.entries[pos].touch();
            let e = self.entries.remove(pos).unwrap();
            self.entries.push_back(e);
            return UpsertResult::Updated;
        }

        // If bucket not full, add to back
        if self.entries.len() < K {
            self.entries.push_back(entry);
            return UpsertResult::Inserted;
        }

        // Bucket is full - return the least recently seen for eviction check
        UpsertResult::BucketFull {
            eviction_candidate: self.entries.front().cloned().unwrap(),
            pending: entry,
        }
    }

    /// Remove the least recently seen entry (after failed ping)
    fn evict_front(&mut self) -> Option<RoutingEntry> {
        self.entries.pop_front()
    }

    /// Get all entries
    fn entries(&self) -> &VecDeque<RoutingEntry> {
        &self.entries
    }

    fn len(&self) -> usize {
        self.entries.len()
    }

    fn remove(&mut self, key: &Key) -> bool {
        if let Some(pos) = self.entries.iter().position(|e| &e.key == key) {
            self.entries.remove(pos);
            true
        } else {
            false
        }
    }
}

enum UpsertResult {
    Inserted,
    Updated,
    BucketFull {
        eviction_candidate: RoutingEntry,
        pending: RoutingEntry,
    },
}

/// Kademlia routing table: 160 K-Buckets indexed by XOR distance
pub struct RoutingTable {
    local_key: Key,
    buckets: Vec<KBucket>,
}

impl RoutingTable {
    pub fn new(local_key: Key) -> Self {
        let mut buckets = Vec::with_capacity(NUM_BUCKETS);
        for _ in 0..NUM_BUCKETS {
            buckets.push(KBucket::new());
        }
        Self { local_key, buckets }
    }

    pub fn local_key(&self) -> &Key {
        &self.local_key
    }

    /// Insert or update a peer. Returns true if the peer was added/updated,
    /// false if the bucket was full (caller should ping the eviction candidate).
    pub fn upsert(&mut self, entry: RoutingEntry) -> InsertOutcome {
        if entry.key == self.local_key {
            return InsertOutcome::SelfIgnored;
        }

        let idx = self.local_key.bucket_index(&entry.key);
        match self.buckets[idx].upsert(entry) {
            UpsertResult::Inserted => InsertOutcome::Inserted,
            UpsertResult::Updated => InsertOutcome::Updated,
            UpsertResult::BucketFull {
                eviction_candidate,
                pending,
            } => InsertOutcome::Full {
                eviction_candidate,
                pending,
            },
        }
    }

    /// Evict a peer from its bucket (after it failed to respond to ping)
    /// and insert the pending entry.
    pub fn evict_and_insert(&mut self, evict_key: &Key, pending: RoutingEntry) -> bool {
        let idx = self.local_key.bucket_index(evict_key);
        if self.buckets[idx].remove(evict_key) {
            self.buckets[idx].entries.push_back(pending);
            true
        } else {
            false
        }
    }

    /// Find the K closest entries to a target key.
    pub fn closest(&self, target: &Key, count: usize) -> Vec<RoutingEntry> {
        let mut all: Vec<(Key, RoutingEntry)> = Vec::new();

        for bucket in &self.buckets {
            for entry in bucket.entries() {
                let dist = target.distance(&entry.key);
                all.push((dist, entry.clone()));
            }
        }

        all.sort_by(|a, b| a.0.cmp(&b.0));
        all.into_iter().take(count).map(|(_, e)| e).collect()
    }

    /// Total number of peers in the routing table.
    pub fn len(&self) -> usize {
        self.buckets.iter().map(|b| b.len()).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get all peers in the table.
    pub fn all_peers(&self) -> Vec<RoutingEntry> {
        self.buckets
            .iter()
            .flat_map(|b| b.entries().iter().cloned())
            .collect()
    }
}

#[derive(Debug)]
pub enum InsertOutcome {
    Inserted,
    Updated,
    SelfIgnored,
    Full {
        eviction_candidate: RoutingEntry,
        pending: RoutingEntry,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_key(byte: u8) -> Key {
        Key([byte; 20])
    }

    fn make_entry(byte: u8) -> RoutingEntry {
        RoutingEntry::new(
            make_key(byte),
            format!("127.0.0.1:{}", 4000 + byte as u16).parse().unwrap(),
        )
    }

    #[test]
    fn insert_and_closest() {
        let local = make_key(0);
        let mut table = RoutingTable::new(local);

        for i in 1..=10u8 {
            table.upsert(make_entry(i));
        }

        assert_eq!(table.len(), 10);

        let closest = table.closest(&make_key(1), 3);
        assert!(!closest.is_empty());
        assert!(closest.len() <= 3);
        // First result should be key=1 (exact match)
        assert_eq!(closest[0].key, make_key(1));
    }

    #[test]
    fn ignore_self() {
        let local = make_key(42);
        let mut table = RoutingTable::new(local);
        let result = table.upsert(make_entry(42));
        assert!(matches!(result, InsertOutcome::SelfIgnored));
        assert_eq!(table.len(), 0);
    }

    #[test]
    fn update_existing() {
        let local = make_key(0);
        let mut table = RoutingTable::new(local);

        table.upsert(make_entry(1));
        assert_eq!(table.len(), 1);

        table.upsert(make_entry(1));
        assert_eq!(table.len(), 1);
    }
}
