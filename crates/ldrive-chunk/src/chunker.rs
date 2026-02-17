use anyhow::{Context, Result};
use fastcdc::v2020::FastCDC;
use ldrive_common::{ChunkHash, ChunkMeta, ErasureConfig, FileHash, Manifest, ShardType};
use std::fs;
use std::io::Write;
use std::path::Path;

use crate::erasure;

/// Minimum chunk size (16 KB)
const MIN_CHUNK: u32 = 16 * 1024;
/// Average chunk size (64 KB)
const AVG_CHUNK: u32 = 64 * 1024;
/// Maximum chunk size (256 KB)
const MAX_CHUNK: u32 = 256 * 1024;

/// A single chunk/shard produced by the chunker
#[derive(Debug)]
pub struct ChunkedPiece {
    pub hash: ChunkHash,
    pub data: Vec<u8>,
    pub index: u32,
    pub group_index: u32,
    pub shard_index: u32,
    pub shard_type: ShardType,
    pub original_size: u32,
}

/// Result of chunking a file
pub struct ChunkedFile {
    pub manifest: Manifest,
    pub pieces: Vec<ChunkedPiece>,
}

/// Chunk a file using FastCDC with BLAKE3 hashing, then apply Reed-Solomon 4+2 erasure coding.
///
/// Returns a manifest describing all shards (data + parity) plus the shard data.
pub fn chunk_file(path: &Path) -> Result<ChunkedFile> {
    let data = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    let file_hash = FileHash(*blake3::hash(&data).as_bytes());

    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();

    let chunker = FastCDC::new(&data, MIN_CHUNK, AVG_CHUNK, MAX_CHUNK);

    // First pass: collect raw data chunks
    let mut raw_chunks: Vec<(ChunkHash, Vec<u8>)> = Vec::new();
    for entry in chunker {
        let chunk_data = &data[entry.offset..entry.offset + entry.length];
        let hash = ChunkHash::compute(chunk_data);
        raw_chunks.push((hash, chunk_data.to_vec()));
    }

    if raw_chunks.is_empty() {
        // Empty file — no chunks, no erasure coding
        return Ok(ChunkedFile {
            manifest: Manifest {
                file_hash,
                file_name,
                file_size: 0,
                chunks: vec![],
                erasure: None,
            },
            pieces: vec![],
        });
    }

    // Group chunks into groups of DATA_SHARDS and apply erasure coding
    let mut all_pieces = Vec::new();
    let mut all_metas = Vec::new();
    let mut global_index = 0u32;

    for (group_idx, group) in raw_chunks.chunks(erasure::DATA_SHARDS).enumerate() {
        let group_chunks: Vec<(ChunkHash, Vec<u8>)> = group.to_vec();
        let shards = erasure::encode_group(&group_chunks, group_idx as u32, global_index)?;

        for shard in shards {
            let meta = shard.to_chunk_meta(global_index);
            all_metas.push(meta);

            all_pieces.push(ChunkedPiece {
                hash: shard.hash,
                data: shard.data,
                index: global_index,
                group_index: shard.group_index,
                shard_index: shard.shard_index,
                shard_type: shard.shard_type,
                original_size: shard.original_size,
            });

            global_index += 1;
        }
    }

    let manifest = Manifest {
        file_hash,
        file_name,
        file_size: data.len() as u64,
        chunks: all_metas,
        erasure: Some(ErasureConfig::rs_4_2()),
    };

    Ok(ChunkedFile { manifest, pieces: all_pieces })
}

/// Chunk a file without erasure coding (simpler mode for direct transfers).
pub fn chunk_file_simple(path: &Path) -> Result<ChunkedFile> {
    let data = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    let file_hash = FileHash(*blake3::hash(&data).as_bytes());

    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();

    let chunker = FastCDC::new(&data, MIN_CHUNK, AVG_CHUNK, MAX_CHUNK);

    let mut pieces = Vec::new();
    let mut chunk_metas = Vec::new();

    for (index, entry) in chunker.enumerate() {
        let chunk_data = &data[entry.offset..entry.offset + entry.length];
        let hash = ChunkHash::compute(chunk_data);
        let idx = index as u32;

        chunk_metas.push(ChunkMeta {
            hash,
            size: entry.length as u32,
            index: idx,
            group_index: 0,
            shard_index: 0,
            shard_type: ShardType::Data,
            original_size: entry.length as u32,
        });

        pieces.push(ChunkedPiece {
            hash,
            data: chunk_data.to_vec(),
            index: idx,
            group_index: 0,
            shard_index: 0,
            shard_type: ShardType::Data,
            original_size: entry.length as u32,
        });
    }

    let manifest = Manifest {
        file_hash,
        file_name,
        file_size: data.len() as u64,
        chunks: chunk_metas,
        erasure: None,
    };

    Ok(ChunkedFile { manifest, pieces })
}

