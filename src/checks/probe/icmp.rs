//! A ping check (v1.1 Q24): one ICMP echo request through an unprivileged datagram socket. No raw
//! socket and no capability. Linux allows it when `net.ipv4.ping_group_range` covers Monita's
//! group (systemd 243+ and Docker set `0 2147483647`); macOS allows it for any user. Where the
//! socket is refused, and on Windows, which has no such socket, the system `ping` sends the echo.

use std::net::IpAddr;
use std::time::{Duration, Instant};

use super::failed;
use crate::checks::CheckResult;
use crate::platform::now;

/// The check's error when the system refuses the socket and has no `ping` to run. It names the fix.
pub const REFUSED: &str = "This server does not allow ping, so set the sysctl net.ipv4.ping_group_range to \"0 2147483647\" or use a TCP check.";

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
    #[cfg(windows)]
    let sent = system_ping(ip, left, timeout_ms).await;
    #[cfg(not(windows))]
    let sent = match tokio::task::spawn_blocking(move || echo(ip, left, timeout_ms)).await {
        Ok(Err(e)) if e == REFUSED => system_ping(ip, left, timeout_ms).await,
        Ok(r) => r,
        Err(e) => Err(format!("Ping failed: {e}")),
    };
    match sent {
        Ok(rtt) => CheckResult {
            checked_at,
            ok: true,
            status_code: None,
            latency_ms: rtt.as_millis() as i64,
            error: None,
            cert_expires_at: None,
        },
        Err(e) => failed(checked_at, start, e),
    }
}

/// One echo through the system `ping`, which may hold a privilege Monita lacks. A reply is a line
/// with `TTL=`: Windows `ping` exits 0 on "Destination host unreachable" too. A missing `ping`
/// reads as the refused socket.
async fn system_ping(ip: IpAddr, limit: Duration, timeout_ms: i64) -> Result<Duration, String> {
    let ms = limit.as_millis().max(1) as u64;
    let mut cmd = tokio::process::Command::new("ping");
    if cfg!(windows) {
        cmd.args(["-n", "1", "-w", &ms.to_string()]);
    } else {
        // iputils and busybox take whole seconds.
        cmd.args(["-n", "-c", "1", "-W", &ms.div_ceil(1000).to_string()]);
        if ip.is_ipv6() {
            cmd.arg("-6");
        }
    }
    cmd.arg(ip.to_string()).kill_on_drop(true);
    #[cfg(windows)]
    cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW: a service shows no console.
    let sent = Instant::now();
    let out = match tokio::time::timeout(limit + Duration::from_secs(1), cmd.output()).await {
        Ok(Ok(out)) => out,
        Ok(Err(_)) => return Err(REFUSED.into()),
        Err(_) => return Err(format!("No reply within {timeout_ms} ms")),
    };
    if let Some(rtt) = reply_time(&String::from_utf8_lossy(&out.stdout)) {
        return Ok(rtt.unwrap_or_else(|| sent.elapsed()));
    }
    let err = String::from_utf8_lossy(&out.stderr).to_ascii_lowercase();
    if err.contains("permission") || err.contains("not permitted") {
        return Err(REFUSED.into());
    }
    Err(format!("No reply within {timeout_ms} ms"))
}

/// The round trip in `ping`'s output: `None` without a reply, `Some(None)` for a reply whose time
/// does not read. `time=12.3 ms` (Linux), `Zeit=12ms` and `time<1ms` (Windows, any language).
fn reply_time(out: &str) -> Option<Option<Duration>> {
    let line = out
        .lines()
        .find(|l| l.to_ascii_lowercase().contains("ttl="))?;
    let ms = line.char_indices().find_map(|(i, ch)| {
        if ch != '=' && ch != '<' {
            return None;
        }
        let rest = &line[i + 1..];
        let num: String = rest
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == '.')
            .collect();
        let after = rest[num.len()..].trim_start();
        (!num.is_empty() && after.starts_with("ms"))
            .then(|| num.parse::<f64>().ok())
            .flatten()
    });
    Some(ms.map(|ms| Duration::from_micros((ms * 1000.0) as u64)))
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
    fn a_system_ping_reply_is_a_ttl_line_and_its_time_reads_in_any_language() {
        let ms = |s: &str| reply_time(s).map(|t| t.map(|d| d.as_micros()));
        let linux = "PING 1.1.1.1 (1.1.1.1) 56(84) bytes of data.\n64 bytes from 1.1.1.1: icmp_seq=1 ttl=57 time=12.3 ms\n";
        assert_eq!(ms(linux), Some(Some(12_300)));
        assert_eq!(
            ms("Reply from 10.0.0.5: bytes=32 time<1ms TTL=128"),
            Some(Some(1_000))
        );
        assert_eq!(
            ms("Antwort von 10.0.0.5: Bytes=32 Zeit=14ms TTL=117"),
            Some(Some(14_000))
        );
        assert_eq!(
            ms("Respuesta desde 10.0.0.5: bytes=32 tiempo=7ms TTL=64"),
            Some(Some(7_000))
        );
        assert_eq!(
            ms("Reply from 10.0.0.1: Destination host unreachable."),
            None,
            "an unreachable answer is no reply"
        );
        assert_eq!(ms("Request timed out."), None);
        assert_eq!(
            ms("Reply from 10.0.0.5: TTL=64"),
            Some(None),
            "a reply without a time"
        );
    }

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
