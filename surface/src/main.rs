//! Asolaria ASI OS — human front-end shell.
//!
//! Pure Rust std, ZERO external crates (8-byte-host ethos, like host8-serve): a thread-per-connection
//! TCP/HTTP server that (1) serves the ASI OS UI, (2) proxies your LOCAL Asolaria fabric so the UI shows the
//! real running system (kernel :5088, recall-vault :4796, bus :4947, fabric :4949), and (3) runs real
//! terminal sessions (cmd / PowerShell / WSL bash) — spawn via std::process, stream stdout/stderr over
//! Server-Sent-Events, feed stdin over POST. Framework latency is microseconds; the CLIs (claude/codex)
//! run as commands inside the shells. E=0: it launches only what the human types.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::Duration;

const BIND: &str = "127.0.0.1:4600";
const INDEX_HTML: &str = include_str!("index.html");

struct Session {
    stdin: Mutex<Option<ChildStdin>>,
    out: Arc<Mutex<Vec<u8>>>,
    child: Mutex<Option<Child>>,
    label: String,
}

fn sessions() -> &'static Mutex<HashMap<u64, Arc<Session>>> {
    static S: OnceLock<Mutex<HashMap<u64, Arc<Session>>>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(HashMap::new()))
}
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

fn main() {
    let listener = TcpListener::bind(BIND).unwrap_or_else(|e| panic!("bind {BIND}: {e}"));
    println!("ASOLARIA ASI OS front-end — live on http://{BIND}  (Rust std, 0 deps)");
    for stream in listener.incoming().flatten() {
        thread::spawn(move || {
            let _ = handle(stream);
        });
    }
}

fn handle(mut stream: TcpStream) -> std::io::Result<()> {
    let peer = stream.try_clone()?;
    let mut reader = BufReader::new(peer);

    let mut req_line = String::new();
    if reader.read_line(&mut req_line)? == 0 {
        return Ok(());
    }
    let mut parts = req_line.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let target = parts.next().unwrap_or("/").to_string();

    let mut content_length = 0usize;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            break;
        }
        let t = line.trim_end();
        if t.is_empty() {
            break;
        }
        let lower = t.to_ascii_lowercase();
        if let Some(v) = lower.strip_prefix("content-length:") {
            content_length = v.trim().parse().unwrap_or(0);
        }
    }
    let mut body = vec![0u8; content_length];
    if content_length > 0 {
        reader.read_exact(&mut body)?;
    }

    let (path, query) = match target.split_once('?') {
        Some((p, q)) => (p.to_string(), q.to_string()),
        None => (target.clone(), String::new()),
    };

    match (method.as_str(), path.as_str()) {
        ("GET", "/") | ("GET", "/index.html") => {
            let html = INDEX_HTML
                .replace("__ASOLARIA_SEAT__", &seat_name())
                .replace("__ASOLARIA_PID__", &seat_pid());
            write_resp(&mut stream, 200, "text/html; charset=utf-8", html.as_bytes())
        }
        ("GET", "/health") => write_resp(&mut stream, 200, "text/plain", b"ok"),
        ("GET", "/api/live") => {
            let s = live_status();
            write_resp(&mut stream, 200, "text/plain; charset=utf-8", s.as_bytes())
        }
        ("POST", "/term/spawn") => {
            let shell = String::from_utf8_lossy(&body).trim().to_string();
            let id = spawn_session(&shell);
            write_resp(&mut stream, 200, "text/plain", id.to_string().as_bytes())
        }
        ("POST", "/term/input") => {
            let id = query_id(&query);
            if let Some(sess) = sessions().lock().unwrap().get(&id).cloned() {
                if let Some(si) = sess.stdin.lock().unwrap().as_mut() {
                    let _ = si.write_all(&body);
                    let _ = si.flush();
                }
            }
            write_resp(&mut stream, 200, "text/plain", b"ok")
        }
        ("POST", "/term/kill") => {
            let id = query_id(&query);
            if let Some(sess) = sessions().lock().unwrap().remove(&id) {
                if let Some(ch) = sess.child.lock().unwrap().as_mut() {
                    let _ = ch.kill();
                }
            }
            write_resp(&mut stream, 200, "text/plain", b"ok")
        }
        ("GET", "/term/stream") => {
            let id = query_id(&query);
            stream_session(stream, id)
        }
        ("POST", "/win/launch") => {
            let r = launch_windows_env();
            write_resp(&mut stream, 200, "text/plain", r.as_bytes())
        }
        _ => write_resp(&mut stream, 404, "text/plain", b"not found"),
    }
}

fn write_resp(stream: &mut TcpStream, code: u16, ctype: &str, body: &[u8]) -> std::io::Result<()> {
    let status = match code {
        200 => "200 OK",
        404 => "404 Not Found",
        _ => "500 Internal Server Error",
    };
    let head = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes())?;
    stream.write_all(body)?;
    stream.flush()
}

