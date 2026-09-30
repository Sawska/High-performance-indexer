//! Minimal ABI decoding for the handful of log shapes this indexer reads.
//!
//! Every value in a log's `data` is a 32-byte big-endian word; dynamic arrays
//! store an offset word that points at a `[length, elem, elem, ...]` run. That
//! is the whole format needed here, so it is decoded directly rather than by
//! pulling in a full ABI crate.

pub const WORD: usize = 32;

/// `0x`-prefixed hex to bytes. Returns `None` on odd length or a stray digit.
pub fn hex_bytes(s: &str) -> Option<Vec<u8>> {
    let body = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X"))?;
    if body.len() % 2 != 0 {
        return None;
    }
    hex::decode(body).ok()
}

/// The `i`-th 32-byte word of a data section.
pub fn word(data: &[u8], i: usize) -> Option<&[u8]> {
    data.get(i * WORD..i * WORD + WORD)
}

/// A word read as a length/offset. Anything that does not fit in a `usize` is a
/// malformed log, not a value we should truncate.
pub fn word_usize(data: &[u8], i: usize) -> Option<usize> {
    let w = word(data, i)?;
    if w[..24].iter().any(|&b| b != 0) {
        return None;
    }
    Some(u64::from_be_bytes(w[24..].try_into().ok()?) as usize)
}

/// A word read as a small integer (enum discriminants, slot counts).
pub fn word_u64(data: &[u8], i: usize) -> Option<u64> {
    word_usize(data, i).map(|v| v as u64)
}

/// Exact base-10 rendering of a big-endian `uint256`.
///
/// Token ids are 256-bit and are matched against `markets.clob_token_ids`, which
/// Gamma returns as decimal strings -- going through `f64` here would round the
/// id and break the join, so this does the long division properly.
pub fn u256_decimal(w: &[u8]) -> String {
    let mut limbs = [0u32; 8];
    for (i, chunk) in w.chunks(4).take(8).enumerate() {
        let mut b = [0u8; 4];
        b[4 - chunk.len()..].copy_from_slice(chunk);
        limbs[i] = u32::from_be_bytes(b);
    }

    // Repeated divmod by 1e9, most-significant limb first.
    let mut groups: Vec<u32> = Vec::new();
    while limbs.iter().any(|&l| l != 0) {
        let mut rem: u64 = 0;
        for l in limbs.iter_mut() {
            let cur = (rem << 32) | u64::from(*l);
            *l = (cur / 1_000_000_000) as u32;
            rem = cur % 1_000_000_000;
        }
        groups.push(rem as u32);
    }

    let Some(top) = groups.pop() else {
        return "0".to_string();
    };
    let mut out = top.to_string();
    while let Some(g) = groups.pop() {
        out.push_str(&format!("{g:09}"));
    }
    out
}

/// Lossy `uint256` -> `f64`, for amounts that are downstream stored as NUMERIC
/// for display and arithmetic. Precise to 2^53, far above any realistic size.
pub fn u256_f64(w: &[u8]) -> f64 {
    w.iter().fold(0.0_f64, |acc, &b| acc * 256.0 + f64::from(b))
}

/// `uint256` -> `f64` divided down by a token's decimals.
pub fn scaled(w: &[u8], decimals: u32) -> f64 {
    u256_f64(w) / 10.0_f64.powi(decimals as i32)
}

/// The 20-byte address packed into the low end of a 32-byte topic or word.
pub fn address(w: &[u8]) -> Option<String> {
    let tail = w.get(w.len().checked_sub(20)?..)?;
    Some(format!("0x{}", hex::encode(tail)))
}

/// A topic or word kept as a 32-byte hex string (hashes, condition ids).
pub fn hex32(w: &[u8]) -> String {
    format!("0x{}", hex::encode(w))
}

