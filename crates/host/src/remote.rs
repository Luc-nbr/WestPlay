//! The iPad: WestPlay's screen on an iPad on the player's Wi-Fi, as a web app in Safari. A small web server on the
//! local network serves it the cabin page (the same as the truck's, laid out as the tablet's), the map and the
//! player's pictures, and a WebSocket carries the host's messages to it and its messages back, as WebView2's do
//! for the truck's page. While it's connected the iPad is the head unit (`lib.rs`).
//!
//! Off until the player switches it on in the companion. A device must be paired first: the code the companion
//! shows, entered once on the iPad, gives it a token kept as a cookie; nothing but the page's own files is served
//! without one. Plain HTTP on the local network (an iPad won't trust a certificate WestPlay could make).
//!
//! Threads: one accepts, one per connection (HTTP requests kept alive, or a WebSocket's reader), one writes to the
//! WebSocket. The host's loop only puts messages in and takes them out.

use std::collections::{BTreeSet, HashMap};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream, UdpSocket};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use base64::Engine;
use serde::{Deserialize, Serialize};

/// The port the iPad opens WestPlay at.
pub const PORT: u16 = 8737;
/// How long the iPad may be gone (locked, Wi-Fi hiccup) before the truck's screen is the player's again.
pub const GRACE: Duration = Duration::from_secs(10);
const COOKIE: &str = "westplay";
const MOST_HEAD: usize = 16 << 10;
const MOST_MESSAGE: usize = 16 << 20;

/// What the companion shows of it.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IpadStatus {
    /// Where to open WestPlay on the iPad (`http://192.168.1.20:8737`).
    pub address: String,
    pub code: String,
    /// The address with the code in it: opened (the companion's QR code), it pairs the iPad at once.
    pub link: String,
    pub connected: bool,
    /// The iPad page's frames a second, as it last said (`None`: not yet).
    pub fps: Option<u32>,
}

#[derive(Default, Serialize, Deserialize)]
struct Pairing {
    code: String,
    tokens: BTreeSet<String>,
}

enum Out {
    Text(String),
    Binary(Vec<u8>),
    Pong(Vec<u8>),
}

struct Shared {
    stop: AtomicBool,
    incoming: Mutex<Vec<String>>,
    client: Mutex<Option<(u64, Sender<Out>)>>,
    next_client: AtomicU64,
    /// When the last iPad went (`None` while one is connected, or none ever was).
    gone_at: Mutex<Option<Instant>>,
    pairing: Mutex<Pairing>,
    pairing_file: PathBuf,
    wrong_codes: AtomicU32,
    fps: AtomicU32,
    ui_root: PathBuf,
    /// The folders served beside the page, by their path's first part (`maps`), as WebView2's virtual hosts.
    folders: Vec<(String, PathBuf)>,
}

pub struct Remote {
    shared: Arc<Shared>,
    address: String,
}

impl Drop for Remote {
    fn drop(&mut self) {
        self.shared.stop.store(true, Ordering::Release);
        // The WebSocket's writer ends, and with it the connection.
        self.shared.client.lock().unwrap_or_else(|e| e.into_inner()).take();
    }
}

impl Remote {
    /// Serves the page in `ui_root` and `folders` on the local network.
    pub fn start(data_dir: &Path, ui_root: PathBuf, folders: Vec<(String, PathBuf)>) -> io::Result<Self> {
        let pairing_file = data_dir.join("ipad.json");
        let mut pairing: Pairing =
            std::fs::read_to_string(&pairing_file).ok().and_then(|json| serde_json::from_str(&json).ok()).unwrap_or_default();
        if pairing.code.len() != 6 {
            pairing.code = format!("{:06}", random() % 1_000_000);
            save_pairing(&pairing_file, &pairing);
        }
        let listener = TcpListener::bind(("0.0.0.0", PORT))?;
        listener.set_nonblocking(true)?;
        let shared = Arc::new(Shared {
            stop: AtomicBool::new(false),
            incoming: Mutex::new(Vec::new()),
            client: Mutex::new(None),
            next_client: AtomicU64::new(1),
            gone_at: Mutex::new(None),
            pairing: Mutex::new(pairing),
            pairing_file,
            wrong_codes: AtomicU32::new(0),
            fps: AtomicU32::new(0),
            ui_root,
            folders,
        });
        let accepting = shared.clone();
        std::thread::Builder::new().name("westplay-ipad".into()).spawn(move || accept(listener, accepting))?;
        let address = format!("http://{}:{PORT}", local_address().unwrap_or_else(|| "localhost".into()));
        log!("iPad: serving at {address}");
        Ok(Self { shared, address })
    }

