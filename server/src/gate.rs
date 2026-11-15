//! The gate: one site for many people, each with a Dactile of their own.
//!
//! The backend keeps one person's settings, cache and open play for a whole
//! process. So a person gets a process: a `dactile-server` of their own, on
//! a port only this machine reaches, over their own folder
//! (`people/<id>/`). The gate is what the browser talks to. It knows who is
//! signed in, starts their server when they come, passes their requests on
//! with a word only it and that server know, and stops the server when they
//! have been away a while.
//!
//! Signing in is by a link the owner makes (`dactile-gate invite ada`),
//! good once, or with the Google account whose address the owner put on
//! the list (`google.rs`). No passwords are kept.

use crate::door::{self, Door};
use crate::google::Google;
use crate::people::{self, Person};
use crate::{COOKIE, GATE, address, fail, session};
use axum::body::Body;
use axum::extract::{Path as Url, Request, State};
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode, Uri, header};
use axum::middleware::{self, Next};
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::routing::{any, get, post};
use axum::{Extension, Json, Router};
use hyper_util::client::legacy::Client;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::rt::TokioExecutor;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

/// A person's server, while it runs.
struct Theirs {
    port: u16,
    /// What the gate says to it with every request: its only way in.
    word: String,
    process: std::sync::Mutex<std::process::Child>,
    /// The second it was last asked something.
    asked: AtomicU64,
}

impl Theirs {
    fn running(&self) -> bool {
        let mut p = self.process.lock().unwrap_or_else(|e| e.into_inner());
        matches!(p.try_wait(), Ok(None))
    }
}

impl Drop for Theirs {
    fn drop(&mut self) {
        let mut p = self.process.lock().unwrap_or_else(|e| e.into_inner());
        let _ = p.kill();
        let _ = p.wait();
    }
}

pub struct Gate {
    /// Everything: `gate/` (the list, the sessions, the links) and
    /// `people/<id>/` (each person's own).
    pub home: PathBuf,
    /// The cookie gets `Secure` behind HTTPS.
    pub secure: bool,
    /// The `dactile-server` program each person gets.
    pub server: PathBuf,
    /// A server nobody has asked anything this long is stopped.
    pub idle: Duration,
    /// Signing in with Google, where it is set up.
    pub google: Option<Google>,
    door: Door,
    running: tokio::sync::Mutex<HashMap<String, Arc<Theirs>>>,
    client: Client<HttpConnector, Body>,
}

type Shared = Arc<Gate>;

impl Gate {
    pub fn new(home: PathBuf, server: PathBuf, secure: bool) -> Gate {
        Gate {
            door: Door::open(home.join("gate/secret/sessions")),
            home,
            secure,
            server,
            idle: Duration::from_secs(20 * 60),
            google: None,
            running: tokio::sync::Mutex::new(HashMap::new()),
            client: Client::builder(TokioExecutor::new()).build(HttpConnector::new()),
        }
    }

