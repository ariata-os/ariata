use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use crate::server::webhook::AppState;

/// Bound on the PTY→websocket output queue. When the shell produces output
/// faster than a slow client can drain it, the reader thread blocks on send,
/// the PTY's kernel buffer fills, and the process is flow-controlled — the
/// same backpressure a real terminal applies. Without a bound, a chatty
/// process (`yes`, `cat bigfile`) would grow memory without limit.
const OUTPUT_QUEUE: usize = 256;

/// The tmux session every terminal tab attaches to. One shared session is the
/// point: two tabs see the same screen.
const TMUX_SESSION: &str = "virtues";

/// Our tmux server runs on its own socket rather than the user's default one.
/// Two reasons: the global options we set below would otherwise be applied to
/// whatever tmux server the user already has (in local dev that's the
/// developer's personal one, and clobbering their `default-terminal` is a nasty
/// surprise), and a `virtues` session appearing in their `tmux ls` is noise.
/// To reach this session from a shell: `tmux -L virtues attach`.
const TMUX_SOCKET: &str = "virtues";

// ---------------------------------------------------------------------------
// WebSocket <-> in-process PTY bridge
//
// The terminal tab in the web UI opens a real login shell on *this* machine.
// We spawn a PTY directly (no sshd, no localhost SSH round-trip): the shell
// runs as whatever user the server process runs as — `virtues` on a box, the
// developer in local dev. Same code path everywhere.
//
// The PTY runs a tmux *client* attached to a long-lived session, not the shell
// itself. The shell (and anything long-running in it — `claude`, a build, a
// migration) is a child of the tmux *server*, which outlives this websocket.
// So a closed tab, a slept laptop, or a dropped iroh path detaches instead of
// killing the work, and the next connection reattaches to the same screen. If
// tmux isn't installed we fall back to a plain login shell, which behaves as
// before: disconnect kills it.
//
// portable-pty is thread-based (blocking reader/writer), so we bridge to the
// async websocket through channels: a reader thread pumps PTY output into a
// bounded mpsc the select loop drains, and a writer thread drains input the
// loop feeds it. The select loop keeps the master so it can resize, and kills
// the child on disconnect so no clients leak.
// ---------------------------------------------------------------------------

/// Terminal size, sent by the client on connect. The PTY has to be born at the
/// right size: a full-screen TUI reads the winsize at startup and paints once,
/// so a PTY that opens at 80x24 and resizes a beat later gets a garbled first
/// frame that only a manual redraw clears.
#[derive(Debug, Deserialize)]
pub struct TerminalParams {
    cols: Option<u16>,
    rows: Option<u16>,
}

/// Clamp a client-supplied dimension: it reaches an ioctl, and 0 rows would
/// make curses apps divide by zero.
fn clamp_dim(v: Option<u16>, default: u16) -> u16 {
    v.unwrap_or(default).clamp(2, 1000)
}

/// Handler for the terminal WebSocket.
pub async fn terminal_ws_handler(
    ws: WebSocketUpgrade,
    State(_state): State<AppState>,
    Query(params): Query<TerminalParams>,
    headers: HeaderMap,
) -> Response {
    // Defense against Cross-Site WebSocket Hijacking: the session cookie alone
    // would let any page open `ws://<box>/ws/terminal` and ride it to a shell.
    // The web app always connects same-origin (location.host), so a foreign
    // Origin means a foreign page — reject the upgrade before it happens.
    if let Some(rejection) = check_same_origin(&headers) {
        return rejection;
    }
    let size = PtySize {
        cols: clamp_dim(params.cols, 80),
        rows: clamp_dim(params.rows, 24),
        pixel_width: 0,
        pixel_height: 0,
    };
    ws.on_upgrade(move |socket| handle_socket(socket, size))
        .into_response()
}

