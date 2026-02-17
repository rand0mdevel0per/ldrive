use anyhow::{Context, Result};
use ldrive_common::{ChunkHash, ChunkMeta, ErasureConfig, ShardType};
use reed_solomon_erasure::galois_8::ReedSolomon;

/// Default erasure coding: 4 data shards + 2 parity shards
pub const DATA_SHARDS: usize = 4;
pub const PARITY_SHARDS: usize = 2;
pub const TOTAL_SHARDS: usize = DATA_SHARDS + PARITY_SHARDS;

/// A shard produced by erasure encoding
#[derive(Debug)]
pub struct ErasureShard {
    pub hash: ChunkHash,
    pub data: Vec<u8>,
    pub group_index: u32,
    pub shard_index: u32,
    pub shard_type: ShardType,
    /// Size of actual data before padding (for data shards)
    pub original_size: u32,
}

/// Encode a group of data chunks into data + parity shards using Reed-Solomon.
///
/// `data_chunks` can have 1..=DATA_SHARDS items. If fewer than DATA_SHARDS,
/// the remaining data slots are filled with empty shards.
/// All shards within a group are padded to the same length (max chunk size in group).
pub fn encode_group(
    data_chunks: &[(ChunkHash, Vec<u8>)],
    group_index: u32,
    global_index_offset: u32,
) -> Result<Vec<ErasureShard>> {
    assert!(!data_chunks.is_empty() && data_chunks.len() <= DATA_SHARDS);

    let rs = ReedSolomon::new(DATA_SHARDS, PARITY_SHARDS)
        .context("creating Reed-Solomon encoder")?;

    // Find max shard size for padding
    let max_size = data_chunks.iter().map(|(_, d)| d.len()).max().unwrap_or(0);
    // Ensure at least 1 byte (RS requires non-empty shards)
    let shard_size = max_size.max(1);

    // Build shard buffers: DATA_SHARDS data + PARITY_SHARDS parity
    let mut shards: Vec<Vec<u8>> = Vec::with_capacity(TOTAL_SHARDS);

    // Data shards (pad to shard_size, fill missing slots with zeros)
    for i in 0..DATA_SHARDS {
        if i < data_chunks.len() {
            let mut buf = data_chunks[i].1.clone();
            buf.resize(shard_size, 0);
            shards.push(buf);
        } else {
            shards.push(vec![0u8; shard_size]);
        }
    }

    // Parity shards (initialized to zeros, RS will fill them)
    for _ in 0..PARITY_SHARDS {
        shards.push(vec![0u8; shard_size]);
    }

    // Encode: fills parity shards in-place
    rs.encode(&mut shards).context("Reed-Solomon encode")?;

    // Build output
    let mut result = Vec::with_capacity(TOTAL_SHARDS);
    let mut global_idx = global_index_offset;

    for (i, shard_data) in shards.into_iter().enumerate() {
        let is_data = i < DATA_SHARDS;
        let original_size = if is_data && i < data_chunks.len() {
            data_chunks[i].1.len() as u32
        } else if is_data {
            0 // padding slot
        } else {
            shard_data.len() as u32 // parity: full shard
        };

        let hash = ChunkHash::compute(&shard_data);

        result.push(ErasureShard {
            hash,
            data: shard_data,
            group_index,
            shard_index: i as u32,
            shard_type: if is_data { ShardType::Data } else { ShardType::Parity },
            original_size,
        });

        global_idx += 1;
    }

    let _ = global_idx; // suppress unused warning

    Ok(result)
}

/// Decode/reconstruct missing shards in a group.
///
/// `shards` has TOTAL_SHARDS slots. Each slot is either Some(data) or None (missing).
/// RS can recover the data as long as at least DATA_SHARDS slots are present.
/// Returns the reconstructed data shards with padding removed.
pub fn decode_group(
    shards: &mut [Option<Vec<u8>>],
    original_sizes: &[u32],
    actual_data_count: usize,
) -> Result<Vec<Vec<u8>>> {
    assert_eq!(shards.len(), TOTAL_SHARDS);

    let rs = ReedSolomon::new(DATA_SHARDS, PARITY_SHARDS)
        .context("creating Reed-Solomon decoder")?;

    // Reconstruct missing shards
    rs.reconstruct(shards).context("Reed-Solomon reconstruct")?;

    // Extract data shards and remove padding
    let mut data_shards = Vec::with_capacity(actual_data_count);
    for i in 0..actual_data_count {
        let shard = shards[i].as_ref()
            .ok_or_else(|| anyhow::anyhow!("shard {} still missing after reconstruct", i))?;
        let orig_size = original_sizes[i] as usize;
        data_shards.push(shard[..orig_size].to_vec());
    }

    Ok(data_shards)
}

