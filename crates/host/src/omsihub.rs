//! Signing in to an Omsi-Hub account with Google or Discord (the companion's start screen and Profile page), as
//! Omsi-Hub's own app does it (its `hub-online/src/oauth.rs`): the player's browser does the provider's sign-in on
//! Omsi-Hub's Supabase Auth and hands the result back to a one-shot listener on this computer
//! (`http://127.0.0.1:<port>/callback`, a free port the system chooses), never to a page on the web.
//!
//! PKCE: the companion makes the verifier and keeps it; only its challenge and the flow's `state` come here. The
//! code the browser brings back goes to the companion, which exchanges it with the verifier
//! (`/auth/v1/token?grant_type=pkce`); a code without its verifier is worth nothing, and an answer whose `state`
//! isn't this flow's is turned away. The listener answers only `GET /callback` on `127.0.0.1:<its port>` (the
//! `Host` checked), shows a small page, and stops at the first answer, after five minutes, or when cancelled.
//! Nothing of it is logged.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// Omsi-Hub's Supabase project (public, as every client of it uses it).
const AUTH: &str = "https://mlatjtjjdfifbgddcxzu.supabase.co/auth/v1/authorize";
const PATH: &str = "/callback";
const TIMEOUT: Duration = Duration::from_secs(5 * 60);
const MAX_REQUEST: usize = 16 * 1024;

/// Who signs the player in, in the browser.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Google,
    Discord,
}

impl Provider {
    fn name(self) -> &'static str {
        match self {
            Provider::Google => "google",
            Provider::Discord => "discord",
        }
    }
}

/// Signs in with `provider` in the player's browser and waits for it: the code it brought back, or why not
/// (`cancelled`: the player said no there or `cancel` was set; `timeout`; `failed`).
pub fn sign_in(provider: Provider, challenge: &str, state: &str, cancel: &AtomicBool) -> Result<String, &'static str> {
    // The challenge is base64url(sha256) (43 characters), the state the companion's random base64url.
    if challenge.len() != 43 || !base64url(challenge) || !(16..=128).contains(&state.len()) || !base64url(state) {
        return Err("failed");
    }
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|_| "failed")?;
    listener.set_nonblocking(true).map_err(|_| "failed")?;
    let port = listener.local_addr().map_err(|_| "failed")?.port();
    let url = format!(
        "{AUTH}?provider={}&redirect_to={}&code_challenge={challenge}&code_challenge_method=s256",
        provider.name(),
        encode(&format!("http://127.0.0.1:{port}{PATH}?state={state}")),
    );
    crate::browser::open(&url);

    let end = Instant::now() + TIMEOUT;
    loop {
        if cancel.load(Ordering::Relaxed) {
            return Err("cancelled");
        }
        if Instant::now() >= end {
            return Err("timeout");
        }
        match listener.accept() {
            Ok((stream, peer)) if peer.ip().is_loopback() => {
                if let Some(result) = serve(stream, port, state) {
                    return result;
                }
            }
            Ok(_) => {}
            Err(_) => std::thread::sleep(Duration::from_millis(100)),
        }
    }
}

/// One request: Some(the result) when it ended the flow, None to wait on.
fn serve(mut stream: TcpStream, port: u16, state: &str) -> Option<Result<String, &'static str>> {
    let _ = stream.set_nonblocking(false);
    let _ = stream.set_read_timeout(Some(Duration::from_secs(3)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));
    let Some(request) = read_request(&mut stream) else {
        reply(&mut stream, 400, "text/plain", "Bad request");
        return None;
    };
    // Only this listener's own address: a page that rebinds a name to 127.0.0.1 is turned away.
    if request.host != format!("127.0.0.1:{port}") {
        reply(&mut stream, 400, "text/plain", "Bad request");
        return None;
    }
    let (path, query) = request.target.split_once('?').unwrap_or((&request.target, ""));
    if path != PATH || request.method != "GET" {
        reply(&mut stream, 404, "text/plain", "Not found");
        return None;
    }
    let pairs = form(query);
    if !same(value(&pairs, "state").unwrap_or(""), state) {
        page(&mut stream, false, "Please return to Euro-Hub and try again.");
        return None;
    }
    if let Some(code) = value(&pairs, "code").filter(|code| !code.is_empty()) {
        page(&mut stream, true, "You may close this tab and return to Euro-Hub.");
        return Some(Ok(code.to_string()));
    }
    let cancelled = value(&pairs, "error") == Some("access_denied");
    page(
        &mut stream,
        false,
        if cancelled { "The sign-in was cancelled. Please return to Euro-Hub." } else { "Please return to Euro-Hub and try again." },
    );
    Some(Err(if cancelled { "cancelled" } else { "failed" }))
}

fn page(stream: &mut TcpStream, ok: bool, text: &str) {
    let (title, mark) = if ok {
        (
            "You are signed in to Euro-Hub",
            r##"<svg viewBox="0 0 24 24" width="40" height="40" aria-hidden="true"><circle cx="12" cy="12" r="11" fill="#2a75f7"/><path d="M7 12.5l3.2 3.2L17.5 8.5" fill="none" stroke="#fff" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"/></svg>"##,
        )
    } else {
        (
            "The sign-in could not be completed",
            r##"<svg viewBox="0 0 24 24" width="40" height="40" aria-hidden="true"><circle cx="12" cy="12" r="11" fill="#d93a30"/><path d="M8.5 8.5l7 7M15.5 8.5l-7 7" fill="none" stroke="#fff" stroke-width="2.4" stroke-linecap="round"/></svg>"##,
        )
    };
    let body = format!(
        r#"<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><meta name="referrer" content="no-referrer"><title>Euro-Hub</title><style>{CSS}</style></head><body><main><p class="brand">Euro-<i>Hub</i></p>{mark}<h1>{title}</h1><p>{text}</p></main></body></html>"#
    );
    reply(stream, 200, "text/html; charset=utf-8", &body);
}

