//! A certificate's expiry (v1.1 Q25): just enough DER to reach `tbsCertificate.validity.notAfter`.
//! The HTTP check gets the leaf certificate from reqwest's `TlsInfo`, so no TLS code runs here.

use crate::platform::time::days_from_civil;

const SEQUENCE: u8 = 0x30;
const VERSION: u8 = 0xa0;
const UTC_TIME: u8 = 0x17;
const GENERALIZED_TIME: u8 = 0x18;

/// The certificate's `notAfter` in Unix seconds, or None when the DER does not parse.
pub fn not_after(der: &[u8]) -> Option<i64> {
    let (SEQUENCE, cert, _) = tlv(der)? else {
        return None;
    };
    let (SEQUENCE, tbs, _) = tlv(cert)? else {
        return None;
    };
    let (tag, _, after_version) = tlv(tbs)?;
    let mut rest = if tag == VERSION { after_version } else { tbs };
    // serialNumber, signature, issuer
    for _ in 0..3 {
        rest = tlv(rest)?.2;
    }
    let (SEQUENCE, validity, _) = tlv(rest)? else {
        return None;
    };
    let (_, _, after_not_before) = tlv(validity)?;
    let (tag, time, _) = tlv(after_not_before)?;
    parse_time(tag, time)
}

/// One DER element: its tag, its contents and what follows it.
fn tlv(b: &[u8]) -> Option<(u8, &[u8], &[u8])> {
    let (&tag, b) = b.split_first()?;
    let (&first, mut b) = b.split_first()?;
    let len = if first < 0x80 {
        usize::from(first)
    } else {
        let n = usize::from(first & 0x7f);
        if n == 0 || n > 4 || b.len() < n {
            return None;
        }
        let len = b[..n]
            .iter()
            .fold(0usize, |l, &x| (l << 8) | usize::from(x));
        b = &b[n..];
        len
    };
    (b.len() >= len).then(|| (tag, &b[..len], &b[len..]))
}

/// `YYMMDDHHMMSSZ` (UTCTime, years 1950 to 2049) or `YYYYMMDDHHMMSSZ` (GeneralizedTime).
fn parse_time(tag: u8, t: &[u8]) -> Option<i64> {
    let s = std::str::from_utf8(t).ok()?.strip_suffix('Z')?;
    if !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let num = |from: usize, len: usize| s.get(from..from + len)?.parse::<i64>().ok();
    let (year, at) = match (tag, s.len()) {
        (UTC_TIME, 12) => {
            let yy = num(0, 2)?;
            (if yy >= 50 { 1900 + yy } else { 2000 + yy }, 2)
        }
        (GENERALIZED_TIME, 14) => (num(0, 4)?, 4),
        _ => return None,
    };
    let (mo, d, h, mi, sec) = (
        num(at, 2)?,
        num(at + 2, 2)?,
        num(at + 4, 2)?,
        num(at + 6, 2)?,
        num(at + 8, 2)?,
    );
    if !(1..=12).contains(&mo) || !(1..=31).contains(&d) || h > 23 || mi > 59 || sec > 60 {
        return None;
    }
    Some(days_from_civil(year, mo as u32, d as u32) * 86_400 + h * 3600 + mi * 60 + sec)
}

/// Whether a check that saw a certificate expiring at `expires_at` should warn now: the expiry
/// is within `warn_days`, and this certificate has not been warned about. A renewed certificate
/// has a new expiry, so it warns again when its own time comes.
pub fn warning_due(
    expires_at: i64,
    now: i64,
    warn_days: Option<i64>,
    warned_for: Option<i64>,
) -> bool {
    warn_days.is_some_and(|d| expires_at - now <= d * 86_400) && warned_for != Some(expires_at)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Made with `openssl req -x509 -newkey ec ... -days 30` and `-days 12000`; the second ends
    // after 2049, so it carries a GeneralizedTime. The values are from `openssl x509 -enddate`.
    const UTC: &[u8] = include_bytes!("../../../tests/fixtures/cert-utc.der");
    const GENERALIZED: &[u8] = include_bytes!("../../../tests/fixtures/cert-generalized.der");

    #[test]
    fn reads_not_after_in_both_time_forms() {
        assert_eq!(
            not_after(UTC),
            Some(1_795_149_048),
            "2026-11-20 04:30:48 UTC"
        );
        assert_eq!(
            not_after(GENERALIZED),
            Some(2_829_357_048),
            "2059-08-29 04:30:48 UTC"
        );
    }

    #[test]
    fn broken_der_reads_as_none() {
        assert_eq!(not_after(&[]), None);
        assert_eq!(not_after(&UTC[..40]), None);
        assert_eq!(not_after(&[0x30, 0x84, 0xff, 0xff, 0xff, 0xff]), None);
        assert_eq!(parse_time(UTC_TIME, b"261320043048Z"), None, "month 13");
        assert_eq!(parse_time(UTC_TIME, b"2611200430\xc3\xa9Z"), None);
        assert_eq!(
            parse_time(GENERALIZED_TIME, b"261120043048Z"),
            None,
            "wrong length"
        );
    }

    #[test]
    fn utc_time_years_wrap_at_1950() {
        assert_eq!(parse_time(UTC_TIME, b"491231235959Z"), Some(2_524_607_999));
        assert_eq!(parse_time(UTC_TIME, b"500101000000Z"), Some(-631_152_000));
    }

    #[test]
    fn warns_once_per_certificate_inside_the_window() {
        let day = 86_400;
        let expires = 100 * day;
        assert!(!warning_due(expires, 0, None, None), "warning off");
        assert!(
            !warning_due(expires, expires - 15 * day, Some(14), None),
            "outside the window"
        );
        assert!(
            warning_due(expires, expires - 14 * day, Some(14), None),
            "at the edge"
        );
        assert!(
            !warning_due(expires, expires - day, Some(14), Some(expires)),
            "already warned"
        );
        assert!(
            warning_due(expires, expires - day, Some(14), Some(expires - 90 * day)),
            "renewed"
        );
        assert!(
            warning_due(expires, expires + day, Some(7), None),
            "already expired"
        );
    }
}