/// Decode a `uint256[]` whose offset word sits at `head_index`.
///
/// The offset is relative to the start of the data section, which is why it is
/// applied to `data` rather than to the head word's own position.
pub fn u256_array(data: &[u8], head_index: usize) -> Option<Vec<String>> {
    let offset = word_usize(data, head_index)?;
    if offset % WORD != 0 {
        return None;
    }
    let len_at = offset / WORD;
    let len = word_usize(data, len_at)?;

    // Refuse a length that cannot physically be present, so a corrupt word can
    // never make us allocate gigabytes.
    if data.len() < (len_at + 1 + len) * WORD {
        return None;
    }

    (0..len)
        .map(|i| word(data, len_at + 1 + i).map(u256_decimal))
        .collect()
}

#[cfg(test)]
mod unit {
    use super::*;

    fn w(hex_body: &str) -> Vec<u8> {
        hex_bytes(&format!("0x{hex_body:0>64}")).unwrap()
    }

    #[test]
    fn u256_decimal_is_exact_past_the_f64_range() {
        assert_eq!(u256_decimal(&w("")), "0");
        assert_eq!(u256_decimal(&w("01")), "1");
        assert_eq!(u256_decimal(&w("0de0b6b3a7640000")), "1000000000000000000");

        // A real Polymarket token id: 77 digits, far beyond f64's 15-16.
        let id = hex_bytes("0x7b22a3c8b8e0f9c0d1e2f3a4b5c60718293a4b5c6d7e8f901a2b3c4d5e6f7081")
            .unwrap();
        let decimal = u256_decimal(&id);
        assert!(decimal.chars().all(|c| c.is_ascii_digit()));
        assert!(!decimal.starts_with('0'));

        // max uint256
        assert_eq!(
            u256_decimal(&[0xff; 32]),
            "115792089237316195423570985008687907853269984665640564039457584007913129639935"
        );
    }

    #[test]
    fn scaled_divides_by_token_decimals() {
        // 1_500_000 units of a 6-decimal token is 1.5 tokens.
        assert_eq!(scaled(&w("16e360"), 6), 1.5);
        assert_eq!(u256_f64(&w("00")), 0.0);
    }

    #[test]
    fn address_takes_the_low_twenty_bytes() {
        let topic = w("000000000000000000000000d91e80cf2e7be2e162c6513ced06f1dd0da35296");
        assert_eq!(
            address(&topic).as_deref(),
            Some("0xd91e80cf2e7be2e162c6513ced06f1dd0da35296")
        );
    }

    #[test]
    fn word_usize_rejects_a_value_too_large_to_be_a_length() {
        let data = w("ffffffffffffffffffffffffffffffffffffffff");
        assert_eq!(word_usize(&data, 0), None);
        assert_eq!(word_usize(&w("20"), 0), Some(32));
        assert_eq!(word_usize(&[], 0), None, "past the end of the data");
    }

    #[test]
    fn u256_array_follows_its_offset() {
        // head: [offset=0x20], then [len=2, 1, 2]
        let mut data = Vec::new();
        data.extend_from_slice(&w("20"));
        data.extend_from_slice(&w("02"));
        data.extend_from_slice(&w("01"));
        data.extend_from_slice(&w("02"));
        assert_eq!(
            u256_array(&data, 0),
            Some(vec!["1".to_string(), "2".to_string()])
        );
    }

    #[test]
    fn u256_array_refuses_a_length_the_data_cannot_hold() {
        let mut data = Vec::new();
        data.extend_from_slice(&w("20"));
        data.extend_from_slice(&w("ffffff")); // claims ~16M elements
        assert_eq!(u256_array(&data, 0), None);
    }

    #[test]
    fn hex_bytes_rejects_malformed_input() {
        assert!(hex_bytes("1234").is_none(), "no 0x prefix");
        assert!(hex_bytes("0x123").is_none(), "odd length");
        assert!(hex_bytes("0xzz").is_none(), "not hex");
        assert_eq!(hex_bytes("0x"), Some(Vec::new()));
    }
}