    /// The person's server, started if it isn't running.
    async fn theirs(&self, id: &str) -> Result<Arc<Theirs>, String> {
        let mut running = self.running.lock().await;
        if let Some(t) = running.get(id) {
            if t.running() {
                t.asked.store(door::now(), Ordering::Relaxed);
                return Ok(t.clone());
            }
            running.remove(id);
        }
        let folder = people::folder(&self.home, id);
        std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
        // A port nobody has: asked for, looked at, given back.
        let port = std::net::TcpListener::bind("127.0.0.1:0")
            .and_then(|l| l.local_addr())
            .map_err(|e| e.to_string())?
            .port();
        let word = door::random().map_err(|e| e.to_string())?;
        let mut c = std::process::Command::new(&self.server);
        c.env("DACTILE_DATA", &folder)
            .env("DACTILE_ADDR", format!("127.0.0.1:{port}"))
            .env("DACTILE_GATE_WORD", &word)
            // The gate serves the pages itself, and has no password to give.
            .env_remove("DACTILE_WEB")
            .env_remove("DACTILE_PASSWORD")
            // What is the gate's alone stays with the gate.
            .env_remove("DACTILE_GOOGLE_SECRET")
            .env_remove("DACTILE_GOOGLE_ID")
            // Held open for as long as the gate lives: the server leaves
            // when it closes, however the gate went.
            .stdin(std::process::Stdio::piped());
        let process = c
            .spawn()
            .map_err(|e| format!("{}: {e}", self.server.display()))?;
        let theirs = Arc::new(Theirs {
            port,
            word,
            process: std::sync::Mutex::new(process),
            asked: AtomicU64::new(door::now()),
        });
        // It answers once it has read its settings: a moment.
        for _ in 0..200 {
            if tokio::net::TcpStream::connect(("127.0.0.1", port))
                .await
                .is_ok()
            {
                running.insert(id.to_string(), theirs.clone());
                return Ok(theirs);
            }
            if !theirs.running() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        Err(format!("the server for {id} didn't start"))
    }

    /// Stops the servers nobody has asked anything for `idle`, and those of
    /// people taken off the list.
    pub async fn rest(&self) {
        let listed: Vec<String> = people::list(&self.home).into_iter().map(|p| p.id).collect();
        let before = door::now().saturating_sub(self.idle.as_secs());
        self.running.lock().await.retain(|id, t| {
            listed.contains(id) && t.asked.load(Ordering::Relaxed) > before && t.running()
        });
    }

    /// The port of a person's server, if it runs.
    pub async fn port_of(&self, id: &str) -> Option<u16> {
        self.running.lock().await.get(id).map(|t| t.port)
    }

    /// How many people's servers are running.
    pub async fn awake(&self) -> usize {
        self.running.lock().await.len()
    }
}

pub fn router(gate: Shared, web: Option<PathBuf>) -> Router {
    let inside = Router::new()
        .route("/me", get(me))
        .route("/version", get(version))
        .route("/logout", post(logout))
        // What isn't the gate's own is the person's server's.
        .route("/{*rest}", any(pass))
        .route_layer(middleware::from_fn_with_state(gate.clone(), guard));
    let api = Router::new()
        .route("/door", get(door_kind))
        .route("/login", post(no_password))
        .merge(inside);
    let app = Router::new()
        .nest("/api", api)
        .route("/giris/google", get(to_google))
        .route("/giris/google/donus", get(from_google))
        .route("/giris/{token}", get(link).post(come_in))
        .with_state(gate);
    match web {
        Some(dir) => app.fallback_service(
            tower_http::services::ServeDir::new(&dir)
                .fallback(tower_http::services::ServeFile::new(dir.join("index.html"))),
        ),
        None => app,
    }
}

/// Only a signed-in browser of someone still on the list gets through, and
/// a write must carry the interface's own header.
async fn guard(State(g): State<Shared>, mut req: Request, next: Next) -> Response {
    let person = session(req.headers())
        .and_then(|t| g.door.who(&t))
        .and_then(|id| people::find(&g.home, &id));
    let Some(person) = person else {
        return fail(StatusCode::UNAUTHORIZED, "signed out");
    };
    let reads = [Method::GET, Method::HEAD].contains(req.method());
    if !reads && !req.headers().contains_key("x-dactile") {
        return fail(StatusCode::FORBIDDEN, "no x-dactile header");
    }
    req.extensions_mut().insert(person);
    next.run(req).await
}

/// How one signs in here, for the page that asks: with a link, and with
/// Google where that is set up.
async fn door_kind(State(g): State<Shared>) -> Json<Value> {
    Json(json!({ "kind": "link", "google": g.google.is_some() }))
}

async fn no_password() -> Response {
    fail(StatusCode::NOT_FOUND, "one signs in with a link here")
}

async fn me(Extension(p): Extension<Person>) -> Json<Value> {
    Json(json!({ "id": p.id, "name": p.name, "role": p.role }))
}

async fn version() -> Json<&'static str> {
    Json(env!("CARGO_PKG_VERSION"))
}

async fn logout(State(g): State<Shared>, headers: HeaderMap) -> Response {
    if let Some(t) = session(&headers) {
        g.door.sign_out(&t);
    }
    let gone = format!("{COOKIE}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0");
    ([(header::SET_COOKIE, gone)], Json(json!(true))).into_response()
}

fn page(title: &str, body: &str) -> Html<String> {
    Html(format!(
        r#"<!doctype html><html lang="tr"><head><meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0"><meta name="robots" content="noindex">
<title>Dactile</title></head>
<body style="max-width:22rem;margin:20vh auto;padding:0 1rem;font-family:system-ui,sans-serif">
<h1 style="font-family:serif;letter-spacing:.3em;margin:0 0 1.5rem">DACTILE</h1>
<p style="font-size:1.1rem;margin:0 0 1rem">{title}</p>{body}</body></html>"#
    ))
}