    pub fn status(&self) -> IpadStatus {
        let code = self.shared.pairing.lock().unwrap_or_else(|e| e.into_inner()).code.clone();
        IpadStatus {
            address: self.address.clone(),
            link: format!("{}/?pair={code}", self.address),
            code,
            connected: self.connected(),
            fps: self.connected().then(|| self.shared.fps.load(Ordering::Acquire)).filter(|fps| *fps > 0),
        }
    }

    pub fn connected(&self) -> bool {
        self.shared.client.lock().unwrap_or_else(|e| e.into_inner()).is_some()
    }

    /// Whether the iPad is the head unit: connected, or gone less than [`GRACE`] ago.
    pub fn head_unit(&self) -> bool {
        self.connected()
            || self.shared.gone_at.lock().unwrap_or_else(|e| e.into_inner()).is_some_and(|at| at.elapsed() < GRACE)
    }

    /// A host message for the iPad's page (nothing without one).
    pub fn post(&self, json: &str) {
        self.send(Out::Text(json.to_owned()));
    }

    /// A binary message for the iPad's page (a player's picture: `lib.rs`).
    pub fn post_binary(&self, bytes: Vec<u8>) {
        self.send(Out::Binary(bytes));
    }

    fn send(&self, out: Out) {
        if let Some((_, sender)) = &*self.shared.client.lock().unwrap_or_else(|e| e.into_inner()) {
            let _ = sender.send(out);
        }
    }

    /// The iPad page's frames a second, as it says (`ipadFps`).
    pub fn set_fps(&self, fps: u32) {
        self.shared.fps.store(fps, Ordering::Release);
    }

    /// The messages the iPad's page posted since last taken.
    pub fn take_messages(&self) -> Vec<String> {
        std::mem::take(&mut *self.shared.incoming.lock().unwrap_or_else(|e| e.into_inner()))
    }
}

fn save_pairing(file: &Path, pairing: &Pairing) {
    if let Err(e) = std::fs::write(file, serde_json::to_string(pairing).expect("pairing serializes")) {
        log!("iPad: cannot write {}: {e}", file.display());
    }
}

/// 64 random bits from the OS (std's hasher keys are seeded from it).
fn random() -> u64 {
    use std::hash::{BuildHasher, Hasher};
    let mut hasher = std::collections::hash_map::RandomState::new().build_hasher();
    hasher.write_u128(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos());
    hasher.finish()
}

/// This PC's address on the local network (the route out's; nothing is sent).
fn local_address() -> Option<String> {
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("192.0.2.1:80").ok()?;
    Some(socket.local_addr().ok()?.ip().to_string())
}

