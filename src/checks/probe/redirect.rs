//! Where a check may follow a redirect when it carries the monitor's own headers: a custom
//! header can hold a sealed project secret (SEC-23), and reqwest strips only `Authorization` and
//! `Cookie` when a redirect leaves the host.

use reqwest::{Url, redirect};

/// Redirects one check follows, as with `Policy::limited(5)`.
const MAX_HOPS: usize = 5;

/// Whether a redirect from the monitored URL to `to` keeps the request on the same server: the
/// same host and port, or the same host moving from http on 80 to https on 443. Anything else,
/// including https to http, would hand the headers to another party or send them in clear.
pub fn same_server(from: &Url, to: &Url) -> bool {
    if from.host_str() != to.host_str() {
        return false;
    }
    let (fs, ts) = (from.scheme(), to.scheme());
    let (fp, tp) = (from.port_or_known_default(), to.port_or_known_default());
    (fs == ts && fp == tp) || (fs == "http" && ts == "https" && fp == Some(80) && tp == Some(443))
}

/// Follows redirects that stay on the monitored server and stops at the first that leaves it, so
/// the check answers with that redirect instead of sending the headers on.
pub fn guarded() -> redirect::Policy {
    redirect::Policy::custom(|attempt| {
        if attempt.previous().len() > MAX_HOPS {
            return attempt.error("too many redirects");
        }
        let stays = attempt
            .previous()
            .first()
            .is_some_and(|first| same_server(first, attempt.url()));
        if stays {
            attempt.follow()
        } else {
            attempt.stop()
        }
    })
}

/// The check's error when it stopped at a redirect to another server.
pub fn left_server(resp: &reqwest::Response) -> Option<String> {
    if !resp.status().is_redirection() {
        return None;
    }
    let location = resp
        .headers()
        .get(reqwest::header::LOCATION)?
        .to_str()
        .ok()?;
    let to = resp.url().join(location).ok()?;
    if same_server(resp.url(), &to) {
        return None;
    }
    let host = to.host_str().unwrap_or("another host");
    Some(format!(
        "Redirected to {host}, another server, so Monita did not send the monitor's headers there"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn u(s: &str) -> Url {
        Url::parse(s).unwrap()
    }

    #[test]
    fn only_the_same_server_or_an_upgrade_to_https_keeps_the_headers() {
        assert!(same_server(&u("http://a.test/x"), &u("http://a.test/y")));
        assert!(same_server(
            &u("https://a.test/x"),
            &u("https://a.test:443/y")
        ));
        assert!(same_server(&u("http://a.test/x"), &u("https://a.test/y")));
        assert!(
            !same_server(&u("https://a.test/x"), &u("http://a.test/y")),
            "a downgrade"
        );
        assert!(!same_server(&u("http://a.test/x"), &u("http://b.test/y")));
        assert!(!same_server(
            &u("http://a.test:8080/x"),
            &u("http://a.test:8081/y")
        ));
        assert!(!same_server(
            &u("http://a.test:8080/x"),
            &u("https://a.test/y")
        ));
    }
}