fn no_such_link() -> Response {
    (
        StatusCode::NOT_FOUND,
        page(
            "Bu bağlantı geçersiz: kullanılmış ya da süresi dolmuş olabilir.",
            "<p>Yenisini site sahibinden isteyebilirsin.</p>",
        ),
    )
        .into_response()
}

fn too_many() -> Response {
    (
        StatusCode::TOO_MANY_REQUESTS,
        page("Çok fazla deneme: on beş dakika sonra yine dene.", ""),
    )
        .into_response()
}

/// A link, opened: a page with one button. Opening it uses nothing up (a
/// chat program opens links to show a preview); pressing the button does.
async fn link(State(g): State<Shared>, headers: HeaderMap, Url(token): Url<String>) -> Response {
    let from = address(&headers);
    if g.door.waiting(&from) {
        return too_many();
    }
    if !people::invited(&g.home, &token) {
        g.door.wrong(&from);
        return no_such_link();
    }
    page(
        "Dactile'a hoş geldin.",
        r#"<form method="post"><button style="padding:.6rem 1.5rem;font:inherit">Gir</button></form>
<p style="color:#666;font-size:.9rem">Bu bağlantı bir kez çalışır ve bu tarayıcıyı içeri alır.</p>"#,
    )
    .into_response()
}

/// Whether a form was sent from a page of this site: the browser says where
/// from (`Origin`), and that is the host it was sent to. A browser that
/// says nothing is let through; they all do say, on a form sent elsewhere.
fn own_page(headers: &HeaderMap) -> bool {
    let said = |name| {
        headers
            .get(name)
            .and_then(|v: &HeaderValue| v.to_str().ok())
    };
    let (Some(origin), Some(host)) = (said(header::ORIGIN), said(header::HOST)) else {
        return true;
    };
    origin
        .split_once("://")
        .is_some_and(|(_, from)| from == host)
}

/// The button of a link's page: the link is used up, the browser signed in.
async fn come_in(State(g): State<Shared>, headers: HeaderMap, Url(token): Url<String>) -> Response {
    let from = address(&headers);
    if g.door.waiting(&from) {
        return too_many();
    }
    // The button of our own page only: another site's form could sign a
    // visitor in as someone of its choosing, to write into their folder.
    if !own_page(&headers) {
        return fail(StatusCode::FORBIDDEN, "not from this site");
    }
    let Some(person) = people::redeem(&g.home, &token) else {
        g.door.wrong(&from);
        return no_such_link();
    };
    g.door.right(&from);
    let_in(&g, &person)
}

/// The cookie that remembers, for the few minutes of a trip to Google, what
/// must come back with the browser.
const TRIP: &str = "dactile-giris";

/// Signs the browser in as `person` and sends it to the start.
fn let_in(g: &Gate, person: &Person) -> Response {
    let Ok(t) = g.door.sign_in(&person.id) else {
        return fail(StatusCode::INTERNAL_SERVER_ERROR, "no randomness");
    };
    let secure = if g.secure { "; Secure" } else { "" };
    let life = door::SESSION.as_secs();
    // Lax, not Strict: the first page after this answer must carry it.
    let cookie = format!("{COOKIE}={t}; Path=/; HttpOnly; SameSite=Lax; Max-Age={life}{secure}");
    ([(header::SET_COOKIE, cookie)], Redirect::to("/")).into_response()
}

/// To Google, to choose an account; what must come back is kept in a cookie
/// of this browser's.
async fn to_google(State(g): State<Shared>) -> Response {
    let Some(google) = &g.google else {
        return no_such_link();
    };
    let (Ok(state), Ok(nonce)) = (door::random(), door::random()) else {
        return fail(StatusCode::INTERNAL_SERVER_ERROR, "no randomness");
    };
    let secure = if g.secure { "; Secure" } else { "" };
    let cookie =
        format!("{TRIP}={state}.{nonce}; Path=/giris; HttpOnly; SameSite=Lax; Max-Age=600{secure}");
    (
        [(header::SET_COOKIE, cookie)],
        Redirect::to(&google.start(&state, &nonce)),
    )
        .into_response()
}