fn accept(listener: TcpListener, shared: Arc<Shared>) {
    while !shared.stop.load(Ordering::Acquire) {
        match listener.accept() {
            Ok((stream, _)) => {
                let shared = shared.clone();
                let _ = std::thread::Builder::new().name("westplay-ipad-connection".into()).spawn(move || {
                    let _ = stream.set_nonblocking(false);
                    if let Err(e) = serve(stream, &shared) {
                        if e.kind() != io::ErrorKind::UnexpectedEof && e.kind() != io::ErrorKind::ConnectionReset {
                            log!("iPad: connection ended: {e}");
                        }
                    }
                });
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => std::thread::sleep(Duration::from_millis(100)),
            Err(e) => {
                log!("iPad: accept failed: {e}");
                std::thread::sleep(Duration::from_secs(1));
            }
        }
    }
}

struct Request {
    path: String,
    query: HashMap<String, String>,
    headers: HashMap<String, String>,
}

impl Request {
    fn header(&self, name: &str) -> &str {
        self.headers.get(name).map_or("", String::as_str)
    }

    fn cookie(&self) -> Option<&str> {
        self.header("cookie").split(';').find_map(|part| part.trim().strip_prefix(COOKIE)?.strip_prefix('='))
    }
}

fn read_request(reader: &mut BufReader<TcpStream>) -> io::Result<Option<Request>> {
    let mut line = String::new();
    if reader.read_line(&mut line)? == 0 {
        return Ok(None);
    }
    let mut parts = line.split_whitespace();
    let (_method, target) = (parts.next().unwrap_or(""), parts.next().unwrap_or("/").to_owned());
    let mut headers = HashMap::new();
    let mut read = line.len();
    loop {
        line.clear();
        read += reader.read_line(&mut line)?;
        if read > MOST_HEAD {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "request head too large"));
        }
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            break;
        }
        if let Some((name, value)) = trimmed.split_once(':') {
            headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_owned());
        }
    }
    let (path, query) = target.split_once('?').unwrap_or((&target, ""));
    let query = query
        .split('&')
        .filter_map(|pair| pair.split_once('=').or(Some((pair, ""))))
        .map(|(key, value)| (decode(key), decode(value)))
        .collect();
    Ok(Some(Request { path: decode(path), query, headers }))
}

fn decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let Ok(byte) = u8::from_str_radix(text.get(i + 1..i + 3).unwrap_or(""), 16)
        {
            out.push(byte);
            i += 3;
        } else {
            out.push(if bytes[i] == b'+' { b' ' } else { bytes[i] });
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn serve(stream: TcpStream, shared: &Shared) -> io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(60)))?;
    // Safari tries an address typed without `http://` over HTTPS first: refused at once (a TLS hello starts with
    // 0x16), it goes on to plain HTTP rather than waiting on an answer that won't come.
    let mut first = [0u8; 1];
    if stream.peek(&mut first)? == 0 || first[0] == 0x16 {
        return Ok(());
    }
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut stream = stream;
    while let Some(request) = read_request(&mut reader)? {
        let paired = request.cookie().is_some_and(|token| {
            shared.pairing.lock().unwrap_or_else(|e| e.into_inner()).tokens.contains(token)
        });
        if request.header("upgrade").eq_ignore_ascii_case("websocket") {
            if !paired {
                return respond(&mut stream, "401 Unauthorized", "text/plain", b"not paired", &[]);
            }
            return websocket(stream, reader, &request, shared);
        }
        match request.path.as_str() {
            "/paired" => {
                let (status, body) = if paired { ("200 OK", "yes") } else { ("401 Unauthorized", "no") };
                respond(&mut stream, status, "text/plain", body.as_bytes(), &[])?;
            }
            "/pair" => pair(&mut stream, &request, shared)?,
            "/manifest.webmanifest" => {
                let manifest = serde_json::json!({
                    "name": "Euro-Hub", "short_name": "Euro-Hub", "start_url": "/", "display": "fullscreen",
                    "orientation": "landscape", "background_color": "#000000", "theme_color": "#0A1F44",
                    "icons": [{ "src": "/dashly.png", "sizes": "512x512", "type": "image/png" }],
                });
                respond(&mut stream, "200 OK", "application/manifest+json", manifest.to_string().as_bytes(), &[])?;
            }
            // Opened from the companion's QR code: paired at once, and on to the page.
            "/" if request.query.contains_key("pair") => {
                let code = request.query.get("pair").map_or("", |code| code.trim());
                match new_token(shared, code) {
                    Some(token) => {
                        let cookie = cookie(&token);
                        respond(&mut stream, "302 Found", "text/plain", b"paired", &["Location: /", &cookie])?;
                    }
                    None => respond(&mut stream, "302 Found", "text/plain", b"wrong code", &["Location: /"])?,
                }
            }
            // The iPad page's own report (`ipad_page`): to the log.
            "/log" => {
                let said: String = request.query.get("m").map_or("", |m| m.as_str()).chars().take(600).collect();
                log!("iPad page: {said}");
                respond(&mut stream, "204 No Content", "text/plain", b"", &[])?;
            }
            "/" | "/cabin.html" => {
                log!(
                    "iPad: the page asked for ({}), {}",
                    if paired { "paired" } else { "not paired" },
                    request.header("user-agent")
                );
                let page = std::fs::read_to_string(shared.ui_root.join("cabin.html")).unwrap_or_default();
                respond(&mut stream, "200 OK", "text/html; charset=utf-8", ipad_page(&page).as_bytes(), &[])?;
            }
            path => {
                let mut parts = path.trim_start_matches('/').splitn(2, '/');
                let first = parts.next().unwrap_or("");
                let rest = parts.next().unwrap_or("");
                let folder = shared.folders.iter().find(|(name, _)| name == first);
                let file = match folder {
                    // The player's own (their maps, pictures): paired only.
                    Some(_) if !paired => None,
                    Some((_, dir)) => within(dir, rest),
                    None => within(&shared.ui_root, path.trim_start_matches('/')),
                };
                match file.and_then(|file| std::fs::read(&file).ok().map(|bytes| (file, bytes))) {
                    Some((file, bytes)) => respond(&mut stream, "200 OK", content_type(&file), &bytes, &[])?,
                    None => respond(&mut stream, "404 Not Found", "text/plain", b"not found", &[])?,
                }
            }
        }
        if request.header("connection").eq_ignore_ascii_case("close") {
            break;
        }
    }
    Ok(())
}