/// Launch a Windows environment as a window over the Asolaria surface.
/// Prefers Windows Sandbox (a clean, disposable Windows env in its own window; Pro/Hyper-V
/// only); falls back to Explorer (the host Windows desktop/shell as a window — always
/// available). E=0: fires only on the operator's click, exactly like the terminal spawns.
fn launch_windows_env() -> String {
    let sandbox = "C:\\Windows\\System32\\WindowsSandbox.exe";
    if std::path::Path::new(sandbox).exists() && Command::new(sandbox).spawn().is_ok() {
        return "launched Windows Sandbox — a clean Windows env in its own window".into();
    }
    match Command::new("explorer.exe").arg("shell:MyComputerFolder").spawn() {
        Ok(_) => "launched Windows — host desktop / Explorer as a window".into(),
        Err(e) => format!("could not launch Windows env: {e}"),
    }
}

fn query_id(query: &str) -> u64 {
    for kv in query.split('&') {
        if let Some(v) = kv.strip_prefix("id=") {
            return v.parse().unwrap_or(0);
        }
    }
    0
}

/// Windows: real Windows shells (+ wsl bash bridge). This same binary also cross-compiles for Linux.
#[cfg(windows)]
fn shell_command(shell: &str) -> Command {
    match shell {
        "powershell" | "pwsh" => {
            let mut c = Command::new("powershell.exe");
            c.args(["-NoLogo", "-NoProfile"]);
            c
        }
        "bash" | "wsl" => {
            let mut c = Command::new("wsl.exe");
            c.args(["-d", "Ubuntu", "--", "bash", "-il"]);
            c
        }
        _ => Command::new("cmd.exe"),
    }
}

/// Linux (the bare Asolaria-on-metal OS): native shells — NO Windows dependency. Terminals work with
/// Windows fully closed. `cmd`/`powershell` requests map to a real shell so the same UI drives both.
#[cfg(unix)]
fn shell_command(shell: &str) -> Command {
    let inner = match shell {
        "zsh" => "zsh",
        "sh" => "sh",
        "pwsh" | "powershell" => "pwsh", // if PowerShell-on-Linux is installed
        _ => "bash",
    };
    // Wrap the shell in a real pseudo-terminal via `script` (util-linux) so it is FULLY interactive
    // over pipes — pure std, no PTY crate. `-f` flushes each write so output streams live.
    let mut c = Command::new("script");
    c.args(["-qfec", inner, "/dev/null"]);
    c
}

fn spawn_session(shell: &str) -> u64 {
    let mut cmd = shell_command(shell);
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let out = Arc::new(Mutex::new(Vec::<u8>::new()));
    let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);

    match cmd.spawn() {
        Ok(mut child) => {
            let stdin = child.stdin.take();
            if let Some(mut so) = child.stdout.take() {
                let o = out.clone();
                thread::spawn(move || pump(&mut so, o));
            }
            if let Some(mut se) = child.stderr.take() {
                let o = out.clone();
                thread::spawn(move || pump(&mut se, o));
            }
            let sess = Arc::new(Session {
                stdin: Mutex::new(stdin),
                out,
                child: Mutex::new(Some(child)),
                label: shell.to_string(),
            });
            sessions().lock().unwrap().insert(id, sess);
        }
        Err(e) => {
            out.lock()
                .unwrap()
                .extend_from_slice(format!("[asi-os] could not spawn '{shell}': {e}\r\n").as_bytes());
            let sess = Arc::new(Session {
                stdin: Mutex::new(None),
                out,
                child: Mutex::new(None),
                label: shell.to_string(),
            });
            sessions().lock().unwrap().insert(id, sess);
        }
    }
    id
}

fn pump<R: Read>(r: &mut R, out: Arc<Mutex<Vec<u8>>>) {
    let mut buf = [0u8; 4096];
    loop {
        match r.read(&mut buf) {
            Ok(0) | Err(_) => break,
            Ok(n) => out.lock().unwrap().extend_from_slice(&buf[..n]),
        }
    }
}

fn stream_session(mut stream: TcpStream, id: u64) -> std::io::Result<()> {
    let sess = match sessions().lock().unwrap().get(&id).cloned() {
        Some(s) => s,
        None => return write_resp(&mut stream, 404, "text/plain", b"no session"),
    };
    let head = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nConnection: keep-alive\r\n\r\n";
    stream.write_all(head.as_bytes())?;
    stream.write_all(format!("event: label\ndata: {}\n\n", sess.label).as_bytes())?;
    stream.flush()?;

    let mut offset = 0usize;
    let mut idle_ticks = 0u32;
    loop {
        let chunk = {
            let buf = sess.out.lock().unwrap();
            if buf.len() > offset {
                let c = buf[offset..].to_vec();
                offset += c.len();
                Some(c)
            } else {
                None
            }
        };
        let alive = sessions().lock().unwrap().contains_key(&id);
        match chunk {
            Some(c) => {
                idle_ticks = 0;
                let ev = format!("data: {}\n\n", base64(&c));
                if stream.write_all(ev.as_bytes()).is_err() || stream.flush().is_err() {
                    break;
                }
            }
            None => {
                if !alive {
                    let _ = stream.write_all(b"event: end\ndata: end\n\n");
                    let _ = stream.flush();
                    break;
                }
                idle_ticks += 1;
                if idle_ticks % 100 == 0 {
                    // keepalive comment so proxies/browsers don't drop the stream
                    if stream.write_all(b": keepalive\n\n").is_err() {
                        break;
                    }
                    let _ = stream.flush();
                }
                thread::sleep(Duration::from_millis(80));
            }
        }
    }
    Ok(())
}

