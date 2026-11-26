//! Text helpers that reproduce how the Python implementation read files.

use std::borrow::Cow;

/// Decode file contents. Invalid UTF-8 is replaced rather than rejected, and
/// a leading byte-order mark is dropped.
pub fn decode(bytes: &[u8]) -> Cow<'_, str> {
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    String::from_utf8_lossy(bytes)
}

/// True if a file's contents start like a gzip-compressed file.
pub fn is_gzip(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0x1f, 0x8b])
}

/// A file's contents, decompressed if they are gzip-compressed (told by
/// their first bytes, not by the file's name), otherwise as they are.
/// Several gzip members one after another (as `cat a.gz b.gz` makes) are
/// read as one file.
pub fn decompress(bytes: &[u8]) -> Result<Cow<'_, [u8]>, String> {
    use std::io::Read;
    if !is_gzip(bytes) {
        return Ok(bytes.into());
    }
    let mut buffer = Vec::new();
    flate2::read::MultiGzDecoder::new(bytes)
        .read_to_end(&mut buffer)
        .map_err(|e| {
            format!(
                "it is gzip-compressed but could not be decompressed ({e}); it may be incomplete"
            )
        })?;
    Ok(buffer.into())
}

/// Up to `limit` bytes from the start of a file's contents, decompressed if
/// they are gzip-compressed. `bytes` may be only the start of a compressed
/// file (as read to tell a file's format): whatever can be decompressed
/// from it is returned.
pub fn decompressed_start(bytes: &[u8], limit: usize) -> Vec<u8> {
    use std::io::Read;
    if !is_gzip(bytes) {
        return bytes[..bytes.len().min(limit)].to_vec();
    }
    let mut decoder = flate2::read::MultiGzDecoder::new(bytes).take(limit as u64);
    let mut out = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        match decoder.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(n) => out.extend_from_slice(&chunk[..n]),
        }
    }
    out
}

/// Split text into lines, accepting `\n`, `\r\n` and `\r` line endings.
/// A final line terminator does not produce an empty last line.
pub fn lines(text: &str) -> Vec<&str> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut start = 0;
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\n' => {
                out.push(&text[start..i]);
                start = i + 1;
            }
            b'\r' => {
                out.push(&text[start..i]);
                if bytes.get(i + 1) == Some(&b'\n') {
                    i += 1;
                }
                start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    if start < bytes.len() {
        out.push(&text[start..]);
    }
    out
}

/// Remove leading and trailing ASCII whitespace (Python's `str.strip()`).
pub fn strip(s: &str) -> &str {
    s.trim_matches(|c| matches!(c, ' ' | '\t' | '\n' | '\r' | '\x0b' | '\x0c'))
}

/// Split a line on tabs and strip every cell.
pub fn cells(line: &str) -> Vec<&str> {
    line.split('\t').map(strip).collect()
}

/// Parse a floating-point number the way Python's `float()` does for the
/// values that occur in profile files.
pub fn parse_float(s: &str) -> Option<f64> {
    let s = strip(s);
    if s.is_empty() || s.contains('_') {
        return None;
    }
    s.parse::<f64>().ok()
}

/// True if the string is a number, optionally prefixed by `<`, `>`, `<=` or
/// `>=`. Used to find the first sample column of a profile.
pub fn is_number(s: &str) -> bool {
    let mut s = s;
    if let Some(rest) = s.strip_prefix(['<', '>']) {
        s = rest;
    }
    if let Some(rest) = s.strip_prefix('=') {
        s = rest;
    }
    parse_float(s).is_some()
}

/// Format a count the way Python's `str(float)` does for whole numbers
/// (`3.0`), using the shortest exact form otherwise.
pub fn format_float(value: f64) -> String {
    format!("{value:?}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_all_line_endings() {
        assert_eq!(lines("a\nb\r\nc\rd"), ["a", "b", "c", "d"]);
        assert_eq!(lines("a\n\nb\n"), ["a", "", "b"]);
        assert_eq!(lines("a\r\n\r\n"), ["a", ""]);
        assert!(lines("").is_empty());
    }

    #[test]
    fn recognises_numbers() {
        for s in ["5", "5.0", "-1e3", ".5", "<5", ">=0.1", "inf", "nan", " 7 "] {
            assert!(is_number(s), "{s}");
        }
        for s in ["", "k__Bacteria", "5a", "<", "1_000", "COG0001"] {
            assert!(!is_number(s), "{s}");
        }
    }

    #[test]
    fn formats_like_python() {
        assert_eq!(format_float(3.0), "3.0");
        assert_eq!(format_float(0.5), "0.5");
    }

    fn gzip(bytes: &[u8]) -> Vec<u8> {
        use std::io::Write;
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(bytes).unwrap();
        encoder.finish().unwrap()
    }

    #[test]
    fn gzip_files_are_decompressed() {
        let text = "sample\tcoverage\ttaxonomy\nA\t1\tRoot; d__Bacteria\n".repeat(2000);
        let compressed = gzip(text.as_bytes());
        assert!(is_gzip(&compressed));
        assert_eq!(decompress(&compressed).unwrap(), text.as_bytes());
        // Plain files are given back as they are, without a copy.
        assert!(matches!(decompress(text.as_bytes()), Ok(Cow::Borrowed(_))));
        // Two members one after the other, as `cat a.gz b.gz` writes.
        let mut two = gzip(b"a\n");
        two.extend(gzip(b"b\n"));
        assert_eq!(decompress(&two).unwrap(), b"a\nb\n".as_slice());
        // A file cut short is an error, not a shorter file.
        let cut = &compressed[..compressed.len() / 2];
        assert!(decompress(cut)
            .unwrap_err()
            .contains("could not be decompressed"));
    }

    #[test]
    fn the_start_of_a_compressed_file_is_decompressed() {
        let text = "line of a profile\n".repeat(20_000);
        let compressed = gzip(text.as_bytes());
        assert_eq!(
            decompressed_start(&compressed, 100),
            &text.as_bytes()[..100]
        );
        // Only the start of the compressed file: what it holds is returned.
        let start = decompressed_start(&compressed[..compressed.len() / 2], usize::MAX);
        assert!(!start.is_empty() && text.as_bytes().starts_with(&start));
        assert_eq!(decompressed_start(b"plain text", 5), b"plain".as_slice());
    }

    #[test]
    fn decodes_and_drops_bom() {
        assert_eq!(decode(b"\xEF\xBB\xBFabc"), "abc");
    }
}