/// The cabin page as the iPad's: laid out 1280 wide as the truck's (Safari scales it to the glass), full screen
/// from the Home Screen, and told it's the iPad's.
fn ipad_page(page: &str) -> String {
    const HEAD: &str = r#"<meta name="viewport" content="width=1280, user-scalable=no, viewport-fit=cover" />
    <meta name="apple-mobile-web-app-capable" content="yes" />
    <meta name="mobile-web-app-capable" content="yes" />
    <meta name="apple-mobile-web-app-status-bar-style" content="black-translucent" />
    <meta name="apple-mobile-web-app-title" content="Euro-Hub" />
    <link rel="manifest" href="/manifest.webmanifest" />
    <link rel="apple-touch-icon" href="/dashly.png" />
    <script>window.westplayIpad = true
    // What goes wrong on the iPad, told to WestPlay's log (Safari's console is out of reach there).
    ;(function () {
      var told = 0
      var tell = function (text) {
        if (told++ > 20) return
        try { new Image().src = '/log?m=' + encodeURIComponent(String(text).slice(0, 600)) } catch (e) {}
      }
      window.addEventListener('error', function (event) {
        var where = event.filename ? ' (' + event.filename + ':' + event.lineno + ':' + event.colno + ')' : ''
        tell('error: ' + (event.message || (event.target && (event.target.src || event.target.href)) || 'unknown') + where)
      }, true)
      window.addEventListener('unhandledrejection', function (event) {
        var reason = event.reason
        tell('rejection: ' + (reason && (reason.stack || reason.message) || reason))
      })
      window.westplayTell = tell
      setTimeout(function () {
        var root = document.getElementById('root')
        tell('after 8 s: ' + (root && root.children.length ? 'the page is drawn' : 'nothing drawn') + ', ' + innerWidth + 'x' + innerHeight)
      }, 8000)
    })()
    </script>"#;
    page.replacen("<head>", &format!("<head>\n    {HEAD}"), 1)
}

fn pair(stream: &mut TcpStream, request: &Request, shared: &Shared) -> io::Result<()> {
    match new_token(shared, request.query.get("code").map_or("", |code| code.trim())) {
        Some(token) => respond(stream, "200 OK", "text/plain", b"paired", &[&cookie(&token)]),
        None => respond(stream, "403 Forbidden", "text/plain", b"wrong code", &[]),
    }
}

