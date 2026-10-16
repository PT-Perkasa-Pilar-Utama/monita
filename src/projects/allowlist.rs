//! IP allowlist: comma- or newline-separated IPs and CIDRs (SEC-08).

use std::net::IpAddr;

struct Cidr {
    net: IpAddr,
    prefix: u8,
}

fn parse(entry: &str) -> Option<Cidr> {
    let (ip, prefix) = match entry.split_once('/') {
        Some((ip, p)) => (ip.parse::<IpAddr>().ok()?, Some(p.parse::<u8>().ok()?)),
        None => (entry.parse::<IpAddr>().ok()?, None),
    };
    let max = if ip.is_ipv4() { 32 } else { 128 };
    let prefix = prefix.unwrap_or(max);
    (prefix <= max).then_some(Cidr { net: ip, prefix })
}

fn matches(c: &Cidr, ip: IpAddr) -> bool {
    match (c.net, ip) {
        (IpAddr::V4(n), IpAddr::V4(i)) => {
            let mask = u32::MAX.checked_shl(32 - u32::from(c.prefix)).unwrap_or(0);
            u32::from(n) & mask == u32::from(i) & mask
        }
        (IpAddr::V6(n), IpAddr::V6(i)) => {
            let mask = u128::MAX
                .checked_shl(128 - u32::from(c.prefix))
                .unwrap_or(0);
            u128::from(n) & mask == u128::from(i) & mask
        }
        _ => false,
    }
}

/// Validates and re-joins the list with commas. `Ok(None)` for an empty list (allowlist off).
/// `Err` names the first bad entry.
pub fn normalize(raw: &str) -> Result<Option<String>, String> {
    let entries: Vec<&str> = raw
        .split([',', '\n'])
        .map(str::trim)
        .filter(|e| !e.is_empty())
        .collect();
    if let Some(bad) = entries.iter().find(|e| parse(e).is_none()) {
        return Err((*bad).to_string());
    }
    Ok((!entries.is_empty()).then(|| entries.join(",")))
}

pub fn contains(list: &str, ip: IpAddr) -> bool {
    list.split(',').filter_map(parse).any(|c| matches(&c, ip))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_cidrs_and_single_ips() {
        let list = "10.20.0.0/16,192.168.1.7,fd00::/8";
        assert!(contains(list, "10.20.5.9".parse().unwrap()));
        assert!(!contains(list, "10.21.0.1".parse().unwrap()));
        assert!(contains(list, "192.168.1.7".parse().unwrap()));
        assert!(!contains(list, "192.168.1.8".parse().unwrap()));
        assert!(contains(list, "fd12::1".parse().unwrap()));
        assert!(contains("0.0.0.0/0", "8.8.8.8".parse().unwrap()));
    }

    #[test]
    fn normalize_rejects_bad_entries_and_empties_to_off() {
        assert_eq!(
            normalize(" 10.0.0.0/8 ,\n1.2.3.4 ").unwrap().as_deref(),
            Some("10.0.0.0/8,1.2.3.4")
        );
        assert_eq!(normalize("  ").unwrap(), None);
        assert_eq!(normalize("10.0.0.0/33").unwrap_err(), "10.0.0.0/33");
        assert_eq!(normalize("intranet").unwrap_err(), "intranet");
    }
}