#[derive(serde::Deserialize)]
struct Back {
    #[serde(default)]
    code: String,
    #[serde(default)]
    state: String,
}

fn not_on_the_list() -> Response {
    (
        StatusCode::FORBIDDEN,
        page(
            "Bu Google hesabı listede yok.",
            r#"<p>Site sahibinden hesabının adresini listeye eklemesini iste, sonra <a href="/giris/google">yeniden dene</a>.</p>"#,
        ),
    )
        .into_response()
}

fn google_failed() -> Response {
    (
        StatusCode::BAD_REQUEST,
        page(
            "Google ile giriş tamamlanamadı.",
            r#"<p><a href="/giris/google">Yeniden dene</a>.</p>"#,
        ),
    )
        .into_response()
}

/// Back from Google with a code: whose it is, Google says; whether they may
/// come in, the list says.
async fn from_google(
    State(g): State<Shared>,
    headers: HeaderMap,
    axum::extract::Query(back): axum::extract::Query<Back>,
) -> Response {
    let Some(google) = g.google.clone() else {
        return no_such_link();
    };
    let from = address(&headers);
    if g.door.waiting(&from) {
        return too_many();
    }
    // The trip this browser set out on, and no other.
    let trip = headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .filter_map(|c| c.trim().split_once('='))
        .find(|(k, _)| *k == TRIP)
        .and_then(|(_, v)| v.split_once('.'))
        .map(|(state, nonce)| (state.to_string(), nonce.to_string()));
    let Some((state, nonce)) = trip.filter(|(state, _)| door::same(state, &back.state)) else {
        return google_failed();
    };
    let _ = state;
    let asked = tokio::task::spawn_blocking(move || google.whose(&back.code, &nonce)).await;
    let mail = match asked {
        Ok(Ok(mail)) => mail,
        Ok(Err(why)) => {
            eprintln!("google: {why}");
            g.door.wrong(&from);
            return google_failed();
        }
        Err(_) => return google_failed(),
    };
    let Some(person) = people::find_by_mail(&g.home, &mail) else {
        g.door.wrong(&from);
        return not_on_the_list();
    };
    g.door.right(&from);
    let mut answer = let_in(&g, &person);
    // The trip is over.
    if let Ok(over) = HeaderValue::from_str(&format!("{TRIP}=; Path=/giris; Max-Age=0")) {
        answer.headers_mut().append(header::SET_COOKIE, over);
    }
    answer
}

/// A request of the person's, passed on to their server as it came, less
/// what is the gate's own: the cookie stays here, the word goes instead.
async fn pass(
    State(g): State<Shared>,
    Extension(person): Extension<Person>,
    mut req: Request,
) -> Response {
    let theirs = match g.theirs(&person.id).await {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{e}");
            return fail(
                StatusCode::SERVICE_UNAVAILABLE,
                dactile::lang::l(
                    "Dactile'ın başlatılamadı; biraz sonra yine dene.",
                    "Your Dactile couldn't be started; try again in a moment.",
                ),
            );
        }
    };
    // Under /api here, under /api there.
    let rest = req
        .uri()
        .path_and_query()
        .map_or("/", |p| p.as_str())
        .to_string();
    let Ok(uri) = format!("http://127.0.0.1:{}/api{rest}", theirs.port).parse::<Uri>() else {
        return fail(StatusCode::BAD_REQUEST, "no such address");
    };
    *req.uri_mut() = uri;
    let h = req.headers_mut();
    h.remove(header::COOKIE);
    h.remove(header::HOST);
    h.remove(GATE);
    match HeaderValue::from_str(&theirs.word) {
        Ok(word) => h.insert(GATE, word),
        Err(_) => return fail(StatusCode::INTERNAL_SERVER_ERROR, "no word"),
    };
    match g.client.request(req).await {
        Ok(answer) => answer.map(Body::new).into_response(),
        Err(e) => {
            eprintln!("{}: {e}", person.id);
            fail(
                StatusCode::BAD_GATEWAY,
                dactile::lang::l(
                    "Dactile'ın yanıt vermedi; biraz sonra yine dene.",
                    "Your Dactile didn't answer; try again in a moment.",
                ),
            )
        }
    }
}
