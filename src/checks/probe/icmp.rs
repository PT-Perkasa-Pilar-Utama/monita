//! A ping check (v1.1 Q24): one ICMP echo request through an unprivileged datagram socket. No raw
//! socket and no capability. Linux allows it when `net.ipv4.ping_group_range` covers Monita's
//! group (systemd 243+ and Docker set `0 2147483647`); macOS allows it for any user; Windows has
//! no such socket, so the form refuses ping there.

use std::net::IpAddr;
use std::time::{Duration, Instant};

use super::failed;
use crate::checks::CheckResult;
use crate::platform::now;

/// The check's error when the system refuses the socket. It names the fix.
pub const REFUSED: &str = "This server does not allow ping. Set the sysctl net.ipv4.ping_group_range to \"0 2147483647\", or use a TCP check.";

/// The form's error on Windows, and the check's if one ever runs there.
pub const NO_WINDOWS: &str = "Ping checks need Linux or macOS. Use a TCP check on Windows.";

// ponytail: one echo per check; send 3 and pass on the first reply if single-packet loss proves noisy.
pub async fn probe_ping(host: &str, timeout_ms: i64) -> CheckResult {
    let checked_at = now();
    let start = Instant::now();
    let limit = Duration::from_millis(timeout_ms as u64);
    let ip = match tokio::time::timeout(limit, tokio::net::lookup_host((host, 0))).await {
        Ok(Ok(mut addrs)) => match addrs.next() {
            Some(a) => a.ip(),
            None => return failed(checked_at, start, "No address in the answer".into()),
        },
        Ok(Err(e)) => return failed(checked_at, start, format!("Lookup failed: {e}")),
        Err(_) => return failed(checked_at, start, format!("Timeout after {timeout_ms} ms")),
    };
    let left = limit.saturating_sub(start.elapsed());
    match tokio::task::spawn_blocking(move || echo(ip, left, timeout_ms)).await {
        Ok(Ok(rtt)) => CheckResult {
            checked_at,
            ok: true,
            status_code: None,
            latency_ms: rtt.as_millis() as i64,
            error: None,
            cert_expires_at: None,
        },
        Ok(Err(e)) => failed(checked_at, start, e),
        Err(e) => failed(checked_at, start, format!("Ping failed: {e}")),
    }
}

#[cfg(windows)]
fn echo(_ip: IpAddr, _limit: Duration, _timeout_ms: i64) -> Result<Duration, String> {
    Err(NO_WINDOWS.into())
}

/// Sends one echo request and waits up to `limit` for its reply. Returns the round trip.
/// `timeout_ms` is the monitor's whole timeout, for the error.
#[cfg(not(windows))]
fn echo(ip: IpAddr, limit: Duration, timeout_ms: i64) -> Result<Duration, String> {
    use socket2::{Domain, Protocol, Socket, Type};
    let (domain, protocol, request, reply) = match ip {
        IpAddr::V4(_) => (Domain::IPV4, Protocol::ICMPV4, 8, 0),
        IpAddr::V6(_) => (Domain::IPV6, Protocol::ICMPV6, 128, 129),
    };
    let socket = Socket::new(domain, Type::DGRAM, Some(protocol)).map_err(|e| {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            REFUSED.to_string()
        } else {
            format!("Ping failed: {e}")
        }
    })?;
    let socket = std::net::UdpSocket::from(socket);
    let token = next_token();
    let packet = echo_request(request, token, ip.is_ipv4());
    let sent = Instant::now();
    socket
        .send_to(&packet, (ip, 0))
        .map_err(|e| format!("Ping failed: {e}"))?;
    let mut buf = [0u8; 1500];
    loop {
        let left = limit.saturating_sub(sent.elapsed());
        if left.is_zero() {
            return Err(format!("No reply within {timeout_ms} ms"));
        }
        socket
            .set_read_timeout(Some(left))
            .map_err(|e| format!("Ping failed: {e}"))?;
        match socket.recv(&mut buf) {
            Ok(n) if is_reply(&buf[..n], reply, token) => return Ok(sent.elapsed()),
            Ok(_) => continue,
            Err(e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) =>
            {
                return Err(format!("No reply within {timeout_ms} ms"));
            }
            Err(e) => return Err(format!("Ping failed: {e}")),
        }
    }
}

/// A token unique within this process: the echo's sequence number and its 8-byte payload. Linux
/// replaces the identifier with its own, so a reply is matched on these instead.
#[cfg(not(windows))]
fn next_token() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    ((now() as u64) << 20) ^ n
}

/// An echo request: type, code 0, checksum, identifier 0, sequence, then the token. ICMPv6
/// checksums cover a pseudo-header the kernel fills in, so only ICMPv4 computes its own.
#[cfg_attr(windows, allow(dead_code))]
fn echo_request(kind: u8, token: u64, v4: bool) -> [u8; 16] {
    let mut p = [0u8; 16];
    p[0] = kind;
    p[6..8].copy_from_slice(&(token as u16).to_be_bytes());
    p[8..16].copy_from_slice(&token.to_be_bytes());
    if v4 {
        let sum = checksum(&p);
        p[2..4].copy_from_slice(&sum.to_be_bytes());
    }
    p
}

/// The internet checksum: ones' complement of the ones' complement sum of 16-bit words.
#[cfg_attr(windows, allow(dead_code))]
fn checksum(b: &[u8]) -> u16 {
    let mut sum: u32 = b
        .chunks(2)
        .map(|w| u32::from(u16::from_be_bytes([w[0], *w.get(1).unwrap_or(&0)])))
        .sum();
    while sum > 0xffff {
        sum = (sum & 0xffff) + (sum >> 16);
    }
    !(sum as u16)
}

/// An echo reply that carries `token`. macOS hands an ICMPv4 datagram socket the IP header too;
/// Linux does not. A version nibble of 4 means the header is there, and it is skipped.
#[cfg_attr(windows, allow(dead_code))]
fn is_reply(b: &[u8], reply: u8, token: u64) -> bool {
    let b = match b.first() {
        Some(&first) if first >> 4 == 4 => {
            b.get(usize::from(first & 0x0f) * 4..).unwrap_or_default()
        }
        _ => b,
    };
    b.len() >= 16
        && b[0] == reply
        && b[6..8] == (token as u16).to_be_bytes()
        && b[8..16] == token.to_be_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_echo_request_carries_its_token_and_a_valid_checksum() {
        let p = echo_request(8, 0x0102_0304_0506_0708, true);
        assert_eq!(p[0], 8);
        assert_eq!(&p[6..8], &[0x07, 0x08]);
        assert_eq!(checksum(&p), 0, "a packet with its checksum sums to zero");
        let v6 = echo_request(128, 7, false);
        assert_eq!(&v6[2..4], &[0, 0], "the kernel fills the ICMPv6 checksum");
    }

    #[test]
    fn a_reply_matches_with_or_without_the_ip_header() {
        let token = 42;
        let mut reply = echo_request(0, token, true);
        reply[4] = 0x99; // Linux rewrites the identifier
        assert!(is_reply(&reply, 0, token));
        let mut with_header = vec![0x45; 1];
        with_header.extend_from_slice(&[0; 19]);
        with_header.extend_from_slice(&reply);
        assert!(is_reply(&with_header, 0, token));
        assert!(!is_reply(&reply, 0, token + 1), "another check's reply");
        assert!(
            !is_reply(&echo_request(8, token, true), 0, token),
            "our own request"
        );
        assert!(!is_reply(&reply[..10], 0, token));
        assert!(
            !is_reply(&[0x4f], 0, token),
            "a header longer than the datagram"
        );
    }
}