/// Minimal std base64 encoder (no external crate) — terminal bytes survive SSE faithfully.
fn base64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0];
        let b1 = *chunk.get(1).unwrap_or(&0);
        let b2 = *chunk.get(2).unwrap_or(&0);
        out.push(T[(b0 >> 2) as usize] as char);
        out.push(T[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);
        out.push(if chunk.len() > 1 {
            T[(((b1 & 0x0f) << 2) | (b2 >> 6)) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            T[(b2 & 0x3f) as usize] as char
        } else {
            '='
        });
    }
    out
}

/// Read the LIVE local fabric surfaces and return a compact HBP-ish status block the UI renders.
fn live_status() -> String {
    let kernel = http_get("127.0.0.1", 5088, "/health.hbp", 400).unwrap_or_default();
    let mut kver = "-".to_string();
    let mut kanchor = "-".to_string();
    for line in kernel.lines() {
        if let Some(rest) = line.strip_prefix("HOST8KERNEL|") {
            for f in rest.split('|') {
                if let Some(v) = f.strip_prefix("version=") {
                    kver = v.to_string();
                }
                if let Some(v) = f.strip_prefix("anchor_pid=") {
                    kanchor = v.to_string();
                }
            }
        }
    }
    let kernel_up = !kernel.is_empty();
    let recall_up = http_get("127.0.0.1", 4796, "/api/health", 400).is_some();
    let bus = http_get("127.0.0.1", 4947, "/behcs/health", 400).unwrap_or_default();
    let bus_up = !bus.is_empty();
    let mut inbox = "-".to_string();
    if let Some(i) = bus.find("\"inbox_depth\":") {
        let tail = &bus[i + 14..];
        let n: String = tail.chars().take_while(|c| c.is_ascii_digit()).collect();
        if !n.is_empty() {
            inbox = n;
        }
    }
    let fabric_up = http_get("127.0.0.1", 4949, "/health", 400).is_some();

    format!(
        "ASIOSLIVE|ts_unix={}|seat={}\n\
         KERNEL|port=5088|up={}|version={}|anchor={}\n\
         RECALL|port=4796|up={}|role=local-inverted-index-vault\n\
         BUS|port=4947|up={}|inbox_depth={}\n\
         FABRIC|port=4949|up={}|role=super-dashboard\n\
         SOVLINUX|role=local-sovereignty-vault|present=local\n",
        now_unix(),
        seat_name(),
        b(kernel_up),
        kver,
        kanchor,
        b(recall_up),
        b(bus_up),
        inbox,
        b(fabric_up),
    )
}

fn b(v: bool) -> u8 {
    u8::from(v)
}

fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// This node's seat identity — minted LOCALLY by `scripts/keygen` (never shared, never on GitHub).
/// Resolves from $ASOLARIA_SEAT / $ASOLARIA_PID, else ~/.asolaria/seat.name|seat.pid, else a default.
fn seat_name() -> String {
    read_ident("ASOLARIA_SEAT", "seat.name", "ASOLARIA-NODE")
}
fn seat_pid() -> String {
    read_ident("ASOLARIA_PID", "seat.pid", "unregistered")
}
fn read_ident(env_key: &str, file: &str, default: &str) -> String {
    if let Ok(v) = std::env::var(env_key) {
        if !v.trim().is_empty() {
            return v.trim().to_string();
        }
    }
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_default();
    if !home.is_empty() {
        if let Ok(s) = std::fs::read_to_string(format!("{home}/.asolaria/{file}")) {
            let s = s.trim();
            if !s.is_empty() {
                return s.to_string();
            }
        }
    }
    default.to_string()
}

fn http_get(host: &str, port: u16, path: &str, timeout_ms: u64) -> Option<String> {
    let addr: std::net::SocketAddr = format!("{host}:{port}").parse().ok()?;
    let mut s = TcpStream::connect_timeout(&addr, Duration::from_millis(timeout_ms)).ok()?;
    s.set_read_timeout(Some(Duration::from_millis(timeout_ms))).ok()?;
    let req = format!("GET {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n");
    s.write_all(req.as_bytes()).ok()?;
    let mut buf = Vec::new();
    let mut tmp = [0u8; 2048];
    loop {
        match s.read(&mut tmp) {
            Ok(0) | Err(_) => break,
            Ok(n) => buf.extend_from_slice(&tmp[..n]),
        }
    }
    if buf.is_empty() {
        return None;
    }
    let resp = String::from_utf8_lossy(&buf).to_string();
    Some(match resp.find("\r\n\r\n") {
        Some(i) => resp[i + 4..].to_string(),
        None => resp,
    })
}
