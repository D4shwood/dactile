//! Dactile on a server: the desktop's commands (`dactile_backend`) over HTTP
//! for one person behind a password, and the web interface beside them.
//!
//! The browser calls `POST /api/call/{name}` with the same arguments the
//! desktop's window sends through Tauri. What a desktop does with its own
//! file dialogs and programs becomes uploads and downloads here.

pub mod confine;

use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, Path as Url, Query, Request, State};
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use dactile_backend::{Ctx, importing, play};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

const COOKIE: &str = "dactile";

/// One person's server: their folder, their password, their open play.
pub struct Server {
    /// Everything of theirs, canonical: plays, uploads, settings, cache.
    pub root: PathBuf,
    pub password: String,
    /// The cookie gets `Secure` behind HTTPS.
    pub secure: bool,
    pub ctx: Ctx,
    sessions: Mutex<HashSet<String>>,
    /// The OCR's pages read and to read, while a document is being read.
    progress: (AtomicUsize, AtomicUsize),
}

type Shared = Arc<Server>;

impl Server {
    pub fn new(root: PathBuf, password: String, secure: bool) -> Server {
        Server {
            root,
            password,
            secure,
            ctx: Ctx::default(),
            sessions: Mutex::new(HashSet::new()),
            progress: (AtomicUsize::new(0), AtomicUsize::new(0)),
        }
    }

    pub fn plays(&self) -> PathBuf {
        self.root.join("plays")
    }

    fn uploads(&self) -> PathBuf {
        self.root.join("uploads")
    }
}

/// The folders and settings of `root`, made before anything reads them: the
/// desktop's code finds its settings, cache and trash through these
/// variables, so they must be set while there is only one thread.
pub fn prepare(root: &Path) -> anyhow::Result<PathBuf> {
    std::fs::create_dir_all(root)?;
    let root = root.canonicalize()?;
    for d in ["plays", "uploads", "config", "cache", "share", "home"] {
        std::fs::create_dir_all(root.join(d))?;
    }
    // SAFETY: called from `main` before the runtime starts its threads.
    unsafe {
        std::env::set_var("HOME", root.join("home"));
        std::env::set_var("XDG_CONFIG_HOME", root.join("config"));
        std::env::set_var("XDG_CACHE_HOME", root.join("cache"));
        std::env::set_var("XDG_DATA_HOME", root.join("share"));
        std::env::set_var("DACTILE_CONFIG", root.join("config/dactile/config.toml"));
    }
    let fresh = !dactile::config::path().exists();
    let mut cfg = dactile::config::Config::read();
    let plays = root.join("plays").display().to_string();
    if fresh || cfg.plays_dir != plays {
        cfg.plays_dir = plays;
        // The server's own language says nothing of the person's.
        if fresh {
            cfg.lang = "tr".into();
        }
        cfg.save()?;
    }
    Ok(root)
}

pub fn router(server: Shared, web: Option<PathBuf>) -> Router {
    let api = Router::new()
        .route("/call/{name}", post(call))
        .route("/import_read", post(import_read))
        .route("/progress", get(progress))
        .route("/upload/{kind}", post(upload))
        .route("/file", get(file))
        .route("/log", post(log))
        .route("/version", get(version))
        .route("/logout", post(logout))
        .route_layer(middleware::from_fn_with_state(server.clone(), guard))
        .route("/login", post(login))
        .layer(DefaultBodyLimit::max(64 << 20));
    let app = Router::new().nest("/api", api).with_state(server);
    match web {
        Some(dir) => app.fallback_service(
            tower_http::services::ServeDir::new(&dir)
                .fallback(tower_http::services::ServeFile::new(dir.join("index.html"))),
        ),
        None => app,
    }
}

/// A command's error, as the desktop's window gets it: its message.
fn fail(code: StatusCode, message: impl Into<String>) -> Response {
    (code, Json(Value::String(message.into()))).into_response()
}

fn session(headers: &HeaderMap) -> Option<String> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .filter_map(|c| c.trim().split_once('='))
        .find(|(k, _)| *k == COOKIE)
        .map(|(_, v)| v.to_string())
}