/// Convert ChunkMetas from an erasure group into the shard array format needed by decode_group.
///
/// `available_shards` maps ChunkHash -> data for shards we actually have.
/// Returns (shard_slots, original_sizes, actual_data_count).
pub fn prepare_group_for_decode(
    group_metas: &[&ChunkMeta],
    available_shards: &std::collections::HashMap<ChunkHash, Vec<u8>>,
) -> (Vec<Option<Vec<u8>>>, Vec<u32>, usize) {
    let mut slots: Vec<Option<Vec<u8>>> = vec![None; TOTAL_SHARDS];
    let mut original_sizes = vec![0u32; DATA_SHARDS];
    let mut actual_data_count = 0usize;

    for meta in group_metas {
        let idx = meta.shard_index as usize;
        if idx < TOTAL_SHARDS {
            if let Some(data) = available_shards.get(&meta.hash) {
                slots[idx] = Some(data.clone());
            }
            if meta.shard_type == ShardType::Data && idx < DATA_SHARDS {
                original_sizes[idx] = meta.original_size;
                if meta.original_size > 0 {
                    actual_data_count = actual_data_count.max(idx + 1);
                }
            }
        }
    }

    (slots, original_sizes, actual_data_count)
}

impl ErasureShard {
    /// Convert to ChunkMeta for the manifest
    pub fn to_chunk_meta(&self, global_index: u32) -> ChunkMeta {
        ChunkMeta {
            hash: self.hash,
            size: self.data.len() as u32,
            index: global_index,
            group_index: self.group_index,
            shard_index: self.shard_index,
            shard_type: self.shard_type,
            original_size: self.original_size,
        }
    }
}

/// Get the default erasure config
pub fn default_erasure_config() -> ErasureConfig {
    ErasureConfig::rs_4_2()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_decode_roundtrip() {
        let chunks: Vec<(ChunkHash, Vec<u8>)> = (0..4)
            .map(|i| {
                let data = vec![i as u8; 100 + i * 10];
                let hash = ChunkHash::compute(&data);
                (hash, data)
            })
            .collect();

        let original_data: Vec<Vec<u8>> = chunks.iter().map(|(_, d)| d.clone()).collect();

        let shards = encode_group(&chunks, 0, 0).unwrap();
        assert_eq!(shards.len(), TOTAL_SHARDS);

        // All data should be recoverable with all shards present
        let mut shard_slots: Vec<Option<Vec<u8>>> = shards.iter()
            .map(|s| Some(s.data.clone()))
            .collect();
        let original_sizes: Vec<u32> = shards.iter()
            .take(DATA_SHARDS)
            .map(|s| s.original_size)
            .collect();

        let recovered = decode_group(&mut shard_slots, &original_sizes, 4).unwrap();
        assert_eq!(recovered, original_data);
    }

    #[test]
    fn recover_with_missing_shards() {
        let chunks: Vec<(ChunkHash, Vec<u8>)> = (0..4)
            .map(|i| {
                let data = vec![(i + 1) as u8; 200 + i * 25];
                let hash = ChunkHash::compute(&data);
                (hash, data)
            })
            .collect();

        let original_data: Vec<Vec<u8>> = chunks.iter().map(|(_, d)| d.clone()).collect();

        let shards = encode_group(&chunks, 0, 0).unwrap();

        // Remove 2 shards (data shard 1 and data shard 3) — should still recover
        let mut shard_slots: Vec<Option<Vec<u8>>> = shards.iter()
            .map(|s| Some(s.data.clone()))
            .collect();
        shard_slots[1] = None;
        shard_slots[3] = None;

        let original_sizes: Vec<u32> = shards.iter()
            .take(DATA_SHARDS)
            .map(|s| s.original_size)
            .collect();

        let recovered = decode_group(&mut shard_slots, &original_sizes, 4).unwrap();
        assert_eq!(recovered, original_data);
    }

    #[test]
    fn partial_group() {
        // Only 2 data chunks in the last group
        let chunks: Vec<(ChunkHash, Vec<u8>)> = (0..2)
            .map(|i| {
                let data = vec![(i + 10) as u8; 50 + i * 20];
                let hash = ChunkHash::compute(&data);
                (hash, data)
            })
            .collect();

        let original_data: Vec<Vec<u8>> = chunks.iter().map(|(_, d)| d.clone()).collect();

        let shards = encode_group(&chunks, 0, 0).unwrap();
        assert_eq!(shards.len(), TOTAL_SHARDS);

        // Remove one data shard — should still recover
        let mut shard_slots: Vec<Option<Vec<u8>>> = shards.iter()
            .map(|s| Some(s.data.clone()))
            .collect();
        shard_slots[0] = None;

        let original_sizes: Vec<u32> = shards.iter()
            .take(DATA_SHARDS)
            .map(|s| s.original_size)
            .collect();

        let recovered = decode_group(&mut shard_slots, &original_sizes, 2).unwrap();
        assert_eq!(recovered, original_data);
    }
}
