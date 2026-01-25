//! yEnc decoder
//!
//! yEnc is the standard encoding for binary data on Usenet.
//! It's more efficient than Base64, using 252 out of 256 byte values directly.
//!
//! # yEnc Format
//!
//! ```text
//! =ybegin part=1 line=128 size=123456 name=filename.ext
//! =ypart begin=1 end=123456
//! <encoded binary data>
//! =yend size=123456 part=1 pcrc32=ABCD1234 crc32=DEADBEEF
//! ```
//!
//! # Encoding Rules
//!
//! - Each byte is encoded as: `(byte + 42) % 256`
//! - Escape character is `=` (0x3D)
//! - Escaped bytes are: `\0`, `\n`, `\r`, `=`
//! - Escaped format: `=` followed by `(original + 64) % 256`

use anyhow::{Result, anyhow};
use tracing::{debug, warn};

/// Result of yEnc decoding
#[derive(Debug, Clone)]
pub struct YencDecoded {
    /// Decoded binary data
    pub data: Vec<u8>,
    /// Filename from yEnc header
    pub filename: Option<String>,
    /// Part number (for multi-part posts)
    pub part: Option<u32>,
    /// Expected size from header
    pub expected_size: Option<u64>,
    /// CRC32 from trailer (if present)
    pub crc32: Option<u32>,
    /// Part CRC32 from trailer (if present)
    pub pcrc32: Option<u32>,
    /// Begin offset (for multi-part)
    pub begin: Option<u64>,
    /// End offset (for multi-part)
    pub end: Option<u64>,
}

/// Decode yEnc-encoded data
///
/// # Arguments
/// * `data` - The raw article body including yEnc headers/trailers
///
/// # Returns
/// Decoded binary data and metadata
pub fn decode_yenc(data: &[u8]) -> Result<YencDecoded> {
    let text = String::from_utf8_lossy(data);
    let lines: Vec<&str> = text.lines().collect();

    if lines.is_empty() {
        return Err(anyhow!("Empty yEnc data"));
    }

    // Find =ybegin header
    let (ybegin_idx, ybegin_line) = lines
        .iter()
        .enumerate()
        .find(|(_, line)| line.starts_with("=ybegin "))
        .ok_or_else(|| anyhow!("No =ybegin header found"))?;

    // Parse =ybegin header
    let mut filename = None;
    let mut expected_size = None;
    let mut part = None;
    for token in ybegin_line.split_whitespace().skip(1) {
        if let Some((key, value)) = token.split_once('=') {
            match key {
                "name" => filename = Some(value.to_string()),
                "size" => expected_size = value.parse::<u64>().ok(),
                "part" => part = value.parse::<u32>().ok(),
                _ => {}
            }
        }
    }

    // Check for =ypart header (multi-part)
    let mut begin = None;
    let mut end = None;
    let data_start;

    if let Some(ypart_line) = lines.get(ybegin_idx + 1) {
        if ypart_line.starts_with("=ypart ") {
            for token in ypart_line.split_whitespace().skip(1) {
                if let Some((key, value)) = token.split_once('=') {
                    match key {
                        "begin" => begin = value.parse::<u64>().ok(),
                        "end" => end = value.parse::<u64>().ok(),
                        _ => {}
                    }
                }
            }
            data_start = ybegin_idx + 2;
        } else {
            data_start = ybegin_idx + 1;
        }
    } else {
        data_start = ybegin_idx + 1;
    }

    // Find =yend trailer and get CRC values
    let mut crc32 = None;
    let mut pcrc32 = None;
    let mut data_end = lines.len();

    for (idx, line) in lines.iter().enumerate().skip(data_start) {
        if line.starts_with("=yend ") {
            data_end = idx;

            for token in line.split_whitespace().skip(1) {
                if let Some((key, value)) = token.split_once('=') {
                    match key {
                        "crc32" => crc32 = u32::from_str_radix(value, 16).ok(),
                        "pcrc32" => pcrc32 = u32::from_str_radix(value, 16).ok(),
                        _ => {}
                    }
                }
            }
            break;
        }
    }

    // Decode the data lines
    let mut decoded = Vec::new();
    let mut escape_next = false;

    for line in &lines[data_start..data_end] {
        for byte in line.bytes() {
            if escape_next {
                // Escaped byte: subtract 64, then subtract 42
                let original = byte.wrapping_sub(64).wrapping_sub(42);
                decoded.push(original);
                escape_next = false;
            } else if byte == b'=' {
                escape_next = true;
            } else {
                // Normal byte: subtract 42
                let original = byte.wrapping_sub(42);
                decoded.push(original);
            }
        }
    }

    // Verify size if expected
    if let Some(expected) = expected_size {
        // For parts, check against part size
        if let (Some(b), Some(e)) = (begin, end) {
            let part_expected = (e - b + 1) as usize;
            if decoded.len() != part_expected {
                debug!(
                    expected = part_expected,
                    actual = decoded.len(),
                    "yEnc part size mismatch"
                );
            }
        } else if decoded.len() != expected as usize {
            warn!(
                expected = expected,
                actual = decoded.len(),
                "yEnc size mismatch"
            );
        }
    }

    // TODO: Verify CRC32 if present
    // The crc32fast crate would be good for this

    Ok(YencDecoded {
        data: decoded,
        filename,
        part,
        expected_size,
        crc32,
        pcrc32,
        begin,
        end,
    })
}