/// Reassemble chunks into a file according to the manifest.
///
/// For manifests with erasure coding, this uses RS decoding to recover missing data shards.
/// `get_chunk` is called for each chunk hash to retrieve the data (returns None if missing).
pub fn reassemble<F>(manifest: &Manifest, output: &Path, mut get_chunk: F) -> Result<()>
where
    F: FnMut(&ChunkHash) -> Result<Vec<u8>>,
{
    let mut file = fs::File::create(output)
        .with_context(|| format!("creating output {}", output.display()))?;

    if manifest.erasure.is_some() {
        // Erasure-coded reassembly: process group by group
        reassemble_erasure(manifest, &mut file, &mut get_chunk)?;
    } else {
        // Simple reassembly: just concatenate data chunks in order
        let mut sorted_chunks = manifest.chunks.clone();
        sorted_chunks.sort_by_key(|c| c.index);

        for chunk_meta in &sorted_chunks {
            let data = get_chunk(&chunk_meta.hash)
                .with_context(|| format!("getting chunk {}", chunk_meta.hash))?;

            if !verify_chunk(&chunk_meta.hash, &data) {
                anyhow::bail!(
                    "chunk hash mismatch for {}: data integrity check failed",
                    chunk_meta.hash
                );
            }

            file.write_all(&data)
                .with_context(|| "writing chunk to output")?;
        }
    }

    file.flush()?;

    // Verify final file hash
    let written = fs::read(output)?;
    let actual_hash = FileHash(*blake3::hash(&written).as_bytes());
    if actual_hash != manifest.file_hash {
        anyhow::bail!(
            "file hash mismatch: expected {}, got {}",
            manifest.file_hash,
            actual_hash
        );
    }

    Ok(())
}

/// Reassemble with erasure coding: try to fetch all shards per group,
/// use RS decoding to recover any missing data shards.
fn reassemble_erasure<F>(
    manifest: &Manifest,
    file: &mut fs::File,
    get_chunk: &mut F,
) -> Result<()>
where
    F: FnMut(&ChunkHash) -> Result<Vec<u8>>,
{
    let group_count = manifest.group_count();

    for group_idx in 0..group_count {
        let group_metas = manifest.group_chunks(group_idx);

        // Try to fetch all available shards for this group
        let mut available: std::collections::HashMap<ChunkHash, Vec<u8>> =
            std::collections::HashMap::new();
        let mut fetch_count = 0;

        for meta in &group_metas {
            match get_chunk(&meta.hash) {
                Ok(data) => {
                    if verify_chunk(&meta.hash, &data) {
                        available.insert(meta.hash, data);
                        fetch_count += 1;
                    }
                }
                Err(_) => {} // Missing shard — RS will handle it
            }
        }

        if fetch_count < erasure::DATA_SHARDS {
            anyhow::bail!(
                "group {}: only {}/{} shards available, need at least {}",
                group_idx,
                fetch_count,
                erasure::TOTAL_SHARDS,
                erasure::DATA_SHARDS
            );
        }

        let (mut slots, original_sizes, actual_data_count) =
            erasure::prepare_group_for_decode(&group_metas, &available);

        let data_chunks = erasure::decode_group(&mut slots, &original_sizes, actual_data_count)?;

        for chunk_data in &data_chunks {
            file.write_all(chunk_data)
                .with_context(|| "writing chunk to output")?;
        }
    }

    Ok(())
}

/// Verify a chunk's data against its hash.
pub fn verify_chunk(hash: &ChunkHash, data: &[u8]) -> bool {
    hash.verify(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn roundtrip_with_erasure() {
        let dir = TempDir::new().unwrap();
        let input = dir.path().join("input.bin");
        let output = dir.path().join("output.bin");

        // Create a test file (~500 KB - will produce multiple erasure groups)
        let data: Vec<u8> = (0..500_000u32).map(|i| (i % 256) as u8).collect();
        fs::write(&input, &data).unwrap();

        // Chunk with erasure coding
        let chunked = chunk_file(&input).unwrap();
        assert!(!chunked.pieces.is_empty());
        assert_eq!(chunked.manifest.file_size, data.len() as u64);
        assert!(chunked.manifest.erasure.is_some());

        // Store all shards
        let shards: std::collections::HashMap<ChunkHash, Vec<u8>> = chunked
            .pieces
            .into_iter()
            .map(|p| (p.hash, p.data))
            .collect();

        // Reassemble (all shards available)
        reassemble(&chunked.manifest, &output, |hash| {
            shards
                .get(hash)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("chunk not found"))
        })
        .unwrap();

        let result = fs::read(&output).unwrap();
        assert_eq!(data, result);
    }

    #[test]
    fn roundtrip_simple_mode() {
        let dir = TempDir::new().unwrap();
        let input = dir.path().join("input.bin");
        let output = dir.path().join("output.bin");

        let data: Vec<u8> = (0..200_000u32).map(|i| (i % 256) as u8).collect();
        fs::write(&input, &data).unwrap();

        let chunked = chunk_file_simple(&input).unwrap();
        assert!(chunked.manifest.erasure.is_none());

        let chunks: std::collections::HashMap<ChunkHash, Vec<u8>> = chunked
            .pieces
            .into_iter()
            .map(|p| (p.hash, p.data))
            .collect();

        reassemble(&chunked.manifest, &output, |hash| {
            chunks
                .get(hash)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("chunk not found"))
        })
        .unwrap();

        let result = fs::read(&output).unwrap();
        assert_eq!(data, result);
    }

    #[test]
    fn verify_chunk_integrity() {
        let data = b"hello world";
        let hash = ChunkHash::compute(data);
        assert!(verify_chunk(&hash, data));
        assert!(!verify_chunk(&hash, b"hello worlD"));
    }

    #[test]
    fn empty_file() {
        let dir = TempDir::new().unwrap();
        let input = dir.path().join("empty.bin");
        let output = dir.path().join("output.bin");

        fs::write(&input, b"").unwrap();
        let chunked = chunk_file(&input).unwrap();
        assert_eq!(chunked.pieces.len(), 0);
        assert_eq!(chunked.manifest.file_size, 0);

        reassemble(&chunked.manifest, &output, |_| {
            unreachable!("no chunks expected")
        })
        .unwrap();

        let result = fs::read(&output).unwrap();
        assert!(result.is_empty());
    }
}
