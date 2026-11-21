//! Dactile on a server: the desktop's commands (`dactile_backend`) over HTTP
//! for one person behind a password, and the web interface beside them.
//!
//! The browser calls `POST /api/call/{name}` with the same arguments the
//! desktop's window sends through Tauri. What a desktop does with its own
//! file dialogs and programs becomes uploads and downloads here.

pub mod confine;
pub mod door;
pub mod gate;
pub mod google;
pub mod mail;
pub mod people;
pub mod projects;
pub mod room;

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
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

pub const COOKIE: &str = "dactile";
/// What a linked desktop and this site must agree on: raised when a command
/// or a route changes so that an older desktop would go wrong with it.
pub const API: u32 = 1;
/// The header the gate's word comes in.
pub const GATE: &str = "x-dactile-gate";

/// One person's server: their folder, their password, their open play.
pub struct Server {
    /// Everything of theirs, canonical: plays, uploads, settings, cache.
    pub root: PathBuf,
    pub password: String,
    /// The cookie gets `Secure` behind HTTPS.
    pub secure: bool,
    /// Behind the gate (`gate.rs`), which has signed the person in already:
    /// the word it sends with every request, and the only way in.
    pub gate: Option<String>,
    pub ctx: Ctx,
    /// Their folder and the projects' they reach (`with_projects`).
    pub roots: confine::Roots,
    /// The projects they are in, as the gate said when it started this.
    pub projects: Vec<projects::Reach>,
    door: door::Door,
    /// The OCR's pages read and to read, while a document is being read.
    progress: (AtomicUsize, AtomicUsize),
}

type Shared = Arc<Server>;

impl Server {
    pub fn new(root: PathBuf, password: String, secure: bool) -> Server {
        let ctx = Ctx::default();
        // Their settings from the first command on, whichever it is: the
        // interface asks for the start screen first, another caller may not.
        ctx.lock().cfg = play::load_config();
        Server {
            door: door::Door::open(root.join("cache/sessions")),
            roots: confine::Roots::alone(root.clone()),
            projects: Vec::new(),
            root,
            password,
            secure,
            gate: None,
            ctx,
            progress: (AtomicUsize::new(0), AtomicUsize::new(0)),
        }
    }

    /// One person's server behind the gate: no password of its own.
    pub fn behind(root: PathBuf, gate: String) -> Server {
        let mut s = Server::new(root, String::new(), false);
        s.gate = Some(gate);
        s
    }

    /// The projects the gate says they are in: their plays' folders are
    /// reached too, those they are a reader of only to read. A folder that
    /// isn't there is left out.
    pub fn with_projects(mut self, reach: Vec<projects::Reach>) -> Server {
        for mut r in reach {
            let Ok(dir) = r.dir.canonicalize() else {
                continue;
            };
            r.dir = dir.clone();
            let writes = r.role.writes();
            self.roots.shared.push((dir.clone(), writes));
            if writes {
                self.ctx.lock().live.push(dir);
            } else {
                self.ctx.lock().reads_only.push(dir);
            }
            self.projects.push(r);
        }
        self
    }

    /// The projects and their plays, for the start screen: each read as
    /// the person's own plays are, from its folder alone.
    fn projects_home(&self) -> Value {
        let cfg = self.ctx.lock().cfg.clone();
        let all: Vec<Value> = self
            .projects
            .iter()
            .map(|r| {
                let mut c = cfg.clone();
                c.plays_dir = r.dir.display().to_string();
                c.recent.clear();
                let plays = serde_json::to_value(play::home(&c))
                    .ok()
                    .and_then(|h| h.get("plays").cloned())
                    .unwrap_or_else(|| json!([]));
                json!({
                    "id": r.id, "name": r.name, "role": r.role, "mode": r.mode,
                    "folder": r.dir.display().to_string(), "plays": plays,
                })
            })
            .collect();
        Value::Array(all)
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
        .route("/lang", get(lang))
        .route("/logout", post(logout))
        .route("/me", get(me))
        .route_layer(middleware::from_fn_with_state(server.clone(), guard))
        .route("/login", post(login))
        .route("/door", get(door_kind))
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
pub fn fail(code: StatusCode, message: impl Into<String>) -> Response {
    (code, Json(Value::String(message.into()))).into_response()
}

/// The session a request carries: a browser's in its cookie, a linked
/// desktop's as `Authorization: Bearer`. A desktop's token never rides
/// along on its own, as a cookie does, so another site can't use it.
pub fn session(headers: &HeaderMap) -> Option<String> {
    let bearer = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty());
    if bearer.is_some() {
        return bearer;
    }
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
    let known = match &s.gate {
        Some(word) => req
            .headers()
            .get(GATE)
            .and_then(|v| v.to_str().ok())
            .is_some_and(|said| same(said, word)),
        None => session(req.headers()).is_some_and(|t| s.door.signed_in(&t)),
    };
    if !known {
        return fail(StatusCode::UNAUTHORIZED, "signed out");
    }
    let reads = [Method::GET, Method::HEAD].contains(req.method());
    if !reads && !req.headers().contains_key("x-dactile") {
        return fail(StatusCode::FORBIDDEN, "no x-dactile header");
    }
    next.run(req).await
}

use door::same;

/// Where the request came from, as the web server in front says it (it
/// writes the last of these itself); "local" with none in front.
pub fn address(headers: &HeaderMap) -> String {
    headers
        .get_all("x-forwarded-for")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(','))
        .map(|a| a.trim().to_string())
        .rfind(|a| !a.is_empty())
        .unwrap_or_else(|| "local".into())
}

