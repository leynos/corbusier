//! Lowercase hexadecimal rendering for digest bytes.
//!
//! `sha2` 0.11 returns `hybrid_array::Array<u8, _>` from `finalize` and
//! `digest`, and that type does not implement [`core::fmt::LowerHex`], so
//! `format!("{:x}", digest)` no longer compiles. This module renders raw
//! digest bytes explicitly, without pulling in a dedicated hex-encoding
//! dependency.
//!
//! # Scope and re-use policy
//!
//! This is the single crate-internal hex encoder; call it from anywhere that
//! needs to render bytes as hexadecimal rather than hand-rolling another
//! nibble loop. It deliberately owns no digest knowledge: callers hash, then
//! encode. The functions are `pub(crate)` because the rendering is an
//! implementation detail of identifier construction, not part of the crate's
//! public contract.

/// Encodes `bytes` as a lowercase hexadecimal string.
///
/// Every byte is rendered as exactly two digits, including leading zeroes, so
/// the returned string is always twice the length of `bytes`.
///
/// ```ignore
/// assert_eq!(to_lower_hex(&[0x00, 0xa0, 0xff]), "00a0ff");
/// ```
#[must_use]
pub(crate) fn to_lower_hex(bytes: &[u8]) -> String {
    to_lower_hex_prefix(bytes, bytes.len())
}

/// Encodes at most the first `byte_limit` bytes of `bytes` as lowercase
/// hexadecimal.
///
/// A `byte_limit` beyond the length of `bytes` renders the whole input, so
/// callers need not clamp the limit themselves. This exists so that callers
/// wanting a truncated digest suffix can avoid slicing the digest, which the
/// crate's lint policy forbids.
///
/// ```ignore
/// assert_eq!(to_lower_hex_prefix(&[0x0f, 0xff], 1), "0f");
/// assert_eq!(to_lower_hex_prefix(&[0x0f], 8), "0f");
/// ```
#[must_use]
pub(crate) fn to_lower_hex_prefix(bytes: &[u8], byte_limit: usize) -> String {
    let limit = byte_limit.min(bytes.len());
    let mut hex = String::with_capacity(limit * 2);
    for &byte in bytes.iter().take(limit) {
        hex.push(nibble_digit(byte >> 4));
        hex.push(nibble_digit(byte));
    }
    hex
}

/// Renders the low nibble of `byte` as a lowercase hexadecimal digit.
///
/// The nibble is masked here rather than by the caller so the mapping is total
/// over every `u8`; deriving the digit arithmetically also avoids a lookup
/// table, which would need a bounds check under this crate's lint policy.
const fn nibble_digit(byte: u8) -> char {
    let nibble = byte & 0x0f;
    let ascii = if nibble < 10 {
        b'0' + nibble
    } else {
        b'a' + (nibble - 10)
    };
    ascii as char
}

#[cfg(test)]
mod tests {
    //! Tests for the lowercase hexadecimal encoder.

    use super::{to_lower_hex, to_lower_hex_prefix};
    use rstest::rstest;

    #[rstest]
    #[case::empty(&[], "")]
    #[case::leading_zeroes(&[0x00, 0x0f, 0xff, 0xa0], "000fffa0")]
    #[case::all_nibbles(&[0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef], "0123456789abcdef")]
    fn renders_bytes_as_two_digits_each(#[case] bytes: &[u8], #[case] expected: &str) {
        assert_eq!(to_lower_hex(bytes), expected);
    }

    #[rstest]
    #[case::truncates(&[0x0f, 0xff, 0x10], 2, "0fff")]
    #[case::zero_limit(&[0x0f], 0, "")]
    #[case::limit_beyond_input(&[0x0f], 8, "0f")]
    fn prefix_encodes_at_most_the_requested_bytes(
        #[case] bytes: &[u8],
        #[case] byte_limit: usize,
        #[case] expected: &str,
    ) {
        assert_eq!(to_lower_hex_prefix(bytes, byte_limit), expected);
    }

    /// Bounded exhaustive check over the whole `u8` range: each byte must
    /// render as exactly two lowercase ASCII hex digits that parse back to the
    /// original value. A couple of example vectors would miss leading-zero and
    /// nibble-boundary bugs that this catches.
    #[test]
    fn every_byte_renders_as_two_round_tripping_lowercase_digits() {
        for byte in u8::MIN..=u8::MAX {
            let rendered = to_lower_hex(&[byte]);
            assert_eq!(rendered.len(), 2, "byte {byte:#04x} must render two digits");
            assert!(
                rendered
                    .bytes()
                    .all(|digit| digit.is_ascii_digit() || (b'a'..=b'f').contains(&digit)),
                "byte {byte:#04x} rendered non-lowercase-hex output {rendered:?}",
            );
            let parsed =
                u8::from_str_radix(&rendered, 16).expect("two hex digits parse as a byte value");
            assert_eq!(parsed, byte, "round-trip mismatch for byte {byte:#04x}");
        }
    }
}