/// A new device's token, for the right code (`None` for a wrong one).
fn new_token(shared: &Shared, given: &str) -> Option<String> {
    // Guessing slowed down: each wrong code makes the next answer wait longer.
    let wrong = shared.wrong_codes.load(Ordering::Acquire);
    if wrong > 3 {
        std::thread::sleep(Duration::from_secs(u64::from(wrong.min(30))));
    }
    let mut pairing = shared.pairing.lock().unwrap_or_else(|e| e.into_inner());
    if given.is_empty() || given != pairing.code {
        shared.wrong_codes.fetch_add(1, Ordering::AcqRel);
        return None;
    }
    shared.wrong_codes.store(0, Ordering::Release);
    let token = format!("{:016x}{:016x}", random(), random());
    pairing.tokens.insert(token.clone());
    save_pairing(&shared.pairing_file, &pairing);
    log!("iPad: a device paired");
    Some(token)
}

fn cookie(token: &str) -> String {
    format!("Set-Cookie: {COOKIE}={token}; Path=/; Max-Age=315360000; SameSite=Lax; HttpOnly")
}

/// `relative` within `dir`, if it stays in it.
fn within(dir: &Path, relative: &str) -> Option<PathBuf> {
    let relative = Path::new(relative);
    if relative.components().any(|part| !matches!(part, Component::Normal(_))) {
        return None;
    }
    let file = dir.join(relative);
    file.is_file().then_some(file)
}

fn content_type(file: &Path) -> &'static str {
    match file.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase().as_str() {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        "ttf" => "font/ttf",
        "pbf" => "application/x-protobuf",
        "wasm" => "application/wasm",
        "mp3" => "audio/mpeg",
        _ => "application/octet-stream",
    }
}

fn respond(stream: &mut TcpStream, status: &str, kind: &str, body: &[u8], extra: &[&str]) -> io::Result<()> {
    let mut head = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nCache-Control: no-cache\r\n",
        body.len()
    );
    for line in extra {
        head.push_str(line);
        head.push_str("\r\n");
    }
    head.push_str("\r\n");
    stream.write_all(head.as_bytes())?;
    stream.write_all(body)?;
    stream.flush()
}

fn websocket(mut stream: TcpStream, mut reader: BufReader<TcpStream>, request: &Request, shared: &Shared) -> io::Result<()> {
    let key = request.header("sec-websocket-key");
    let accept = base64::engine::general_purpose::STANDARD
        .encode(sha1(format!("{key}258EAFA5-E914-47DA-95CA-C5AB0DC85B11").as_bytes()));
    let head = format!(
        "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {accept}\r\n\r\n"
    );
    stream.write_all(head.as_bytes())?;
    stream.set_read_timeout(None)?;
    stream.set_nodelay(true)?;

    let (sender, receiver) = mpsc::channel::<Out>();
    let id = shared.next_client.fetch_add(1, Ordering::AcqRel);
    // The newest iPad is the head unit: an earlier one's writer ends, and its connection with it.
    *shared.client.lock().unwrap_or_else(|e| e.into_inner()) = Some((id, sender.clone()));
    *shared.gone_at.lock().unwrap_or_else(|e| e.into_inner()) = None;
    log!("iPad: connected");

    let mut writer = stream.try_clone()?;
    let writing = std::thread::Builder::new().name("westplay-ipad-writer".into()).spawn(move || {
        for out in receiver {
            let written = match out {
                Out::Text(text) => write_frame(&mut writer, 0x1, text.as_bytes()),
                Out::Binary(bytes) => write_frame(&mut writer, 0x2, &bytes),
                Out::Pong(bytes) => write_frame(&mut writer, 0xA, &bytes),
            };
            if written.is_err() {
                break;
            }
        }
        let _ = writer.shutdown(std::net::Shutdown::Both);
    })?;

    let mut message: Vec<u8> = Vec::new();
    let ended = loop {
        match read_frame(&mut reader) {
            Ok((fin, opcode, payload)) => match opcode {
                0x0 | 0x1 | 0x2 => {
                    message.extend_from_slice(&payload);
                    if message.len() > MOST_MESSAGE {
                        break Ok(());
                    }
                    if fin {
                        if let Ok(text) = String::from_utf8(std::mem::take(&mut message)) {
                            shared.incoming.lock().unwrap_or_else(|e| e.into_inner()).push(text);
                        }
                    }
                }
                0x8 => break Ok(()),
                0x9 => {
                    let _ = sender.send(Out::Pong(payload));
                }
                _ => {}
            },
            Err(e) => break Err(e),
        }
    };
    drop(sender);
    let mut client = shared.client.lock().unwrap_or_else(|e| e.into_inner());
    if client.as_ref().is_some_and(|(current, _)| *current == id) {
        *client = None;
        *shared.gone_at.lock().unwrap_or_else(|e| e.into_inner()) = Some(Instant::now());
        log!("iPad: gone");
    }
    drop(client);
    let _ = stream.shutdown(std::net::Shutdown::Both);
    let _ = writing.join();
    match ended {
        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => Ok(()),
        ended => ended,
    }
}