/// Encode data in yEnc format
///
/// This is primarily for testing, but could be used for uploading
pub fn encode_yenc(data: &[u8], filename: &str, line_length: usize) -> Vec<u8> {
    let mut result = Vec::new();

    // Write =ybegin header
    let header = format!("=ybegin line={} size={} name={}\r\n", line_length, data.len(), filename);
    result.extend_from_slice(header.as_bytes());

    // Encode data
    let mut line_pos = 0;
    for &byte in data {
        let encoded = byte.wrapping_add(42);

        // Check if needs escaping
        let needs_escape = matches!(encoded, 0x00 | 0x0A | 0x0D | 0x3D);

        if needs_escape {
            result.push(b'=');
            result.push(encoded.wrapping_add(64));
            line_pos += 2;
        } else {
            result.push(encoded);
            line_pos += 1;
        }

        // Line wrap
        if line_pos >= line_length {
            result.extend_from_slice(b"\r\n");
            line_pos = 0;
        }
    }

    // Final newline if needed
    if line_pos > 0 {
        result.extend_from_slice(b"\r\n");
    }

    // Write =yend trailer
    let crc = crc32fast::hash(data);
    let trailer = format!("=yend size={} crc32={:08X}\r\n", data.len(), crc);
    result.extend_from_slice(trailer.as_bytes());

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_simple() {
        let encoded = b"=ybegin line=128 size=5 name=test.txt\r\n\
                        MN[\\]\r\n\
                        =yend size=5 crc32=12345678\r\n";

        let decoded = decode_yenc(encoded).unwrap();
        assert_eq!(decoded.filename, Some("test.txt".to_string()));
        assert_eq!(decoded.expected_size, Some(5));
    }

    #[test]
    fn test_encode_decode_roundtrip() {
        let original = b"Hello, World! This is a test.";
        let encoded = encode_yenc(original, "test.txt", 128);
        let decoded = decode_yenc(&encoded).unwrap();

        assert_eq!(decoded.data, original);
        assert_eq!(decoded.filename, Some("test.txt".to_string()));
    }

    #[test]
    fn test_escape_characters() {
        // Test that special characters are properly escaped and decoded
        let original: Vec<u8> = vec![0x00, 0x0A, 0x0D, 0x3D - 42]; // Characters that become special when encoded
        let encoded = encode_yenc(&original, "special.bin", 128);
        let decoded = decode_yenc(&encoded).unwrap();

        assert_eq!(decoded.data, original);
    }
}