/// Returns `Some(rejection)` if the browser's `Origin` is not one of ours.
///
/// The policy is the CORS layer's (`server::origin_is_ours`), not a second
/// one: this used to compare `Origin` against `Host`, which holds only when
/// the page was served by the same authority it is dialling. The Mac app is
/// (the SPA comes through the loopback splice), but the phone is not — its
/// bundle loads from the `virtues://` scheme, so `Origin` is
/// `virtues://localhost` and `Host` is the box, and every terminal upgrade
/// from iOS was a 403 while the CORS layer beside it was happily saying yes.
///
/// A missing `Origin` (non-browser client) is allowed — auth is still enforced
/// by the route layer, and the CSWSH vector is browser-only: browsers always
/// send `Origin` on a WebSocket handshake.
fn check_same_origin(headers: &HeaderMap) -> Option<Response> {
    let origin = headers.get(header::ORIGIN).and_then(|v| v.to_str().ok())?;
    if crate::server::origin_is_ours(origin) {
        None
    } else {
        tracing::warn!("Terminal WS rejected: foreign origin {:?}", origin);
        Some((StatusCode::FORBIDDEN, "cross-origin websocket rejected").into_response())
    }
}

// ---------------------------------------------------------------------------
// Paste / drop -> a file on the box
//
// The clipboard lives in the browser, on the user's laptop; the shell runs here.
// stdin can't carry an image between them — a terminal has never moved pictures
// that way, and a native `claude` only manages it by reading the *local* OS
// clipboard, which this box doesn't have. So the paste becomes a file on disk
// and the terminal gets its path typed in at the cursor, exactly as if the user
// had typed it. Every CLI already knows what to do with a path.
//
// These land in the user's home, not the drive's media store: `media` is
// app-level content (page embeds, notebook sources), while this is a scratch
// file belonging to a shell session.
// ---------------------------------------------------------------------------

/// Where pasted files land, under $HOME.
const PASTE_DIR: &str = ".virtues/pastes";

/// Big enough for any screenshot, small enough that a stray paste can't fill the
/// disk. Enforced again as a body limit on the route.
const PASTE_MAX_BYTES: usize = 25 * 1024 * 1024;

/// Pastes are scratch. Sweep anything older than this on the next paste.
const PASTE_TTL: std::time::Duration = std::time::Duration::from_secs(7 * 24 * 60 * 60);

#[derive(Serialize)]
pub struct PasteResponse {
    /// Absolute, so it resolves no matter what the shell's cwd is.
    path: String,
}

/// Map the client-declared content type to an extension.
///
/// A whitelist of literals, not a sanitised passthrough: the content type is
/// attacker-controlled, and the return value becomes part of a filename. Nothing
/// here can carry a `/` or a `..`, so the path is safe by construction rather
/// than by validation.
fn paste_extension(content_type: Option<&str>) -> &'static str {
    let ct = content_type.unwrap_or("").split(';').next().unwrap_or("").trim();
    match ct {
        "image/png" => "png",
        "image/jpeg" => "jpg",
        "image/gif" => "gif",
        "image/webp" => "webp",
        "image/svg+xml" => "svg",
        "application/pdf" => "pdf",
        "text/plain" => "txt",
        _ => "bin",
    }
}

