//! ZIP local headers only. Deflate payloads are blocked rather than decoded.

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ZipEntry {
    pub name: String,
    pub method: u16,
    pub blocked: bool,
}

pub fn zip_entries(bytes: &[u8]) -> Result<Vec<ZipEntry>, String> {
    let mut entries = Vec::new();
    let mut index = 0;
    while index + 4 <= bytes.len() {
        if bytes[index..index + 4] != [0x50, 0x4b, 0x03, 0x04] {
            break;
        }
        if index + 30 > bytes.len() {
            return Err("truncated zip header".to_string());
        }
        let flags = u16::from_le_bytes([bytes[index + 6], bytes[index + 7]]);
        let method = u16::from_le_bytes([bytes[index + 8], bytes[index + 9]]);
        let comp_size_u32 = u32::from_le_bytes([
            bytes[index + 18],
            bytes[index + 19],
            bytes[index + 20],
            bytes[index + 21],
        ]);
        let comp_size = comp_size_u32 as usize;
        let name_len = u16::from_le_bytes([bytes[index + 26], bytes[index + 27]]) as usize;
        let extra_len = u16::from_le_bytes([bytes[index + 28], bytes[index + 29]]) as usize;
        let name_at = index + 30;
        let name_end = name_at
            .checked_add(name_len)
            .ok_or_else(|| "zip name".to_string())?;
        if name_end + extra_len > bytes.len() {
            return Err("truncated zip name".to_string());
        }
        let name = String::from_utf8_lossy(&bytes[name_at..name_end]).into_owned();
        let blocked = method != 0 || flags & 0x0008 != 0 || comp_size_u32 == u32::MAX;
        entries.push(ZipEntry {
            name,
            method,
            blocked,
        });
        if blocked {
            break;
        }
        let data_at = name_end + extra_len;
        index = data_at
            .checked_add(comp_size)
            .ok_or_else(|| "zip size".to_string())?;
        if index > bytes.len() {
            return Err("truncated zip payload".to_string());
        }
    }
    if entries.is_empty() {
        return Err("no zip local header".to_string());
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::zip_entries;

    #[test]
    fn stored_payloads_can_be_skipped_and_deflate_stops() {
        let mut stored = Vec::new();
        stored.extend_from_slice(&[0x50, 0x4b, 0x03, 0x04]);
        stored.extend_from_slice(&20_u16.to_le_bytes());
        stored.extend_from_slice(&0_u16.to_le_bytes());
        stored.extend_from_slice(&0_u16.to_le_bytes());
        stored.extend_from_slice(&0_u16.to_le_bytes());
        stored.extend_from_slice(&0_u16.to_le_bytes());
        stored.extend_from_slice(&0_u32.to_le_bytes());
        stored.extend_from_slice(&4_u32.to_le_bytes());
        stored.extend_from_slice(&4_u32.to_le_bytes());
        stored.extend_from_slice(&1_u16.to_le_bytes());
        stored.extend_from_slice(&0_u16.to_le_bytes());
        stored.push(b'a');
        stored.extend_from_slice(b"wxyz");
        let entries = zip_entries(&stored).unwrap();
        assert_eq!(entries.len(), 1);
        assert!(!entries[0].blocked);

        let mut deflate = stored.clone();
        deflate[8] = 8;
        deflate[9] = 0;
        let entries = zip_entries(&deflate).unwrap();
        assert!(entries[0].blocked);
        assert_eq!(entries[0].method, 8);
    }

    #[test]
    fn a_u32_max_size_is_blocked_without_widening() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&[0x50, 0x4b, 0x03, 0x04]);
        bytes.extend_from_slice(&20_u16.to_le_bytes());
        bytes.extend_from_slice(&0_u16.to_le_bytes());
        bytes.extend_from_slice(&0_u16.to_le_bytes());
        bytes.extend_from_slice(&0_u16.to_le_bytes());
        bytes.extend_from_slice(&0_u16.to_le_bytes());
        bytes.extend_from_slice(&0_u32.to_le_bytes());
        bytes.extend_from_slice(&u32::MAX.to_le_bytes());
        bytes.extend_from_slice(&u32::MAX.to_le_bytes());
        bytes.extend_from_slice(&1_u16.to_le_bytes());
        bytes.extend_from_slice(&0_u16.to_le_bytes());
        bytes.push(b'a');
        let entries =
            zip_entries(&bytes).expect("sentinel is a blocked header, not a huge payload");
        assert_eq!(entries.len(), 1);
        assert!(entries[0].blocked);
        assert_eq!(entries[0].method, 0);
        assert_eq!(entries[0].name, "a");
    }
}