const CSS: &str = "html{color-scheme:dark}body{margin:0;min-height:100vh;display:flex;align-items:center;justify-content:center;background:#0a142e;color:#e8ebf2;font:15px/1.5 system-ui,-apple-system,'Segoe UI',sans-serif}main{width:min(420px,calc(100% - 32px));padding:28px 24px;border-radius:14px;background:#141a26;border:1px solid #232b3b;text-align:center;box-shadow:0 0 24px rgba(42,117,247,.45)}.brand{margin:0 0 16px;font-weight:700;font-size:20px}.brand i{color:#f58620;font-weight:800}h1{margin:14px 0 6px;font-size:18px}p{margin:0;color:#959db0}";

struct Request {
    method: String,
    target: String,
    host: String,
}

fn read_request(stream: &mut TcpStream) -> Option<Request> {
    let mut buffer = Vec::new();
    let mut chunk = [0u8; 2048];
    let head_end = loop {
        if let Some(at) = buffer.windows(4).position(|w| w == b"\r\n\r\n") {
            break at;
        }
        if buffer.len() > MAX_REQUEST {
            return None;
        }
        let read = stream.read(&mut chunk).ok()?;
        if read == 0 {
            return None;
        }
        buffer.extend_from_slice(&chunk[..read]);
    };
    let head = std::str::from_utf8(&buffer[..head_end]).ok()?;
    let mut lines = head.split("\r\n");
    let mut first = lines.next()?.split(' ');
    let (method, target) = (first.next()?.to_string(), first.next()?.to_string());
    let host = lines
        .filter_map(|line| line.split_once(':'))
        .find(|(key, _)| key.trim().eq_ignore_ascii_case("host"))
        .map(|(_, value)| value.trim().to_string())
        .unwrap_or_default();
    Some(Request { method, target, host })
}

fn reply(stream: &mut TcpStream, status: u16, kind: &str, body: &str) {
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        _ => "Not Found",
    };
    let head = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nReferrer-Policy: no-referrer\r\nX-Content-Type-Options: nosniff\r\nContent-Security-Policy: default-src 'none'; style-src 'unsafe-inline'; form-action 'none'; frame-ancestors 'none'\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(head.as_bytes()).and_then(|_| stream.write_all(body.as_bytes())).and_then(|_| stream.flush());
}

fn base64url(text: &str) -> bool {
    text.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
}

/// Percent-encodes everything but the unreserved characters.
fn encode(text: &str) -> String {
    text.bytes()
        .map(|c| match c {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (c as char).to_string(),
            _ => format!("%{c:02X}"),
        })
        .collect()
}

/// `a=1&b=2` as pairs, percent-decoded.
fn form(text: &str) -> Vec<(String, String)> {
    text.split('&')
        .filter(|pair| !pair.is_empty())
        .map(|pair| {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            (decode(key), decode(value))
        })
        .collect()
}

fn value<'a>(pairs: &'a [(String, String)], name: &str) -> Option<&'a str> {
    pairs.iter().find(|(key, _)| key == name).map(|(_, value)| value.as_str())
}

fn decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        match bytes[at] {
            b'+' => out.push(b' '),
            b'%' if at + 2 < bytes.len() => match (hex(bytes[at + 1]), hex(bytes[at + 2])) {
                (Some(high), Some(low)) => {
                    out.push(high << 4 | low);
                    at += 2;
                }
                _ => out.push(b'%'),
            },
            c => out.push(c),
        }
        at += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex(c: u8) -> Option<u8> {
    (c as char).to_digit(16).map(|digit| digit as u8)
}

/// The same text, compared in a time that doesn't tell how much of it matched.
fn same(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0u8, |d, (x, y)| d | (x ^ y)) == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queries_are_encoded_and_decoded() {
        assert_eq!(encode("http://127.0.0.1:5/callback?state=a-b"), "http%3A%2F%2F127.0.0.1%3A5%2Fcallback%3Fstate%3Da-b");
        let pairs = form("a=1%2B2&b=x+y&c&d=%zz");
        assert_eq!(value(&pairs, "a"), Some("1+2"));
        assert_eq!(value(&pairs, "b"), Some("x y"));
        assert_eq!(value(&pairs, "c"), Some(""));
        assert_eq!(value(&pairs, "d"), Some("%zz"));
        assert!(same("abc", "abc") && !same("abc", "abd") && !same("abc", "ab"));
    }

    #[test]
    fn only_base64url_challenges_and_states_are_taken() {
        let cancel = AtomicBool::new(true);
        assert_eq!(sign_in(Provider::Google, "short", "statestatestatestate", &cancel), Err("failed"));
        assert_eq!(sign_in(Provider::Google, &"a".repeat(42).replace('a', "&"), "statestatestatestate", &cancel), Err("failed"));
        assert_eq!(sign_in(Provider::Discord, &"a".repeat(43), "bad state with spaces!", &cancel), Err("failed"));
    }
}