/// Delete pastes past their TTL. Best-effort: a paste that works but doesn't
/// tidy up is better than one that fails because tidying up did.
fn sweep_old_pastes(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let stale = entry
            .metadata()
            .and_then(|m| m.modified())
            .map(|t| t.elapsed().map(|age| age > PASTE_TTL).unwrap_or(false))
            .unwrap_or(false);
        if stale {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

/// Write a pasted blob under `home` and return its absolute path.
///
/// Content-addressed: pasting the same screenshot twice costs one file and
/// yields one stable path, which is also what makes the write idempotent.
fn store_paste(home: &Path, content_type: Option<&str>, body: &[u8]) -> std::io::Result<PathBuf> {
    let ext = paste_extension(content_type);
    let digest = <sha2::Sha256 as sha2::Digest>::digest(body);
    let name = format!("paste-{}.{ext}", &hex::encode(digest)[..16]);

    let dir = home.join(PASTE_DIR);
    std::fs::create_dir_all(&dir)?;
    sweep_old_pastes(&dir);

    let path = dir.join(name);
    if !path.exists() {
        std::fs::write(&path, body)?;
    }
    Ok(path)
}

/// Take a pasted or dropped blob, write it under $HOME, and hand back the path
/// for the frontend to type into the terminal.
pub async fn terminal_paste_handler(
    State(_state): State<AppState>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    if body.is_empty() {
        return (StatusCode::BAD_REQUEST, "empty paste").into_response();
    }
    if body.len() > PASTE_MAX_BYTES {
        return (StatusCode::PAYLOAD_TOO_LARGE, "paste too large").into_response();
    }

    let Ok(home) = std::env::var("HOME") else {
        tracing::error!("terminal paste: no HOME to write into");
        return (StatusCode::INTERNAL_SERVER_ERROR, "no home directory").into_response();
    };
    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok());

    match store_paste(Path::new(&home), content_type, &body) {
        Ok(path) => {
            tracing::debug!("terminal paste: {} ({} bytes)", path.display(), body.len());
            (
                StatusCode::CREATED,
                axum::Json(PasteResponse {
                    path: path.to_string_lossy().into_owned(),
                }),
            )
                .into_response()
        }
        Err(e) => {
            tracing::error!("terminal paste: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "cannot store paste").into_response()
        }
    }
}

/// Handle the established WebSocket connection.
async fn handle_socket(mut socket: WebSocket, size: PtySize) {
    if let Err(e) = pty_bridge(&mut socket, size).await {
        tracing::error!("Terminal PTY bridge error: {}", e);
        let _ = socket
            .send(Message::Text(format!(
                "\r\n\x1b[31mTerminal error: {}\x1b[0m\r\n",
                e
            )))
            .await;
    }
}

/// The PATH terminal sessions run with: the conventional per-user bin dirs
/// ahead of the service PATH.
///
/// CLIs the user installs into their home (the Claude Code installer →
/// ~/.local/bin, `npm -g`, cargo, our own helpers → ~/.virtues/bin) must
/// survive across sessions. The shell is a login shell, but on the appliance
/// the `virtues` HOME has no profile to source, so each session would otherwise
/// start with the bare service PATH and lose whatever was installed last
/// session — the binary persists on disk, but nothing points PATH at it.
fn session_path(home: Option<&str>) -> String {
    let service_path = std::env::var("PATH").unwrap_or_default();
    let Some(home) = home else {
        return service_path;
    };
    let mut path = [
        ".local/bin",
        ".virtues/bin",
        "bin",
        ".npm-global/bin",
        ".cargo/bin",
    ]
    .iter()
    .map(|d| format!("{home}/{d}"))
    .collect::<Vec<_>>()
    .join(":");
    if !service_path.is_empty() {
        path.push(':');
        path.push_str(&service_path);
    }
    path
}

/// Find `tmux` on the session PATH. We can't shell out to `which` (there's no
/// shell in this path) and we don't take a dependency for one lookup.
fn find_tmux(path: &str) -> Option<PathBuf> {
    path.split(':')
        .filter(|dir| !dir.is_empty())
        .map(|dir| PathBuf::from(dir).join("tmux"))
        .find(|candidate| candidate.is_file())
}

/// The tmux server options we want, as one `;`-separated command list.
///
/// The shape depends on whether the session already exists, because tmux is
/// unforgiving in both directions:
///
/// - Creating: the list must `start-server` first (a bare `set-option` against a
///   socket with no server fails), and must *end* by creating the session. A
///   tmux server with no sessions exits immediately, so configuring one and
///   leaving would take the options down with it — verified, `mouse` came back
///   unset on the next client. Creating last also means the options are already
///   set when the pane is born, which matters: `default-terminal` and
///   `history-limit` are read at pane creation, so setting them afterwards would
///   strand the first pane on `screen` with 2000 lines of history forever.
/// - Reattaching: no `new-session` at all. Both `-A -d` and a bare `-d` *fail*
///   against an existing session ("open terminal failed" / "duplicate session"),
///   and a failing command aborts the rest of the list. The server is already up,
///   so the options just get re-asserted.
///
/// - `default-terminal` is the $TERM programs inside tmux see, and `Tc` is what
///   tells tmux the outer terminal — xterm.js — takes 24-bit colour, which it
///   does. Neither is inferable: verified on tmux 3.4 that COLORTERM=truecolor
///   alone does *not* get RGB into the client's terminal features.
///   `screen-256color` over `tmux-256color` because the latter's terminfo entry
///   is missing on minimal images.
/// - `mouse` makes the wheel scroll tmux's history. Without it the wheel does
///   nothing at all: tmux owns the screen, so xterm's own scrollback never
///   fills and there is nothing under the viewport to scroll to.
/// - `history-limit` defaults to 2000 lines, which would silently truncate well
///   before the 10k scrollback the frontend advertises.
/// - Right-click otherwise opens tmux's own context menu, which inside a browser
///   reads as the page being broken; unbinding it gives the browser's menu back.
///   `unbind-key` on an already-unbound key is a no-op, so this stays idempotent.
///
/// Every set is `-g`, never `-ga`: this runs on *every* connect and appending is
/// not idempotent — three connects would leave three copies of `*:Tc` in the
/// option. Overwriting is safe because the server on this socket is ours alone.
fn tmux_config_args(session_exists: bool) -> Vec<&'static str> {
    let mut args = vec!["-L", TMUX_SOCKET];
    if !session_exists {
        args.extend(["start-server", ";"]);
    }
    args.extend([
        // The pane is born from *this* command, not from the PTY's client, so it
        // inherits this process's environment. tmux overrides TERM from
        // `default-terminal`, but nothing would otherwise put COLORTERM inside
        // the pane — and that's what programs read to decide they may emit
        // 24-bit colour. Without it tmux can render RGB while the app inside
        // never tries. (PATH gets in the same way: see `configure_tmux`.)
        "set-environment",
        "-g",
        "COLORTERM",
        "truecolor",
        ";",
        "set-option",
        "-g",
        "default-terminal",
        "screen-256color",
        ";",
        "set-option",
        "-g",
        "terminal-overrides",
        ",*:Tc",
        ";",
        "set-option",
        "-g",
        "mouse",
        "on",
        ";",
        "set-option",
        "-g",
        "history-limit",
        "10000",
        ";",
        "unbind-key",
        "-n",
        "MouseDown3Pane",
    ]);
    if !session_exists {
        args.extend([";", "new-session", "-d", "-s", TMUX_SESSION]);
    }
    args
}

/// Apply the server options and make sure the session exists. Best-effort *on
/// purpose*, and deliberately not part of the PTY's own command list: tmux aborts
/// a `;`-separated list at the first command that errors, so an option this tmux
/// build doesn't recognise would take `new-session` down with it and leave the
/// tab with no shell at all. Run separately, a failure here costs duller colours
/// and no mouse — the PTY still attaches, creating the session itself if this
/// never got that far.
async fn configure_tmux(tmux: &Path, path: &str) {
    let exists = tokio::process::Command::new(tmux)
        .args(["-L", TMUX_SOCKET, "has-session", "-t", TMUX_SESSION])
        .env("PATH", path)
        .output()
        .await
        .map(|out| out.status.success())
        .unwrap_or(false);

    // This call is what creates the session, so the shell in it inherits *this*
    // environment — not the PTY client's. PATH matters most: it's how a `claude`
    // installed into ~/.local/bin is on PATH at all (see `session_path`).
    let result = tokio::process::Command::new(tmux)
        .args(tmux_config_args(exists))
        .env("PATH", path)
        .env("COLORTERM", "truecolor")
        .output()
        .await;
    match result {
        Ok(out) if !out.status.success() => tracing::warn!(
            "tmux options rejected ({}): {}",
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        ),
        Err(e) => tracing::warn!("could not configure tmux: {e}"),
        Ok(_) => {}
    }
}

/// Build the command the PTY runs: a tmux client attached to the shared
/// session, or a plain login shell if tmux isn't installed.
fn session_command(tmux: Option<&Path>, path: &str, home: Option<&str>) -> CommandBuilder {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());

    let mut cmd = match tmux {
        Some(tmux) => {
            let mut cmd = CommandBuilder::new(tmux);
            // `new-session -A` is attach-or-create: the first tab starts the
            // session, every later one reattaches to it. Nothing else rides in
            // this list — see `configure_tmux` for why.
            cmd.args(["-L", TMUX_SOCKET, "new-session", "-A", "-s", TMUX_SESSION]);
            cmd
        }
        None => {
            tracing::warn!(
                "tmux not found on PATH — terminal sessions will not survive disconnects"
            );
            let mut cmd = CommandBuilder::new(&shell);
            cmd.arg("-l");
            cmd
        }
    };

    // What the tmux *client* (or the bare shell) talks to: xterm.js, which is a
    // truecolor xterm.
    cmd.env("TERM", "xterm-256color");
    cmd.env("COLORTERM", "truecolor");
    cmd.env("PATH", path);
    // tmux runs the login shell for us; tell it which one.
    cmd.env("SHELL", &shell);
    if let Some(home) = home {
        cmd.cwd(home);
    }
    cmd
}