fn read_frame(reader: &mut impl Read) -> io::Result<(bool, u8, Vec<u8>)> {
    let mut head = [0u8; 2];
    reader.read_exact(&mut head)?;
    let fin = head[0] & 0x80 != 0;
    let opcode = head[0] & 0x0F;
    let masked = head[1] & 0x80 != 0;
    let mut len = u64::from(head[1] & 0x7F);
    if len == 126 {
        let mut bytes = [0u8; 2];
        reader.read_exact(&mut bytes)?;
        len = u64::from(u16::from_be_bytes(bytes));
    } else if len == 127 {
        let mut bytes = [0u8; 8];
        reader.read_exact(&mut bytes)?;
        len = u64::from_be_bytes(bytes);
    }
    if len > MOST_MESSAGE as u64 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "message too large"));
    }
    let mut mask = [0u8; 4];
    if masked {
        reader.read_exact(&mut mask)?;
    }
    let mut payload = vec![0u8; len as usize];
    reader.read_exact(&mut payload)?;
    if masked {
        for (i, byte) in payload.iter_mut().enumerate() {
            *byte ^= mask[i % 4];
        }
    }
    Ok((fin, opcode, payload))
}

fn write_frame(writer: &mut impl Write, opcode: u8, payload: &[u8]) -> io::Result<()> {
    let mut head = vec![0x80 | opcode];
    match payload.len() {
        len if len < 126 => head.push(len as u8),
        len if len <= 0xFFFF => {
            head.push(126);
            head.extend_from_slice(&(len as u16).to_be_bytes());
        }
        len => {
            head.push(127);
            head.extend_from_slice(&(len as u64).to_be_bytes());
        }
    }
    writer.write_all(&head)?;
    writer.write_all(payload)?;
    writer.flush()
}

