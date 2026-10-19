//! RFC 6238 time-based codes: HMAC-SHA-1 (RFC 2104), 6 digits, 30-second steps. Pure, no I/O.
//! `sha1_smol` is already built for `uuid`, so the HMAC here costs no new crate (decision Q19).

use crate::platform::token;

const STEP_S: i64 = 30;
const MODULUS: u32 = 1_000_000;
/// 160 bits, the key length RFC 4226 recommends for SHA-1.
const SECRET_BYTES: usize = 20;
const BLOCK: usize = 64;

fn sha1(parts: &[&[u8]]) -> [u8; 20] {
    let mut h = sha1_smol::Sha1::new();
    for p in parts {
        h.update(p);
    }
    h.digest().bytes()
}

fn hmac_sha1(key: &[u8], msg: &[u8]) -> [u8; 20] {
    let mut k = [0u8; BLOCK];
    if key.len() > BLOCK {
        k[..20].copy_from_slice(&sha1(&[key]));
    } else {
        k[..key.len()].copy_from_slice(key);
    }
    let ipad = k.map(|b| b ^ 0x36);
    let opad = k.map(|b| b ^ 0x5c);
    sha1(&[&opad, &sha1(&[&ipad, msg])])
}

/// RFC 4226 HOTP with dynamic truncation, before the modulus.
fn truncated(key: &[u8], counter: u64) -> u32 {
    let h = hmac_sha1(key, &counter.to_be_bytes());
    let o = usize::from(h[19] & 0x0f);
    u32::from_be_bytes([h[o], h[o + 1], h[o + 2], h[o + 3]]) & 0x7fff_ffff
}

/// A new random secret in uppercase base32, as authenticator apps show it.
pub fn new_secret() -> String {
    let mut bytes = [0u8; SECRET_BYTES];
    getrandom::fill(&mut bytes).expect("OS random source unavailable");
    token::base32(&bytes).to_uppercase()
}

/// Base32 to bytes, ignoring case, spaces and padding; None for any other character.
fn decode(secret: &str) -> Option<Vec<u8>> {
    let (mut buf, mut bits, mut out) = (0u32, 0, Vec::new());
    for c in secret.chars().filter(|c| !matches!(c, ' ' | '=')) {
        let v = match c.to_ascii_uppercase() {
            c @ 'A'..='Z' => c as u32 - 'A' as u32,
            c @ '2'..='7' => c as u32 - '2' as u32 + 26,
            _ => return None,
        };
        buf = (buf << 5) | v;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
            buf &= (1 << bits) - 1;
        }
    }
    Some(out)
}

/// The step of the code `code` matches: within one step of `now` either way, and later than
/// `last`, so no code works twice. None for a wrong, reused or malformed code.
pub fn check(secret: &str, code: &str, now: i64, last: Option<i64>) -> Option<i64> {
    let code: String = code.chars().filter(|c| !c.is_whitespace()).collect();
    if code.len() != 6 || !code.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let want: u32 = code.parse().ok()?;
    let key = decode(secret)?;
    let now_step = now.div_euclid(STEP_S);
    (now_step - 1..=now_step + 1)
        .filter(|&s| s >= 0 && last.is_none_or(|l| s > l))
        .find(|&s| truncated(&key, s as u64) % MODULUS == want)
}

/// The code for `at`, for tests that sign in as an authenticator app would.
pub fn code_at(secret: &str, at: i64) -> String {
    let key = decode(secret).unwrap_or_default();
    format!(
        "{:06}",
        truncated(&key, at.div_euclid(STEP_S) as u64) % MODULUS
    )
}

/// The `otpauth://` link authenticator apps read (Key Uri Format), for `email` at `issuer`.
pub fn link(issuer: &str, email: &str, secret: &str) -> String {
    let enc = |s: &str| -> String {
        s.bytes()
            .map(|b| match b {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                    (b as char).to_string()
                }
                _ => format!("%{b:02X}"),
            })
            .collect()
    };
    format!(
        "otpauth://totp/{}:{}?secret={secret}&issuer={}&algorithm=SHA1&digits=6&period=30",
        enc(issuer),
        enc(email),
        enc(issuer)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// RFC 6238 appendix B: the ASCII key "12345678901234567890".
    const RFC_SECRET: &str = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";

    #[test]
    fn hmac_sha1_matches_rfc_2202() {
        let hex: String = hmac_sha1(&[0x0b; 20], b"Hi There")
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert_eq!(hex, "b617318655057264e28bc0b6fb378c8ef146be00");
    }

    #[test]
    fn codes_match_rfc_6238_sha1_rows() {
        // The RFC prints 8 digits; the last 6 are the code.
        for (t, code) in [
            (59, "287082"),
            (1_111_111_109, "081804"),
            (1_234_567_890, "005924"),
            (20_000_000_000, "353130"),
        ] {
            assert_eq!(code_at(RFC_SECRET, t), code, "T = {t}");
        }
    }

    #[test]
    fn base32_round_trips_and_ignores_case_and_spaces() {
        assert_eq!(decode(RFC_SECRET).unwrap(), b"12345678901234567890");
        assert_eq!(decode("gezd gnbv").unwrap(), decode("GEZDGNBV").unwrap());
        assert_eq!(decode("GEZ1"), None);
        let s = new_secret();
        assert_eq!(s.len(), 32);
        assert_eq!(decode(&s).unwrap().len(), SECRET_BYTES);
    }

    #[test]
    fn a_code_works_one_step_either_side_and_never_twice() {
        let now = 1_234_567_890;
        let step = now / 30;
        let before = code_at(RFC_SECRET, now - 30);
        let after = code_at(RFC_SECRET, now + 30);
        assert_eq!(check(RFC_SECRET, &before, now, None), Some(step - 1));
        assert_eq!(check(RFC_SECRET, &after, now, None), Some(step + 1));
        assert_eq!(
            check(RFC_SECRET, &code_at(RFC_SECRET, now - 60), now, None),
            None
        );
        assert_eq!(check(RFC_SECRET, "005 924", now, None), Some(step));
        assert_eq!(check(RFC_SECRET, "005924", now, Some(step)), None, "reused");
        assert_eq!(
            check(RFC_SECRET, &after, now, Some(step)),
            Some(step + 1),
            "a later step still works"
        );
        assert_eq!(check(RFC_SECRET, "12345", now, None), None);
        assert_eq!(check(RFC_SECRET, "abcdef", now, None), None);
    }

    #[test]
    fn the_link_names_the_issuer_and_escapes_the_email() {
        assert_eq!(
            link("Monita", "a+b@x.co", "ABC"),
            "otpauth://totp/Monita:a%2Bb%40x.co?secret=ABC&issuer=Monita&algorithm=SHA1&digits=6&period=30"
        );
    }
}