async fn pty_bridge(
    socket: &mut WebSocket,
    size: PtySize,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Born at the client's real size — see `TerminalParams`.
    let pty = native_pty_system().openpty(size)?;

    let home = std::env::var("HOME").ok();
    let path = session_path(home.as_deref());
    let tmux = find_tmux(&path);
    if let Some(tmux) = &tmux {
        configure_tmux(tmux, &path).await;
    }
    let cmd = session_command(tmux.as_deref(), &path, home.as_deref());

    let mut child = pty.slave.spawn_command(cmd)?;
    // Close our handle to the slave; only the child should hold it, so the
    // master sees EOF when the shell exits.
    drop(pty.slave);

    let mut reader = pty.master.try_clone_reader()?;
    let mut writer = pty.master.take_writer()?;
    let master = pty.master;
    let mut killer = child.clone_killer();

    // PTY output -> async loop (blocking reads on a dedicated thread).
    // Bounded channel: `blocking_send` parks the thread when the consumer is
    // behind, which backpressures the shell through the kernel PTY buffer.
    let (out_tx, mut out_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(OUTPUT_QUEUE);
    std::thread::spawn(move || {
        let mut buf = [0u8; 8192];
        loop {
            match reader.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    if out_tx.blocking_send(buf[..n].to_vec()).is_err() {
                        break;
                    }
                }
            }
        }
    });

    // async loop -> PTY input (blocking writes on a dedicated thread).
    let (in_tx, in_rx) = std::sync::mpsc::channel::<Vec<u8>>();
    std::thread::spawn(move || {
        while let Ok(data) = in_rx.recv() {
            if writer.write_all(&data).is_err() {
                break;
            }
            let _ = writer.flush();
        }
    });

    // Child exit -> async loop (blocking wait on a dedicated thread).
    let (exit_tx, exit_rx) = tokio::sync::oneshot::channel::<u32>();
    std::thread::spawn(move || {
        let code = child.wait().map(|s| s.exit_code()).unwrap_or(1);
        let _ = exit_tx.send(code);
    });

    // The loop breaks on either the client disconnecting or the PTY closing.
    // We do NOT break on the child-exit signal: that can fire before the reader
    // thread has drained the shell's final bytes. Reader-EOF (`out_rx` yields
    // `None`) is the authoritative "all output flushed" signal, so we key off
    // it and only then report the exit code.
    let mut shell_closed = false;
    loop {
        tokio::select! {
            // WebSocket -> PTY: user input and resize events.
            ws_msg = socket.recv() => {
                match ws_msg {
                    Some(Ok(Message::Text(t))) => {
                        if let Ok(cmd) = serde_json::from_str::<serde_json::Value>(&t) {
                            let msg_type = cmd.get("type").and_then(|v| v.as_str()).unwrap_or("");
                            match msg_type {
                                "input" => {
                                    if let Some(data) = cmd.get("data").and_then(|v| v.as_str()) {
                                        if in_tx.send(data.as_bytes().to_vec()).is_err() {
                                            break;
                                        }
                                    }
                                }
                                "resize" => {
                                    let cols = cmd.get("cols").and_then(|v| v.as_u64()).unwrap_or(80) as u16;
                                    let rows = cmd.get("rows").and_then(|v| v.as_u64()).unwrap_or(24) as u16;
                                    let _ = master.resize(PtySize {
                                        rows,
                                        cols,
                                        pixel_width: 0,
                                        pixel_height: 0,
                                    });
                                }
                                _ => {}
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Err(e)) => {
                        tracing::debug!("Terminal WebSocket error: {}", e);
                        break;
                    }
                    _ => {}
                }
            }

            // PTY -> WebSocket: shell output. Sent as Binary so xterm.js decodes
            // UTF-8 itself — a lossy decode here would mangle any multibyte glyph
            // straddling a read boundary.
            out = out_rx.recv() => {
                match out {
                    Some(bytes) => {
                        if socket.send(Message::Binary(bytes)).await.is_err() {
                            break;
                        }
                    }
                    None => {
                        shell_closed = true; // PTY closed and fully drained
                        break;
                    }
                }
            }
        }
    }

    // If the session ended on its own (vs. the client disconnecting), all its
    // output is now drained, so report the exit code.
    if shell_closed {
        if let Ok(code) = exit_rx.await {
            let _ = socket
                .send(Message::Text(format!(
                    "\r\n\x1b[90m[process exited with code {}]\x1b[0m\r\n",
                    code
                )))
                .await;
        }
    }

    // Reap the PTY's child on disconnect so no clients leak (no-op if it already
    // exited). Under tmux that child is the *client*: killing it detaches, and
    // the tmux server keeps the shell and whatever it's running alive for the
    // next connection to reattach to. Without tmux it's the shell itself, and
    // this kills it — the pre-tmux behaviour.
    let _ = killer.kill();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    fn headers_with_origin(origin: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(header::ORIGIN, origin.parse().unwrap());
        h.insert(header::HOST, "127.0.0.1:8000".parse().unwrap());
        h
    }

    /// The app's own origins must pass even though none of them equals `Host`:
    /// the phone's bundle scheme, the Mac's loopback splice on its own port.
    #[test]
    fn our_origins_pass_regardless_of_host() {
        for o in ["virtues://localhost", "tauri://localhost", "http://127.0.0.1:7117"] {
            assert!(check_same_origin(&headers_with_origin(o)).is_none(), "{o}");
        }
    }

    /// A remote page must not ride a paired session to a shell.
    #[test]
    fn a_foreign_origin_is_refused() {
        for o in ["https://evil.example", "http://localhost.evil.example"] {
            assert!(check_same_origin(&headers_with_origin(o)).is_some(), "{o}");
        }
    }

    /// Non-browser clients send no Origin; the route layer's auth still applies.
    #[test]
    fn a_missing_origin_is_allowed() {
        assert!(check_same_origin(&HeaderMap::new()).is_none());
    }

    #[test]
    fn clamps_client_supplied_dimensions() {
        assert_eq!(clamp_dim(Some(120), 80), 120);
        assert_eq!(clamp_dim(None, 80), 80);
        // 0 rows reaches an ioctl and makes curses apps divide by zero.
        assert_eq!(clamp_dim(Some(0), 24), 2);
        assert_eq!(clamp_dim(Some(60000), 80), 1000);
    }

    #[test]
    fn session_path_puts_user_bins_ahead_of_the_service_path() {
        let path = session_path(Some("/home/virtues"));
        let dirs: Vec<&str> = path.split(':').collect();
        assert_eq!(dirs[0], "/home/virtues/.local/bin");
        assert!(dirs.contains(&"/home/virtues/.virtues/bin"));

        // No HOME: fall back to the service PATH untouched, rather than
        // synthesising nonsense like "/.local/bin" from an empty home.
        assert_eq!(session_path(None), std::env::var("PATH").unwrap_or_default());
    }

    #[test]
    fn finds_tmux_only_when_it_is_there() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().to_str().unwrap().to_string();
        assert!(find_tmux(&path).is_none());

        std::fs::write(dir.path().join("tmux"), b"#!/bin/sh\n").unwrap();
        assert_eq!(find_tmux(&path), Some(dir.path().join("tmux")));

        // Empty segments in PATH must not resolve to a relative "tmux".
        assert!(find_tmux("::").is_none());
    }

    fn argv_of(cmd: &CommandBuilder) -> Vec<String> {
        cmd.get_argv()
            .iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn attaches_to_the_shared_tmux_session_when_tmux_exists() {
        let tmux = PathBuf::from("/usr/bin/tmux");
        let cmd = session_command(Some(&tmux), "/usr/bin", Some("/home/virtues"));

        // Nothing but the attach rides in the PTY's command list: tmux aborts a
        // list at the first erroring command, so an option this build didn't
        // recognise would take `new-session` down with it and leave the tab with
        // no shell at all. Options go through configure_tmux instead.
        assert_eq!(
            argv_of(&cmd),
            vec![
                "/usr/bin/tmux",
                "-L",
                TMUX_SOCKET,
                "new-session",
                "-A",
                "-s",
                TMUX_SESSION
            ]
        );
        assert_eq!(cmd.get_env("COLORTERM").unwrap(), "truecolor");
        assert_eq!(cmd.get_env("TERM").unwrap(), "xterm-256color");
    }

    #[test]
    fn tmux_options_are_idempotent_and_scoped_to_our_socket() {
        for exists in [false, true] {
            let args = tmux_config_args(exists).join(" ");

            // Always our own socket, never the user's tmux server.
            assert!(args.starts_with(&format!("-L {TMUX_SOCKET} ")));
            // The wheel scrolls tmux's history; without this it does nothing at
            // all, because tmux owns the screen and xterm's scrollback is empty.
            assert!(args.contains("set-option -g mouse on"));
            // ...as far back as the frontend's scrollback claims to go (tmux's
            // own default is 2000 lines).
            assert!(args.contains("set-option -g history-limit 10000"));
            // xterm.js takes 24-bit colour and tmux can't infer it.
            assert!(args.contains("terminal-overrides ,*:Tc"));
            // This runs on every connect, so no `-ga`: appending is not
            // idempotent and would stack another copy of `*:Tc` each time.
            assert!(!args.contains("-ga"));
        }
    }

    #[test]
    fn tmux_config_creates_the_session_only_when_it_is_missing() {
        // Cold: bring a server up first (a bare `set-option` against an empty
        // socket fails), and create the session last — both because the options
        // must be set before the pane is born to reach it, and because a tmux
        // server with no sessions exits and takes the options with it.
        let cold = tmux_config_args(false).join(" ");
        assert!(cold.starts_with(&format!("-L {TMUX_SOCKET} start-server")));
        assert!(cold.ends_with(&format!("new-session -d -s {TMUX_SESSION}")));

        // Warm: no new-session at all. Against an existing session both `-A -d`
        // and a bare `-d` fail, and a failed command aborts the rest of the list.
        let warm = tmux_config_args(true).join(" ");
        assert!(!warm.contains("new-session"));
        assert!(!warm.contains("start-server"));
    }

    #[test]
    fn falls_back_to_a_login_shell_without_tmux() {
        let cmd = session_command(None, "/usr/bin", None);
        let argv = argv_of(&cmd);

        assert_eq!(argv.len(), 2, "shell + -l, no tmux args: {argv:?}");
        assert_eq!(argv[1], "-l");
        assert_eq!(cmd.get_env("COLORTERM").unwrap(), "truecolor");
    }

    #[test]
    fn paste_extension_cannot_escape_the_paste_dir() {
        assert_eq!(paste_extension(Some("image/png")), "png");
        assert_eq!(paste_extension(Some("image/jpeg; charset=binary")), "jpg");
        assert_eq!(paste_extension(None), "bin");
        // The content type is client-controlled and lands in a filename. The
        // whitelist returns literals, so traversal can't survive it.
        assert_eq!(paste_extension(Some("../../etc/passwd")), "bin");
        assert_eq!(paste_extension(Some("image/png/../../x")), "bin");
    }

    #[test]
    fn stores_pastes_content_addressed_under_home() {
        let home = tempfile::tempdir().unwrap();
        let png = b"\x89PNG\r\n\x1a\n fake";

        let path = store_paste(home.path(), Some("image/png"), png).unwrap();

        // Absolute, under $HOME/.virtues/pastes, and *not* in the drive's media
        // store — this is a shell scratch file, not app-level content.
        assert!(path.is_absolute());
        assert_eq!(path.parent().unwrap(), home.path().join(PASTE_DIR));
        assert_eq!(path.extension().unwrap(), "png");
        assert_eq!(std::fs::read(&path).unwrap(), png);

        // Same bytes -> same path, written once. Pasting a screenshot twice must
        // not litter the directory.
        let again = store_paste(home.path(), Some("image/png"), png).unwrap();
        assert_eq!(again, path);
        assert_eq!(std::fs::read_dir(home.path().join(PASTE_DIR)).unwrap().count(), 1);

        // Different bytes -> different path.
        let other = store_paste(home.path(), Some("image/png"), b"other").unwrap();
        assert_ne!(other, path);
    }

    #[test]
    fn sweep_removes_only_expired_pastes() {
        let dir = tempfile::tempdir().unwrap();
        let fresh = dir.path().join("paste-fresh.png");
        let stale = dir.path().join("paste-stale.png");
        std::fs::write(&fresh, b"x").unwrap();
        std::fs::write(&stale, b"x").unwrap();

        // Backdate one past the TTL.
        let old = std::time::SystemTime::now() - PASTE_TTL - std::time::Duration::from_secs(60);
        std::fs::File::options()
            .write(true)
            .open(&stale)
            .unwrap()
            .set_modified(old)
            .unwrap();

        sweep_old_pastes(dir.path());

        assert!(fresh.exists(), "a fresh paste must survive the sweep");
        assert!(!stale.exists(), "an expired paste must be swept");
    }

    /// The PTY must be born at the client's size: a full-screen TUI reads the
    /// winsize once at startup, so one opened at 80x24 and resized a beat later
    /// paints its first frame into the wrong box.
    #[test]
    fn pty_opens_at_the_size_the_client_asked_for() {
        let size = PtySize {
            cols: 137,
            rows: 42,
            pixel_width: 0,
            pixel_height: 0,
        };
        let pty = native_pty_system().openpty(size).unwrap();

        let mut cmd = CommandBuilder::new("stty");
        cmd.arg("size");
        let mut child = pty.slave.spawn_command(cmd).unwrap();
        drop(pty.slave);

        let mut out = String::new();
        pty.master
            .try_clone_reader()
            .unwrap()
            .read_to_string(&mut out)
            .unwrap();
        child.wait().unwrap();

        // `stty size` prints "rows cols".
        assert_eq!(out.trim(), "42 137");
    }
}