/// Only a signed-in browser gets through, and a write must carry the
/// interface's own header: another site's form can't add it.
async fn guard(State(s): State<Shared>, req: Request, next: Next) -> Response {
    let known = session(req.headers()).is_some_and(|t| s.sessions.lock().unwrap().contains(&t));
    if !known {
        return fail(StatusCode::UNAUTHORIZED, "signed out");
    }
    let reads = [Method::GET, Method::HEAD].contains(req.method());
    if !reads && !req.headers().contains_key("x-dactile") {
        return fail(StatusCode::FORBIDDEN, "no x-dactile header");
    }
    next.run(req).await
}

fn same(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0, |d, (x, y)| d | (x ^ y)) == 0
}

fn token() -> anyhow::Result<String> {
    use std::io::Read;
    let mut b = [0u8; 32];
    std::fs::File::open("/dev/urandom")?.read_exact(&mut b)?;
    Ok(b.iter().map(|x| format!("{x:02x}")).collect())
}

#[derive(Deserialize)]
struct Login {
    password: String,
}

async fn login(State(s): State<Shared>, Json(l): Json<Login>) -> Response {
    if !same(&l.password, &s.password) {
        // Slows a guesser down.
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        return fail(StatusCode::UNAUTHORIZED, "wrong password");
    }
    let Ok(t) = token() else {
        return fail(StatusCode::INTERNAL_SERVER_ERROR, "no randomness");
    };
    s.sessions.lock().unwrap().insert(t.clone());
    let secure = if s.secure { "; Secure" } else { "" };
    let cookie =
        format!("{COOKIE}={t}; Path=/; HttpOnly; SameSite=Strict; Max-Age=2592000{secure}");
    ([(header::SET_COOKIE, cookie)], Json(json!(true))).into_response()
}

async fn logout(State(s): State<Shared>, headers: HeaderMap) -> Response {
    if let Some(t) = session(&headers) {
        s.sessions.lock().unwrap().remove(&t);
    }
    let gone = format!("{COOKIE}=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0");
    ([(header::SET_COOKIE, gone)], Json(json!(true))).into_response()
}