#[derive(Deserialize)]
struct Login {
    password: String,
}

async fn login(State(s): State<Shared>, headers: HeaderMap, Json(l): Json<Login>) -> Response {
    if s.gate.is_some() {
        return fail(StatusCode::NOT_FOUND, "the gate signs in");
    }
    let from = address(&headers);
    if s.door.waiting(&from) {
        return fail(
            StatusCode::TOO_MANY_REQUESTS,
            dactile::lang::l(
                "Çok fazla yanlış deneme: on beş dakika sonra yine dene.",
                "Too many wrong tries: try again in fifteen minutes.",
            ),
        );
    }
    if !same(&l.password, &s.password) {
        s.door.wrong(&from);
        // Slows a guesser down.
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        return fail(StatusCode::UNAUTHORIZED, "wrong password");
    }
    s.door.right(&from);
    let Ok(t) = s.door.sign_in("") else {
        return fail(StatusCode::INTERNAL_SERVER_ERROR, "no randomness");
    };
    let secure = if s.secure { "; Secure" } else { "" };
    let life = door::SESSION.as_secs();
    let cookie = format!("{COOKIE}={t}; Path=/; HttpOnly; SameSite=Strict; Max-Age={life}{secure}");
    ([(header::SET_COOKIE, cookie)], Json(json!(true))).into_response()
}

