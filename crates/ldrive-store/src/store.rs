use anyhow::{Context, Result};
use ldrive_common::{ChunkHash, FileHash, Manifest};
use redb::{Database, ReadableTable, TableDefinition};
use std::fs;
use std::path::{Path, PathBuf};

// redb table: chunk_hash -> serialized ChunkRecord { size, refcount }
const CHUNK_INDEX: TableDefinition<&[u8], &[u8]> = TableDefinition::new("chunk_index");
// redb table: file_hash -> serialized Manifest
const MANIFESTS: TableDefinition<&[u8], &[u8]> = TableDefinition::new("manifests");

/// Lightweight record stored in the chunk index
#[derive(Debug)]
struct ChunkRecord {
    size: u32,
    refcount: u32,
}

impl ChunkRecord {
    fn to_bytes(&self) -> [u8; 8] {
        let mut buf = [0u8; 8];
        buf[..4].copy_from_slice(&self.size.to_le_bytes());
        buf[4..].copy_from_slice(&self.refcount.to_le_bytes());
        buf
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 8 {
            return None;
        }
        Some(Self {
            size: u32::from_le_bytes(bytes[..4].try_into().ok()?),
            refcount: u32::from_le_bytes(bytes[4..8].try_into().ok()?),
        })
    }
}

/// Local chunk store: redb index + filesystem chunk data
pub struct ChunkStore {
    db: Database,
    chunks_dir: PathBuf,
    quota_bytes: u64,
}

impl ChunkStore {
    /// Open or create a chunk store at the given path.
    pub fn open(base_path: &Path, quota_bytes: u64) -> Result<Self> {
        let db_path = base_path.join("index.redb");
        let chunks_dir = base_path.join("chunks");
        fs::create_dir_all(&chunks_dir)?;

        let db = Database::create(&db_path)
            .with_context(|| format!("opening redb at {}", db_path.display()))?;

        // Ensure tables exist
        let txn = db.begin_write()?;
        {
            let _ = txn.open_table(CHUNK_INDEX)?;
            let _ = txn.open_table(MANIFESTS)?;
        }
        txn.commit()?;

        Ok(Self {
            db,
            chunks_dir,
            quota_bytes,
        })
    }

    /// Store a chunk. Returns true if newly stored, false if already existed.
    pub fn put_chunk(&self, hash: &ChunkHash, data: &[u8]) -> Result<bool> {
        // Check if already stored
        if self.has_chunk(hash)? {
            // Increment refcount: read then write
            let txn = self.db.begin_write()?;
            {
                let mut table = txn.open_table(CHUNK_INDEX)?;
                let record = {
                    let existing = table.get(hash.as_bytes().as_slice())?;
                    existing.and_then(|e| ChunkRecord::from_bytes(e.value()))
                };
                if let Some(mut record) = record {
                    record.refcount += 1;
                    table.insert(hash.as_bytes().as_slice(), record.to_bytes().as_slice())?;
                }
            }
            txn.commit()?;
            return Ok(false);
        }

        // Write data to filesystem
        let chunk_path = self.chunk_path(hash);
        if let Some(parent) = chunk_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&chunk_path, data)
            .with_context(|| format!("writing chunk to {}", chunk_path.display()))?;

        // Write index entry
        let record = ChunkRecord {
            size: data.len() as u32,
            refcount: 1,
        };
        let txn = self.db.begin_write()?;
        {
            let mut table = txn.open_table(CHUNK_INDEX)?;
            table.insert(hash.as_bytes().as_slice(), record.to_bytes().as_slice())?;
        }
        txn.commit()?;

