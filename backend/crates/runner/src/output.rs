//! Turning captured process output into the strings the protocol carries.
//!
//! Capture itself caps each stream at `max_output_bytes` (see `exec`), which
//! means a cut can land in the middle of a multi-byte character. Decoding such
//! a buffer naively would end every truncated output with a spurious U+FFFD,
//! so the incomplete tail is dropped first. Bytes that are invalid *inside*
//! the output are a different matter — a program printing binary garbage — and
//! are replaced, not hidden, so the user sees that something odd was printed.

/// Drop an incomplete UTF-8 sequence at the very end of `bytes`, if any.
///
/// Only an *unfinished* final character is removed; an invalid byte anywhere
/// else is kept for lossy decoding to replace.
pub fn trim_partial_char(bytes: &[u8]) -> &[u8] {
    // A UTF-8 character is at most four bytes, so only the last three can
    // belong to an unfinished one; checking just that window keeps this O(1)
    // no matter how long (or how invalid) the rest of the buffer is.
    let window_start = bytes.len().saturating_sub(3);
    for start in (window_start..bytes.len()).rev() {
        let b = bytes[start];
        if b & 0b1100_0000 == 0b1000_0000 {
            continue; // continuation byte: keep looking for the lead byte
        }
        let needed = match b {
            0xC0..=0xDF => 2,
            0xE0..=0xEF => 3,
            0xF0..=0xF7 => 4,
            _ => return bytes, // ASCII or invalid lead: nothing unfinished
        };
        return if bytes.len() - start < needed {
            &bytes[..start]
        } else {
            bytes
        };
    }
    bytes
}

/// Decode captured bytes for the response: incomplete tail dropped, invalid
/// sequences replaced with U+FFFD.
pub fn decode(bytes: &[u8]) -> String {
    String::from_utf8_lossy(trim_partial_char(bytes)).into_owned()
}

/// Concatenate two streams (a compiler's stdout then stderr) under one byte
/// cap. Returns the decoded text and whether anything was cut.
pub fn combine_capped(first: &[u8], second: &[u8], cap: usize) -> (String, bool) {
    let mut joined = Vec::with_capacity((first.len() + second.len()).min(cap));
    for part in [first, second] {
        let room = cap - joined.len();
        joined.extend_from_slice(&part[..part.len().min(room)]);
    }
    let cut = first.len() + second.len() > cap;
    (decode(&joined), cut)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_character_split_by_the_cap_is_dropped_not_mangled() {
        let s = "ok é".as_bytes(); // 'é' is 2 bytes
        let cut = &s[..s.len() - 1];
        assert_eq!(decode(cut), "ok ");

        let euro = "x€".as_bytes(); // '€' is 3 bytes
        assert_eq!(decode(&euro[..2]), "x");
        assert_eq!(decode(&euro[..3]), "x");
        assert_eq!(decode(euro), "x€");

        let crab = "🦀".as_bytes(); // 4 bytes
        for n in 1..4 {
            assert_eq!(decode(&crab[..n]), "", "cut after {n} bytes");
        }
        assert_eq!(decode(crab), "🦀");
    }

    #[test]
    fn complete_text_is_untouched() {
        assert_eq!(decode(b"0 1\n"), "0 1\n");
        assert_eq!(decode("héllo wörld".as_bytes()), "héllo wörld");
        assert_eq!(decode(b""), "");
    }

    #[test]
    fn invalid_bytes_inside_the_output_are_replaced_not_hidden() {
        let out = decode(b"a\xFFb\n");
        assert_eq!(out, "a\u{FFFD}b\n");
        // A stray continuation byte at the end is invalid, not unfinished.
        assert_eq!(decode(b"ab\x80"), "ab\u{FFFD}");
    }

    #[test]
    fn combined_compiler_output_respects_one_cap() {
        let (s, cut) = combine_capped(b"warn\n", b"error\n", 64);
        assert_eq!(s, "warn\nerror\n");
        assert!(!cut);

        let (s, cut) = combine_capped(b"abcdef", b"ghij", 8);
        assert_eq!(s, "abcdefgh");
        assert!(cut);

        let (s, cut) = combine_capped(b"abcdefghij", b"xyz", 4);
        assert_eq!(s, "abcd");
        assert!(cut);
    }
}