/// One of the backend's commands, off the async threads: some run XeLaTeX.
async fn call(
    State(s): State<Shared>,
    Url(name): Url<String>,
    Json(args): Json<Value>,
) -> Response {
    if let Err(e) = confine::check(&s.root, &name, &args) {
        return fail(StatusCode::FORBIDDEN, e);
    }
    let done =
        tokio::task::spawn_blocking(move || dactile_backend::dispatch(&s.ctx, &name, args)).await;
    match done {
        Ok(Ok(v)) => Json(v).into_response(),
        Ok(Err(e)) => fail(StatusCode::BAD_REQUEST, e),
        Err(e) => fail(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

#[derive(Deserialize)]
struct Read {
    path: String,
    ocr: bool,
}

/// Reads an uploaded document and keeps its draft for the preview, as the
/// desktop's `import_read`; the OCR's pages are counted for `progress`.
async fn import_read(State(s): State<Shared>, Json(r): Json<Read>) -> Response {
    let file = match confine::inside(&s.root, &r.path) {
        Ok(f) => f,
        Err(e) => return fail(StatusCode::FORBIDDEN, e),
    };
    let done = tokio::task::spawn_blocking(move || {
        s.progress.0.store(0, Ordering::Relaxed);
        s.progress.1.store(0, Ordering::Relaxed);
        let lang = play::lang_of(&s.ctx.lock().cfg);
        let draft = importing::read(&file, lang, r.ocr, &|done, total| {
            s.progress.0.store(done, Ordering::Relaxed);
            s.progress.1.store(total, Ordering::Relaxed);
        })?;
        let shown = importing::shown(&file, &draft);
        s.ctx.lock().importing = Some((file, draft));
        Ok::<_, String>(shown)
    })
    .await;
    match done {
        Ok(Ok(shown)) => Json(shown).into_response(),
        Ok(Err(e)) => fail(StatusCode::BAD_REQUEST, e),
        Err(e) => fail(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

async fn progress(State(s): State<Shared>) -> Json<(usize, usize)> {
    Json((
        s.progress.0.load(Ordering::Relaxed),
        s.progress.1.load(Ordering::Relaxed),
    ))
}

/// A file from the browser: a play (`.tex`) gets its own folder among the
/// plays, a document to import waits in the uploads. The answer is its path.
async fn upload(
    State(s): State<Shared>,
    Url(kind): Url<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let name = headers
        .get("x-name")
        .and_then(|v| v.to_str().ok())
        .map(|v| {
            percent_encoding::percent_decode_str(v)
                .decode_utf8_lossy()
                .to_string()
        })
        .unwrap_or_default();
    let name = Path::new(&name)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let ext = Path::new(&name)
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let stem = Path::new(&name)
        .file_stem()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let to = match kind.as_str() {
        "play" if ext == "tex" => {
            let stem = dactile::library::file_name(&stem);
            let stem = if stem.is_empty() {
                "oyun".to_string()
            } else {
                stem
            };
            dactile::library::new_home(&s.plays(), &stem)
        }
        "import" if dactile::import::EXTENSIONS.contains(&ext.as_str()) => {
            let stem = dactile::library::file_name(&stem);
            let stem = if stem.is_empty() {
                "belge".to_string()
            } else {
                stem
            };
            dactile::library::new_folder(&s.uploads(), &stem)
                .map(|d| d.join(format!("{stem}.{ext}")))
        }
        _ => {
            return fail(
                StatusCode::BAD_REQUEST,
                dactile::lang::l(
                    "Bu dosya türü yüklenemez.",
                    "That kind of file can't be uploaded.",
                ),
            );
        }
    };
    let written = to.and_then(|p| std::fs::write(&p, &body).map(|_| p));
    match written {
        Ok(p) => Json(p.display().to_string()).into_response(),
        Err(e) => fail(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

#[derive(Deserialize)]
struct Wanted {
    path: String,
    #[serde(default)]
    download: bool,
}

fn kind(p: &Path) -> &'static str {
    match p.extension().and_then(|e| e.to_str()).unwrap_or("") {
        "pdf" => "application/pdf",
        "tex" | "log" | "txt" | "md" | "toml" => "text/plain; charset=utf-8",
        "csv" => "text/csv; charset=utf-8",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "odt" => "application/vnd.oasis.opendocument.text",
        _ => "application/octet-stream",
    }
}

/// A file of theirs to the browser: shown in a tab, or saved.
async fn file(State(s): State<Shared>, Query(w): Query<Wanted>) -> Response {
    let p = match confine::inside(&s.root, &w.path) {
        Ok(p) => p,
        Err(e) => return fail(StatusCode::FORBIDDEN, e),
    };
    if p.is_dir() {
        return fail(
            StatusCode::BAD_REQUEST,
            dactile::lang::l(
                "Klasörler tarayıcıda açılamaz.",
                "Folders can't be opened in the browser.",
            ),
        );
    }
    let bytes = match tokio::fs::read(&p).await {
        Ok(b) => b,
        Err(e) => return fail(StatusCode::NOT_FOUND, e.to_string()),
    };
    let name = p
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let encoded = percent_encoding::utf8_percent_encode(&name, percent_encoding::NON_ALPHANUMERIC);
    let how = if w.download { "attachment" } else { "inline" };
    let disposition = format!("{how}; filename*=UTF-8''{encoded}");
    let mut r = bytes.into_response();
    let h = r.headers_mut();
    h.insert(header::CONTENT_TYPE, HeaderValue::from_static(kind(&p)));
    if let Ok(v) = HeaderValue::from_str(&disposition) {
        h.insert(header::CONTENT_DISPOSITION, v);
    }
    h.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    r
}

#[derive(Deserialize)]
struct Message {
    message: String,
}

async fn log(Json(m): Json<Message>) -> Json<Value> {
    eprintln!("[window] {}", m.message);
    Json(Value::Null)
}

async fn version() -> Json<&'static str> {
    Json(env!("CARGO_PKG_VERSION"))
}