async fn logout(State(s): State<Shared>, headers: HeaderMap) -> Response {
    if let Some(t) = session(&headers) {
        s.door.sign_out(&t);
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
    let open_reads = s
        .ctx
        .lock()
        .open
        .as_ref()
        .is_some_and(|o| !s.roots.writes_at(&o.path));
    if let Err(e) = confine::check(&s.roots, &name, &args, open_reads) {
        return fail(StatusCode::FORBIDDEN, e);
    }
    let home = name == "home";
    let root = s.root.clone();
    let server = s.clone();
    let done =
        tokio::task::spawn_blocking(move || dactile_backend::dispatch(&s.ctx, &name, args)).await;
    match done {
        Ok(Ok(mut v)) => {
            if home {
                // A project's plays are under the project, not among theirs,
                // though they were opened lately.
                if let Some(Value::Array(plays)) = v.get_mut("plays") {
                    plays.retain(|p| {
                        let path = Path::new(p["path"].as_str().unwrap_or(""));
                        !server.roots.shared.iter().any(|(d, _)| path.starts_with(d))
                    });
                }
                v["projects"] = server.projects_home();
            }
            // The start screen's one line of warning, if it has none of its
            // own: the backup's.
            if home
                && v.get("notice").is_some_and(Value::is_null)
                && let Some(n) = backup_notice(&root, std::time::SystemTime::now())
            {
                v["notice"] = Value::String(n);
            }
            Json(v).into_response()
        }
        Ok(Err(e)) => fail(StatusCode::BAD_REQUEST, e),
        Err(e) => fail(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

/// A backup is late after this: it runs every hour.
const BACKUP_LATE: u64 = 26 * 3600;

/// What is wrong with the backup, from what `deploy/backup.sh` last wrote
/// in `cache/backup` of the folder it backs up, or in `$DACTILE_BACKUP` (`ok=` the second it last went well, `away=1` if it
/// leaves this machine, `error=` what stopped it). Nothing if all is well,
/// or if no backup has ever run here.
pub fn backup_notice(root: &Path, now: std::time::SystemTime) -> Option<String> {
    // Behind the gate one backup takes everyone's folders: it says where.
    let file =
        std::env::var_os("DACTILE_BACKUP").map_or_else(|| root.join("cache/backup"), PathBuf::from);
    let said = std::fs::read_to_string(file).ok()?;
    let get = |key: &str| {
        said.lines()
            .find_map(|l| l.strip_prefix(key)?.strip_prefix('='))
            .map(str::trim)
            .filter(|v| !v.is_empty())
    };
    let ok: Option<u64> = get("ok").and_then(|v| v.parse().ok());
    let now = now
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let ago = |then: u64| {
        let hours = now.saturating_sub(then) / 3600;
        if hours < 48 {
            dactile::lang::f("{} saat önce", "{} hours ago", &[&hours.to_string()])
        } else {
            dactile::lang::f("{} gün önce", "{} days ago", &[&(hours / 24).to_string()])
        }
    };
    let last = ok.map_or_else(|| dactile::lang::l("hiç", "never").to_string(), ago);
    if let Some(why) = get("error") {
        return Some(dactile::lang::f(
            "Yedek alınamıyor ({}). Son başarılı yedek: {}.",
            "The backup is failing ({}). The last one that went well: {}.",
            &[&why, &last],
        ));
    }
    if ok.is_none_or(|then| now.saturating_sub(then) > BACKUP_LATE) {
        return Some(dactile::lang::f(
            "Yedek gecikti. Son başarılı yedek: {}.",
            "The backup is late. The last one that went well: {}.",
            &[&last],
        ));
    }
    if get("away") != Some("1") {
        return Some(
            dactile::lang::l(
                "Yedek yalnızca bu makinede duruyor: dışarıda bir yere (git ya da Drive) henüz bağlanmadı.",
                "The backup stays on this machine only: it isn't connected to anywhere outside (git or Drive) yet.",
            )
            .to_string(),
        );
    }
    None
}

#[derive(Deserialize)]
struct Read {
    path: String,
    ocr: bool,
}

/// Reads an uploaded document and keeps its draft for the preview, as the
/// desktop's `import_read`; the OCR's pages are counted for `progress`.
async fn import_read(State(s): State<Shared>, Json(r): Json<Read>) -> Response {
    let file = match s.roots.place(&r.path) {
        Ok((f, _)) => f,
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

/// A document waits in the uploads this long after it came: time enough to
/// read it again another way, and then it is the play that is kept.
const UPLOAD_DAYS: u64 = 7;
/// All the waiting documents together: past this the oldest go first.
const UPLOADS_BYTES: u64 = 1 << 30;

fn folder_bytes(dir: &Path) -> u64 {
    std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| e.metadata().ok())
        .filter(|m| m.is_file())
        .map(|m| m.len())
        .sum()
}

/// Removes the uploads' folders that have waited too long, and the oldest
/// of the rest while they are more than `room` together.
pub fn sweep_uploads(dir: &Path, age: std::time::Duration, room: u64) {
    let mut folders: Vec<(std::time::SystemTime, PathBuf, u64)> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .filter_map(|e| {
            let came = e.metadata().and_then(|m| m.modified()).ok()?;
            Some((came, e.path(), folder_bytes(&e.path())))
        })
        .collect();
    folders.sort();
    let mut all: u64 = folders.iter().map(|f| f.2).sum();
    for (came, path, bytes) in folders {
        let old = came.elapsed().is_ok_and(|waited| waited > age);
        if old || all > room {
            // Only ever a folder straight inside the uploads.
            if path.parent() == Some(dir) && std::fs::remove_dir_all(&path).is_ok() {
                all -= bytes;
            }
        }
    }
}

/// All the bytes under `dir`, links not followed.
pub fn bytes_under(dir: &Path) -> u64 {
    std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| match e.file_type() {
            Ok(t) if t.is_dir() => bytes_under(&e.path()),
            Ok(t) if t.is_file() => e.metadata().map_or(0, |m| m.len()),
            _ => 0,
        })
        .sum()
}

/// How much one person may keep here, from `DACTILE_QUOTA_MB`; no limit
/// unless it says. Past it nothing more can be uploaded. Writing is never
/// stopped by it: a play is small, and theirs to save.
fn quota() -> Option<u64> {
    let mb: u64 = std::env::var("DACTILE_QUOTA_MB").ok()?.parse().ok()?;
    Some(mb << 20)
}

/// Whether `more` bytes still fit in `root` under a limit of `room`.
pub fn fits(root: &Path, more: u64, room: u64) -> bool {
    bytes_under(root).saturating_add(more) <= room
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
    if let Some(room) = quota()
        && !fits(&s.root, body.len() as u64, room)
    {
        return fail(
            StatusCode::PAYLOAD_TOO_LARGE,
            dactile::lang::l(
                "Yerin doldu: yeni bir dosya yüklemeden önce eski oyunlardan ya da belgelerden yer aç.",
                "Your space is full: make room among the old plays or documents before uploading another file.",
            ),
        );
    }
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
            sweep_uploads(
                &s.uploads(),
                std::time::Duration::from_secs(UPLOAD_DAYS * 24 * 3600),
                UPLOADS_BYTES.saturating_sub(body.len() as u64),
            );
            let stem = dactile::library::file_name(&stem);
            let stem = if stem.is_empty() {
                "belge".to_string()
            } else {
                stem
            };
            // `file_name` makes a play's name of it: the folder and the
            // document take it without the `.tex`.
            let stem = stem.strip_suffix(".tex").unwrap_or(&stem);
            let stem = if stem.is_empty() { "belge" } else { stem }.to_string();
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
    let p = match s.roots.place(&w.path) {
        Ok((p, _)) => p,
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

/// How one signs in here, for the page that asks: with the password.
async fn door_kind() -> Json<Value> {
    Json(json!({ "kind": "password" }))
}

/// Who is signed in: with one password for the one person, no one by name.
async fn me() -> Json<Value> {
    Json(json!({ "name": null }))
}

/// The interface's language as it is now: the settings say it, or with
/// "auto" the open play does. The browser's own commands follow it.
async fn lang() -> Json<&'static str> {
    Json(match dactile::lang::get() {
        dactile::lang::Lang::En => "en",
        dactile::lang::Lang::Tr => "tr",
    })
}

async fn version() -> Json<Value> {
    Json(serde_json::json!({ "version": env!("CARGO_PKG_VERSION"), "api": API }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_backup_speaks_only_when_something_is_wrong() {
        let root = std::env::temp_dir().join(format!("dactile-backup-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("cache")).unwrap();
        let now = std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_800_000_000);
        let said = |text: &str| {
            std::fs::write(root.join("cache/backup"), text).unwrap();
            backup_notice(&root, now)
        };
        // No backup has run here: nothing to say.
        assert_eq!(backup_notice(&root, now), None);
        // An hour ago, off the machine: all is well.
        assert_eq!(said("ok=1799996400\naway=1\nerror=\n"), None);
        // Well, but on this machine alone.
        assert!(said("ok=1799996400\naway=0\n").is_some());
        // Three days ago.
        let late = said("ok=1799740800\naway=1\n").unwrap();
        assert!(late.contains('3'), "{late}");
        // Stopped by something.
        let failing = said("ok=1799996400\naway=1\nerror=git push\n").unwrap();
        assert!(failing.contains("git push"), "{failing}");
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn what_a_person_keeps_is_counted_folder_by_folder() {
        let dir = std::env::temp_dir().join(format!("dactile-quota-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("plays/kar")).unwrap();
        std::fs::write(dir.join("plays/kar/kar.tex"), vec![0u8; 700]).unwrap();
        std::fs::write(dir.join("a.txt"), vec![0u8; 300]).unwrap();
        // A link to something large elsewhere counts for nothing.
        std::os::unix::fs::symlink("/usr", dir.join("elsewhere")).unwrap();
        assert_eq!(bytes_under(&dir), 1000);
        assert!(fits(&dir, 24, 1024));
        assert!(!fits(&dir, 25, 1024));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn old_uploads_go_and_the_oldest_when_there_is_no_room() {
        let dir = std::env::temp_dir().join(format!("dactile-uploads-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let put = |name: &str, bytes: usize, days_ago: u64| {
            let d = dir.join(name);
            std::fs::create_dir_all(&d).unwrap();
            std::fs::write(d.join("belge.pdf"), vec![0u8; bytes]).unwrap();
            let when =
                std::time::SystemTime::now() - std::time::Duration::from_secs(days_ago * 24 * 3600);
            std::fs::File::open(&d).unwrap().set_modified(when).unwrap();
        };
        put("eski", 10, 9);
        put("dun", 600, 1);
        put("bugun", 600, 0);
        std::fs::write(dir.join("a-file"), "left alone").unwrap();
        let week = std::time::Duration::from_secs(7 * 24 * 3600);
        // Room for both of the new ones: only the old one goes.
        sweep_uploads(&dir, week, 2000);
        assert!(!dir.join("eski").exists());
        assert!(dir.join("dun").exists() && dir.join("bugun").exists());
        // Room for one: the older goes.
        sweep_uploads(&dir, week, 1000);
        assert!(!dir.join("dun").exists());
        assert!(dir.join("bugun").exists());
        assert!(dir.join("a-file").exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
