//! Random tokens and their stored hashes (SEC-07, SEC-09).

use sha2::{Digest, Sha256};

const ALPHABET: &[u8; 32] = b"abcdefghijklmnopqrstuvwxyz234567";

/// 128 random bits as 26 lowercase base32 characters.
pub fn random_token() -> String {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).expect("OS random source unavailable");
    base32(&bytes)
}

/// SHA-256 as lowercase hex. The only form a secret token is stored in.
pub fn hash(token: &str) -> String {
    Sha256::digest(token.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// A uniform-enough random number in `0..n` for spreading check offsets. Not for secrets.
pub fn random_below(n: u64) -> u64 {
    let mut bytes = [0u8; 8];
    getrandom::fill(&mut bytes).expect("OS random source unavailable");
    u64::from_le_bytes(bytes) % n.max(1)
}

/// RFC 4648 base32 in lowercase, without padding.
pub fn base32(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 8 / 5 + 1);
    let mut buf: u32 = 0;
    let mut bits = 0;
    for &b in bytes {
        buf = (buf << 8) | u32::from(b);
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            out.push(ALPHABET[((buf >> bits) & 31) as usize] as char);
        }
        buf &= (1 << bits) - 1;
    }
    if bits > 0 {
        out.push(ALPHABET[((buf << (5 - bits)) & 31) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base32_matches_rfc4648_without_padding() {
        assert_eq!(base32(b"foobar"), "mzxw6ytboi");
    }

    #[test]
    fn token_is_26_base32_chars_and_unique() {
        let a = random_token();
        assert_eq!(a.len(), 26);
        assert!(a.bytes().all(|c| ALPHABET.contains(&c)));
        assert_ne!(a, random_token());
    }

    #[test]
    fn hash_is_sha256_hex() {
        assert_eq!(
            hash("abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
