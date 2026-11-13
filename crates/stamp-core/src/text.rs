//! Text helpers that reproduce how the Python implementation read files.

use std::borrow::Cow;

/// Decode file contents. Invalid UTF-8 is replaced rather than rejected, and
/// a leading byte-order mark is dropped.
pub fn decode(bytes: &[u8]) -> Cow<'_, str> {
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    String::from_utf8_lossy(bytes)
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

    #[test]
    fn decodes_and_drops_bom() {
        assert_eq!(decode(b"\xEF\xBB\xBFabc"), "abc");
    }
}
