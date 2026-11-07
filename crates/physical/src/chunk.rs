//! `<f64 little-endian seconds><u32 little-endian length><bytes>`.

#[derive(Clone, Debug, PartialEq)]
pub struct Chunk {
    pub t_s: f64,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChunkParse {
    pub chunks: Vec<Chunk>,
    pub error: Option<String>,
}

pub fn parse_chunks(bytes: &[u8]) -> ChunkParse {
    let mut chunks = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes.len() - index < 12 {
            return ChunkParse {
                chunks,
                error: Some(format!("truncated header at {index}")),
            };
        }
        let mut time_bytes = [0_u8; 8];
        time_bytes.copy_from_slice(&bytes[index..index + 8]);
        let t_s = f64::from_le_bytes(time_bytes);
        let mut len_bytes = [0_u8; 4];
        len_bytes.copy_from_slice(&bytes[index + 8..index + 12]);
        let len = u32::from_le_bytes(len_bytes) as usize;
        let start = index + 12;
        if len > bytes.len() - start {
            return ChunkParse {
                chunks,
                error: Some(format!("truncated payload at {index}")),
            };
        }
        if !t_s.is_finite() {
            return ChunkParse {
                chunks,
                error: Some(format!("non-finite time at {index}")),
            };
        }
        chunks.push(Chunk {
            t_s,
            bytes: bytes[start..start + len].to_vec(),
        });
        index = start + len;
    }
    ChunkParse {
        chunks,
        error: None,
    }
}

#[cfg(test)]
mod tests {
    use super::parse_chunks;

    #[test]
    fn one_chunk_round_trips_and_a_short_tail_is_an_error() {
        let mut bytes = 1.5_f64.to_le_bytes().to_vec();
        bytes.extend_from_slice(&3_u32.to_le_bytes());
        bytes.extend_from_slice(b"abc");
        let parsed = parse_chunks(&bytes);
        assert!(parsed.error.is_none());
        assert_eq!(parsed.chunks.len(), 1);
        assert!((parsed.chunks[0].t_s - 1.5).abs() < 1e-12);
        assert_eq!(parsed.chunks[0].bytes, b"abc");
        let bad = parse_chunks(&bytes[..10]);
        assert!(bad.error.is_some());
        assert!(bad.chunks.is_empty());
    }
}