/// SHA-1, for the WebSocket handshake only.
fn sha1(data: &[u8]) -> [u8; 20] {
    let mut h: [u32; 5] = [0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0];
    let mut message = data.to_vec();
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&((data.len() as u64) * 8).to_be_bytes());
    for chunk in message.chunks(64) {
        let mut w = [0u32; 80];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([chunk[4 * i], chunk[4 * i + 1], chunk[4 * i + 2], chunk[4 * i + 3]]);
        }
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }
        let [mut a, mut b, mut c, mut d, mut e] = h;
        for (i, word) in w.iter().enumerate() {
            let (f, k) = match i {
                0..=19 => ((b & c) | (!b & d), 0x5A827999),
                20..=39 => (b ^ c ^ d, 0x6ED9EBA1),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1BBCDC),
                _ => (b ^ c ^ d, 0xCA62C1D6),
            };
            let t = a.rotate_left(5).wrapping_add(f).wrapping_add(e).wrapping_add(k).wrapping_add(*word);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = t;
        }
        for (state, value) in h.iter_mut().zip([a, b, c, d, e]) {
            *state = state.wrapping_add(value);
        }
    }
    let mut out = [0u8; 20];
    for (i, word) in h.iter().enumerate() {
        out[4 * i..4 * i + 4].copy_from_slice(&word.to_be_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handshake_accept_key() {
        // RFC 6455's example.
        let accept = base64::engine::general_purpose::STANDARD
            .encode(sha1(b"dGhlIHNhbXBsZSBub25jZQ==258EAFA5-E914-47DA-95CA-C5AB0DC85B11"));
        assert_eq!(accept, "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=");
    }

    #[test]
    fn frames_round_trip() {
        let mut bytes = Vec::new();
        write_frame(&mut bytes, 0x1, &[7u8; 300]).unwrap();
        let (fin, opcode, payload) = read_frame(&mut bytes.as_slice()).unwrap();
        assert!(fin);
        assert_eq!(opcode, 0x1);
        assert_eq!(payload, vec![7u8; 300]);
    }

    #[test]
    fn files_stay_in_their_folder() {
        let dir = std::env::temp_dir();
        assert_eq!(within(&dir, "../secret"), None);
        assert_eq!(within(&dir, "C:/Windows/win.ini"), None);
    }

    #[test]
    fn pairs_then_talks_over_the_websocket() {
        let dir = std::env::temp_dir().join(format!("westplay-ipad-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("cabin.html"), "<html><head></head></html>").unwrap();
        let remote = Remote::start(&dir, dir.clone(), Vec::new()).unwrap();
        let ask = |request: &str| {
            let mut stream = TcpStream::connect(("127.0.0.1", PORT)).unwrap();
            stream.write_all(request.as_bytes()).unwrap();
            let mut answer = String::new();
            let _ = stream.read_to_string(&mut answer);
            answer
        };
        assert!(ask("GET /paired HTTP/1.1
Connection: close

").starts_with("HTTP/1.1 401"));
        assert!(ask("GET / HTTP/1.1
Connection: close

").contains("window.westplayIpad = true"));
        assert!(ask("GET /pair?code=000000x HTTP/1.1
Connection: close

").starts_with("HTTP/1.1 403"));
        let code = remote.status().code;
        let paired = ask(&format!("GET /pair?code={code} HTTP/1.1
Connection: close

"));
        let token = paired.split("westplay=").nth(1).unwrap().split(';').next().unwrap().to_owned();

        let mut socket = TcpStream::connect(("127.0.0.1", PORT)).unwrap();
        let upgrade = format!(
            "GET /ws HTTP/1.1
Upgrade: websocket
Connection: Upgrade
Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==
Cookie: westplay={token}

"
        );
        socket.write_all(upgrade.as_bytes()).unwrap();
        let mut head = [0u8; 129];
        socket.read_exact(&mut head).unwrap();
        assert!(String::from_utf8_lossy(&head).starts_with("HTTP/1.1 101"));
        // A masked text frame from the page.
        let text = br#"{"type":"ready"}"#;
        let mask = [1u8, 2, 3, 4];
        let mut frame = vec![0x81, 0x80 | text.len() as u8];
        frame.extend_from_slice(&mask);
        frame.extend(text.iter().enumerate().map(|(i, byte)| byte ^ mask[i % 4]));
        socket.write_all(&frame).unwrap();
        let start = Instant::now();
        let mut taken = Vec::new();
        while taken.is_empty() && start.elapsed() < Duration::from_secs(2) {
            taken = remote.take_messages();
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(taken, vec![r#"{"type":"ready"}"#.to_owned()]);
        assert!(remote.connected() && remote.head_unit());
        remote.post(r#"{"type":"x"}"#);
        let (_, opcode, payload) = read_frame(&mut socket).unwrap();
        assert_eq!((opcode, payload.as_slice()), (0x1, br#"{"type":"x"}"#.as_slice()));
    }
}