        Ok(true)
    }

    /// Retrieve chunk data by hash.
    pub fn get_chunk(&self, hash: &ChunkHash) -> Result<Option<Vec<u8>>> {
        if !self.has_chunk(hash)? {
            return Ok(None);
        }
        let chunk_path = self.chunk_path(hash);
        match fs::read(&chunk_path) {
            Ok(data) => Ok(Some(data)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Check if a chunk exists in the index.
    pub fn has_chunk(&self, hash: &ChunkHash) -> Result<bool> {
        let txn = self.db.begin_read()?;
        let table = txn.open_table(CHUNK_INDEX)?;
        Ok(table.get(hash.as_bytes().as_slice())?.is_some())
    }

    /// Store a file manifest.
    pub fn put_manifest(&self, manifest: &Manifest) -> Result<()> {
        let data = manifest.to_bytes();
        let txn = self.db.begin_write()?;
        {
            let mut table = txn.open_table(MANIFESTS)?;
            table.insert(manifest.file_hash.as_bytes().as_slice(), data.as_slice())?;
        }
        txn.commit()?;
        Ok(())
    }

    /// Retrieve a manifest by file hash.
    pub fn get_manifest(&self, file_hash: &FileHash) -> Result<Option<Manifest>> {
        let txn = self.db.begin_read()?;
        let table = txn.open_table(MANIFESTS)?;
        match table.get(file_hash.as_bytes().as_slice())? {
            Some(data) => Ok(Manifest::from_bytes(data.value())),
            None => Ok(None),
        }
    }

    /// Calculate total bytes stored on disk.
    pub fn used_bytes(&self) -> Result<u64> {
        let mut total = 0u64;
        let txn = self.db.begin_read()?;
        let table = txn.open_table(CHUNK_INDEX)?;
        let iter = table.iter()?;
        for entry in iter {
            let entry = entry?;
            if let Some(record) = ChunkRecord::from_bytes(entry.1.value()) {
                total += record.size as u64;
            }
        }
        Ok(total)
    }

    /// Check if quota allows storing more data.
    pub fn quota_available(&self) -> Result<u64> {
        let used = self.used_bytes()?;
        Ok(self.quota_bytes.saturating_sub(used))
    }

    /// Filesystem path for a chunk, using 2-char prefix directory.
    fn chunk_path(&self, hash: &ChunkHash) -> PathBuf {
        let hex = hash.to_hex();
        self.chunks_dir.join(&hex[..2]).join(&hex)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn put_get_chunk() {
        let dir = TempDir::new().unwrap();
        let store = ChunkStore::open(dir.path(), 1_000_000).unwrap();

        let data = b"hello world chunk data";
        let hash = ChunkHash::compute(data);

        assert!(!store.has_chunk(&hash).unwrap());
        assert!(store.put_chunk(&hash, data).unwrap()); // newly stored
        assert!(!store.put_chunk(&hash, data).unwrap()); // already exists
        assert!(store.has_chunk(&hash).unwrap());

        let retrieved = store.get_chunk(&hash).unwrap().unwrap();
        assert_eq!(retrieved, data);
    }

    #[test]
    fn put_get_manifest() {
        let dir = TempDir::new().unwrap();
        let store = ChunkStore::open(dir.path(), 1_000_000).unwrap();

        let manifest = Manifest {
            file_hash: FileHash(*blake3::hash(b"test file").as_bytes()),
            file_name: "test.txt".to_string(),
            file_size: 1234,
            chunks: vec![],
            erasure: None,
        };

        store.put_manifest(&manifest).unwrap();
        let retrieved = store.get_manifest(&manifest.file_hash).unwrap().unwrap();
        assert_eq!(retrieved.file_name, "test.txt");
        assert_eq!(retrieved.file_size, 1234);
    }

    #[test]
    fn used_bytes_tracking() {
        let dir = TempDir::new().unwrap();
        let store = ChunkStore::open(dir.path(), 1_000_000).unwrap();

        assert_eq!(store.used_bytes().unwrap(), 0);

        let data = vec![0u8; 1000];
        let hash = ChunkHash::compute(&data);
        store.put_chunk(&hash, &data).unwrap();

        assert_eq!(store.used_bytes().unwrap(), 1000);
    }
}
