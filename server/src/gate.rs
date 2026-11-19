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
//! People have accounts (`people.rs`): a user name and a password, or the
//! Google account of their address. No one makes one of their own: the
//! owner invites them by mail (`mail.rs`), and the invitation's link is the
//! way to an account. A forgotten password is set anew by a link mailed to
//! the address, or one the owner makes and passes on.

use crate::door::{self, Door};
use crate::google::Google;
use crate::mail::{self, Mail};
use crate::people::{self, Account, Errand, Person};
use crate::projects::{self, Mode, Role};
use crate::room::{self, Rooms};
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
    /// Sending mail, where it is set up.
    pub mail: Option<Mail>,
    /// The site as browsers reach it, for links in mail:
    /// `https://oyun.example`.
    pub site: String,
    /// `pbe` on the site friends try things on before the real one gets
    /// them: the page then says it is a trial. Empty on the real site.
    pub stage: String,
    door: Door,
    /// Invitations being taken up with Google, for the trip: by its state.
    trips: std::sync::Mutex<HashMap<String, (u64, Joining)>>,
    running: tokio::sync::Mutex<HashMap<String, Arc<Theirs>>>,
    client: Client<HttpConnector, Body>,
    /// The projects' plays open now, written live (`room.rs`).
    pub rooms: Arc<Rooms>,
}

type Shared = Arc<Gate>;

impl Gate {
    pub fn new(home: PathBuf, server: PathBuf, secure: bool) -> Gate {
        Gate {
            door: Door::open(home.join("gate/secret/sessions")),
            home: home.clone(),
            secure,
            server,
            idle: Duration::from_secs(20 * 60),
            google: None,
            mail: None,
            site: "http://127.0.0.1:1430".into(),
            stage: String::new(),
            trips: std::sync::Mutex::new(HashMap::new()),
            running: tokio::sync::Mutex::new(HashMap::new()),
            client: Client::builder(TokioExecutor::new()).build(HttpConnector::new()),
            rooms: Rooms::new(home.clone()),
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
        let reach = serde_json::to_string(&projects::reach(&self.home, id)).unwrap_or_default();
        let who = people::find(&self.home, id)
            .map(|p| p.name)
            .unwrap_or_default();
        let mut c = std::process::Command::new(&self.server);
        c.env("DACTILE_DATA", &folder)
            .env("DACTILE_PROJECTS", reach)
            .env("DACTILE_WHO", who)
            .env("DACTILE_ADDR", format!("127.0.0.1:{port}"))
            .env("DACTILE_GATE_WORD", &word)
            // The gate serves the pages itself, and has no password to give.
            .env_remove("DACTILE_WEB")
            .env_remove("DACTILE_PASSWORD")
            // What is the gate's alone stays with the gate.
            .env_remove("DACTILE_GOOGLE_SECRET")
            .env_remove("DACTILE_GOOGLE_ID")
            .env_remove("DACTILE_MAIL_PASS")
            .env_remove("DACTILE_MAIL_USER")
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

    /// Stops `who`'s servers, for them to start again with what changed:
    /// the projects they reach, their name.
    pub async fn restart(&self, who: &[String]) {
        self.running.lock().await.retain(|id, _| !who.contains(id));
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
        .route("/hesap", get(account).post(account_rename))
        .route("/hesap/parola", post(account_password))
        .route("/hesap/eposta", post(account_mail))
        .route("/hesap/cik", post(account_out))
        .route("/people", get(people_list))
        .route("/people/davet", post(person_invite))
        .route("/people/davet/geri", post(person_uninvite))
        .route("/people/{id}/reset", post(person_reset))
        .route("/people/{id}/remove", post(person_remove))
        .route("/projects", get(project_list).post(project_create))
        .route("/projects/people", get(project_people))
        .route("/projects/{id}/member", post(project_member))
        .route("/projects/{id}/mode", post(project_mode))
        .route("/projects/{id}/name", post(project_rename))
        .route("/projects/{id}/leave", post(project_leave))
        .route("/oda", get(room_join))
        // What isn't the gate's own is the person's server's.
        .route("/{*rest}", any(pass))
        .route_layer(middleware::from_fn_with_state(gate.clone(), guard));
    let api = Router::new()
        .route("/door", get(door_kind))
        .route("/login", post(no_such_way))
        .route("/giris", post(sign_in))
        .route("/davet/{code}", get(invitation).post(take_invitation))
        .route("/davet/{code}/google", post(invitation_by_google))
        .route("/parola", post(forgot))
        .route("/parola/{code}", get(reset_page).post(reset))
        .route("/dogrula/{code}", post(check))
        .merge(inside);
    let app = Router::new()
        .nest("/api", api)
        .route("/giris/google", get(to_google))
        .route("/giris/google/donus", get(from_google))
        // A link made before accounts: it sets a password now.
        .route("/giris/{code}", get(old_link))
        .route("/anahtar/{code}", get(key_in))
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

fn l(tr: &'static str, en: &'static str) -> &'static str {
    dactile::lang::l(tr, en)
}

/// How one signs in here, for the page that asks.
async fn door_kind(State(g): State<Shared>) -> Json<Value> {
    Json(json!({
        "kind": "accounts",
        "google": g.google.is_some(),
        "mail": g.mail.is_some(),
        "stage": g.stage,
    }))
}

async fn no_such_way() -> Response {
    fail(StatusCode::NOT_FOUND, "one signs in at /api/giris here")
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
    ([(header::SET_COOKIE, gone())], Json(json!(true))).into_response()
}

fn gone() -> String {
    format!("{COOKIE}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0")
}

// ------------------------------------------------------------ signing in

/// Whether a request came from a page of this site. Browsers of today say
/// so themselves (`Sec-Fetch-Site`); older ones give the page's origin. A
/// browser that says nothing is let through: they all do say, sending to
/// another site. (Behind the site's `Referrer-Policy: no-referrer`, Firefox
/// gives the origin "null" even for our own page; it says Sec-Fetch-Site
/// too, so it never gets that far.)
fn own_page(headers: &HeaderMap) -> bool {
    let said = |name: &str| {
        headers
            .get(name)
            .and_then(|v: &HeaderValue| v.to_str().ok())
    };
    if let Some(site) = said("sec-fetch-site") {
        return site == "same-origin";
    }
    let (Some(origin), Some(host)) = (said("origin"), said("host")) else {
        return true;
    };
    origin
        .split_once("://")
        .is_some_and(|(_, from)| from == host)
}

fn too_many() -> Response {
    fail(
        StatusCode::TOO_MANY_REQUESTS,
        l(
            "Çok fazla deneme: on beş dakika sonra yine dene.",
            "Too many tries: try again in fifteen minutes.",
        ),
    )
}

fn not_from_here() -> Response {
    fail(StatusCode::FORBIDDEN, "not from this site")
}

/// The cookie of a new session for `person`.
fn cookie_for(g: &Gate, person: &Person) -> Option<HeaderValue> {
    let t = g.door.sign_in(&person.id).ok()?;
    let secure = if g.secure { "; Secure" } else { "" };
    let life = door::SESSION.as_secs();
    // Lax, not Strict: the first page after a trip to Google must carry it.
    HeaderValue::from_str(&format!(
        "{COOKIE}={t}; Path=/; HttpOnly; SameSite=Lax; Max-Age={life}{secure}"
    ))
    .ok()
}

/// Signed in, for a page that asked by script: it goes on to the start.
fn signed_in_as(g: &Gate, person: &Person) -> Response {
    match cookie_for(g, person) {
        Some(c) => ([(header::SET_COOKIE, c)], Json(json!({ "id": person.id }))).into_response(),
        None => fail(StatusCode::INTERNAL_SERVER_ERROR, "no randomness"),
    }
}

/// Signed in, for a browser coming back from Google: to the start.
fn let_in(g: &Gate, person: &Person) -> Response {
    match cookie_for(g, person) {
        Some(c) => ([(header::SET_COOKIE, c)], Redirect::to("/")).into_response(),
        None => fail(StatusCode::INTERNAL_SERVER_ERROR, "no randomness"),
    }
}

#[derive(serde::Deserialize)]
struct SignIn {
    /// A user name or an address.
    who: String,
    password: String,
}

/// A name and a password. A wrong one counts against the address it came
/// from and against the account: five, and both wait a quarter hour.
async fn sign_in(State(g): State<Shared>, headers: HeaderMap, Json(s): Json<SignIn>) -> Response {
    if !own_page(&headers) {
        return not_from_here();
    }
    let from = address(&headers);
    // The account's own count, by name or by address alike.
    let account = match people::find_by_either(&g.home, &s.who) {
        Some(p) => format!("@{}", p.id),
        None => format!("@{}", s.who.trim().to_lowercase()),
    };
    if g.door.waiting(&from) || g.door.waiting(&account) {
        return too_many();
    }
    let home = g.home.clone();
    let checked =
        tokio::task::spawn_blocking(move || people::check_password(&home, &s.who, &s.password))
            .await
            .ok()
            .flatten();
    let Some(person) = checked else {
        g.door.wrong(&from);
        g.door.wrong(&account);
        return fail(
            StatusCode::UNAUTHORIZED,
            l(
                "Kullanıcı adı ya da parola yanlış.",
                "The user name or the password is wrong.",
            ),
        );
    };
    g.door.right(&from);
    g.door.right(&account);
    signed_in_as(&g, &person)
}

/// Sends mail where the site has a way to; `false` where it hasn't, or it
/// went wrong (said in the log).
async fn send(g: &Gate, to: &str, (subject, text): (String, String)) -> bool {
    let Some(mail) = g.mail.clone() else {
        return false;
    };
    let to = to.to_string();
    match tokio::task::spawn_blocking(move || mail.send(&to, &subject, &text)).await {
        Ok(Ok(())) => true,
        Ok(Err(e)) => {
            eprintln!("{e}");
            false
        }
        Err(_) => false,
    }
}

fn link(g: &Gate, path: &str) -> String {
    format!("{}{path}", g.site)
}

fn no_longer_good() -> Response {
    fail(
        StatusCode::NOT_FOUND,
        l(
            "Bu bağlantı artık geçerli değil: kullanılmış ya da süresi dolmuş.",
            "This link is no longer good: used, or ended.",
        ),
    )
}

/// What an invitation is: to whom, from whom. Looking uses nothing up.
async fn invitation(
    State(g): State<Shared>,
    headers: HeaderMap,
    Url(code): Url<String>,
) -> Response {
    let from = address(&headers);
    if g.door.waiting(&from) {
        return too_many();
    }
    match people::peek(&g.home, &code) {
        Some(Errand::Davet { mail, name, by }) => {
            Json(json!({ "mail": mail, "name": name, "by": by })).into_response()
        }
        _ => {
            g.door.wrong(&from);
            no_longer_good()
        }
    }
}

#[derive(serde::Deserialize)]
struct NewAccount {
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    mail: String,
    password: String,
}

/// The account an invitation was for, made: signed in, and an address
/// other than the invitation's sent a link to check it.
async fn take_invitation(
    State(g): State<Shared>,
    headers: HeaderMap,
    Url(code): Url<String>,
    Json(a): Json<NewAccount>,
) -> Response {
    if !own_page(&headers) {
        return not_from_here();
    }
    let from = address(&headers);
    if g.door.waiting(&from) {
        return too_many();
    }
    let home = g.home.clone();
    let made = tokio::task::spawn_blocking(move || {
        people::accept(
            &home,
            &code,
            Account {
                id: a.id.trim(),
                name: &a.name,
                mail: &a.mail,
                password: Some(&a.password),
                google: None,
            },
        )
    })
    .await;
    let person = match made {
        Ok(Ok(p)) => p,
        Ok(Err(why)) => return fail(StatusCode::BAD_REQUEST, why),
        Err(_) => return fail(StatusCode::INTERNAL_SERVER_ERROR, "no account"),
    };
    g.door.right(&from);
    if !person.checked {
        send_check(&g, &person, &person.mail.clone()).await;
    }
    signed_in_as(&g, &person)
}

/// A link to check `mail` is `person`'s, sent to it.
async fn send_check(g: &Gate, person: &Person, mail: &str) -> bool {
    match people::check_address(&g.home, &person.id, mail) {
        Ok(code) => {
            let words = mail::say::check(&person.name, &link(g, &format!("/dogrula/{code}")));
            send(g, mail, words).await
        }
        Err(e) => {
            eprintln!("{}: {e}", person.id);
            false
        }
    }
}

#[derive(serde::Deserialize)]
struct ByGoogle {
    id: String,
    #[serde(default)]
    name: String,
}

/// An invitation taken up with a Google account: the user name and the
/// name are chosen first, and kept for the trip; the account is made when
/// Google says whose it is.
async fn invitation_by_google(
    State(g): State<Shared>,
    headers: HeaderMap,
    Url(code): Url<String>,
    Json(b): Json<ByGoogle>,
) -> Response {
    if !own_page(&headers) {
        return not_from_here();
    }
    if !matches!(people::peek(&g.home, &code), Some(Errand::Davet { .. })) {
        return no_longer_good();
    }
    let id = b.id.trim().to_string();
    if !people::valid_id(&id) {
        return fail(
            StatusCode::BAD_REQUEST,
            l(
                "Kullanıcı adı küçük harf, rakam ve tireden oluşur; en çok 32 karakter.",
                "A user name is small letters, digits and hyphens, 32 at the most.",
            ),
        );
    }
    if people::find(&g.home, &id).is_some() || people::folder(&g.home, &id).exists() {
        return fail(
            StatusCode::BAD_REQUEST,
            l(
                "Bu kullanıcı adı alınmış; başka birini seç.",
                "This user name is taken; choose another.",
            ),
        );
    }
    match set_out(
        &g,
        Some(Joining {
            code,
            id,
            name: b.name,
        }),
    ) {
        Ok((cookie, to)) => {
            ([(header::SET_COOKIE, cookie)], Json(json!({ "to": to }))).into_response()
        }
        Err(r) => *r,
    }
}

#[derive(serde::Deserialize)]
struct Forgot {
    mail: String,
}

/// A link for a new password, sent to the address if it is someone's. The
/// answer is the same either way: it says nothing of who is here.
async fn forgot(State(g): State<Shared>, headers: HeaderMap, Json(f): Json<Forgot>) -> Response {
    if !own_page(&headers) {
        return not_from_here();
    }
    let from = address(&headers);
    if g.door.waiting(&from) {
        return too_many();
    }
    if g.mail.is_none() {
        return fail(
            StatusCode::SERVICE_UNAVAILABLE,
            l(
                "Bu site posta göndermiyor: yeni parola için site sahibine yaz.",
                "This site sends no mail: ask its owner for a new password.",
            ),
        );
    }
    // Each ask counts as a guess: no one fills a mailbox from here.
    g.door.wrong(&from);
    // Only to an address known to be theirs.
    let person = people::find_by_mail(&g.home, &f.mail).filter(|p| p.checked);
    if let Some(person) = person
        && let Ok(code) = people::reset(&g.home, &person.id, people::HOUR)
    {
        let words = mail::say::new_password(&person.name, &link(&g, &format!("/parola/{code}")));
        send(&g, &person.mail, words).await;
    }
    Json(json!(true)).into_response()
}

/// Whose new password a link is for, to greet them. Looking uses nothing up.
async fn reset_page(
    State(g): State<Shared>,
    headers: HeaderMap,
    Url(code): Url<String>,
) -> Response {
    let from = address(&headers);
    if g.door.waiting(&from) {
        return too_many();
    }
    match people::peek(&g.home, &code) {
        Some(Errand::Parola { id }) => match people::find(&g.home, &id) {
            Some(p) => Json(json!({ "id": p.id, "name": p.name })).into_response(),
            None => no_longer_good(),
        },
        _ => {
            g.door.wrong(&from);
            no_longer_good()
        }
    }
}

#[derive(serde::Deserialize)]
struct NewPassword {
    password: String,
}

/// The new password: every other session of theirs ends, this one begins.
async fn reset(
    State(g): State<Shared>,
    headers: HeaderMap,
    Url(code): Url<String>,
    Json(n): Json<NewPassword>,
) -> Response {
    if !own_page(&headers) {
        return not_from_here();
    }
    let from = address(&headers);
    if g.door.waiting(&from) {
        return too_many();
    }
    let home = g.home.clone();
    match tokio::task::spawn_blocking(move || people::new_password(&home, &code, &n.password)).await
    {
        Ok(Ok(person)) => {
            g.door.sign_out_all(&person.id);
            g.door.right(&format!("@{}", person.id));
            signed_in_as(&g, &person)
        }
        Ok(Err(why)) => fail(StatusCode::BAD_REQUEST, why),
        Err(_) => fail(StatusCode::INTERNAL_SERVER_ERROR, "no password"),
    }
}

/// An address checked: what was sent to it was opened.
async fn check(State(g): State<Shared>, headers: HeaderMap, Url(code): Url<String>) -> Response {
    if !own_page(&headers) {
        return not_from_here();
    }
    let from = address(&headers);
    if g.door.waiting(&from) {
        return too_many();
    }
    match people::checked(&g.home, &code) {
        Ok(p) => Json(json!({ "mail": p.mail })).into_response(),
        Err(why) => {
            g.door.wrong(&from);
            fail(StatusCode::NOT_FOUND, why)
        }
    }
}

/// A lasting way in (`people::key`): on the trial site alone, where it
/// spares typing a password. Elsewhere there is no such page.
async fn key_in(State(g): State<Shared>, headers: HeaderMap, Url(code): Url<String>) -> Response {
    if g.stage != "pbe" {
        return StatusCode::NOT_FOUND.into_response();
    }
    let from = address(&headers);
    if g.door.waiting(&from) {
        return too_many();
    }
    match people::by_key(&g.home, &code) {
        Some(person) => let_in(&g, &person),
        None => {
            g.door.wrong(&from);
            Redirect::to("/").into_response()
        }
    }
}

/// A sign-in link made before accounts: it sets a password now.
async fn old_link(Url(code): Url<String>) -> Redirect {
    let plain = code.chars().all(|c| c.is_ascii_alphanumeric());
    Redirect::to(&format!("/parola/{}", if plain { &code } else { "" }))
}

// ------------------------------------------------------------ the account

async fn account(State(g): State<Shared>, Extension(p): Extension<Person>) -> Json<Value> {
    Json(json!({
        "id": p.id,
        "name": p.name,
        "role": p.role,
        "mail": p.mail,
        "checked": p.checked,
        "password": people::has_password(&g.home, &p.id),
        "google": g.google.is_some(),
        "sends": g.mail.is_some(),
    }))
}

#[derive(serde::Deserialize)]
struct Rename {
    name: String,
}

async fn account_rename(
    State(g): State<Shared>,
    Extension(p): Extension<Person>,
    Json(r): Json<Rename>,
) -> Response {
    match people::rename(&g.home, &p.id, &r.name) {
        Ok(p) => Json(json!({ "name": p.name })).into_response(),
        Err(why) => fail(StatusCode::BAD_REQUEST, why),
    }
}

#[derive(serde::Deserialize)]
struct ChangePassword {
    #[serde(default)]
    old: String,
    new: String,
}

/// A new password: the old one first, where there was one. Every other
/// session of theirs ends.
async fn account_password(
    State(g): State<Shared>,
    Extension(p): Extension<Person>,
    Json(c): Json<ChangePassword>,
) -> Response {
    let account = format!("@{}", p.id);
    if g.door.waiting(&account) {
        return too_many();
    }
    let home = g.home.clone();
    let id = p.id.clone();
    let done = tokio::task::spawn_blocking(move || {
        if people::has_password(&home, &id) && people::check_password(&home, &id, &c.old).is_none()
        {
            return Err(None);
        }
        people::set_password(&home, &id, &c.new).map_err(Some)
    })
    .await;
    match done {
        Ok(Ok(())) => {
            // Elsewhere out; here, in again.
            g.door.sign_out_all(&p.id);
            signed_in_as(&g, &p)
        }
        Ok(Err(None)) => {
            g.door.wrong(&account);
            fail(
                StatusCode::BAD_REQUEST,
                l("Şimdiki parolan yanlış.", "Your password now is wrong."),
            )
        }
        Ok(Err(Some(why))) => fail(StatusCode::BAD_REQUEST, why),
        Err(_) => fail(StatusCode::INTERNAL_SERVER_ERROR, "no password"),
    }
}

#[derive(serde::Deserialize)]
struct NewMail {
    mail: String,
}

/// Another address: a link is sent to it, and it is theirs once opened.
async fn account_mail(
    State(g): State<Shared>,
    Extension(p): Extension<Person>,
    Json(m): Json<NewMail>,
) -> Response {
    let mail = match people::address(&m.mail) {
        Ok(m) => m,
        Err(why) => return fail(StatusCode::BAD_REQUEST, why),
    };
    if g.mail.is_none() {
        return fail(
            StatusCode::SERVICE_UNAVAILABLE,
            l(
                "Bu site posta göndermiyor: adresini site sahibi değiştirebilir.",
                "This site sends no mail: its owner can change your address.",
            ),
        );
    }
    if let Err(why) = people::check_address(&g.home, &p.id, &mail) {
        return fail(StatusCode::BAD_REQUEST, why);
    }
    if send_check(&g, &p, &mail).await {
        Json(json!(true)).into_response()
    } else {
        fail(
            StatusCode::BAD_GATEWAY,
            l(
                "Posta gönderilemedi; biraz sonra yine dene.",
                "The mail couldn't be sent; try again in a moment.",
            ),
        )
    }
}

/// Out everywhere, this browser too.
async fn account_out(State(g): State<Shared>, Extension(p): Extension<Person>) -> Response {
    g.door.sign_out_all(&p.id);
    ([(header::SET_COOKIE, gone())], Json(json!(true))).into_response()
}

// ------------------------------------------------- the owner's list

/// The list is the owner's to see and change: the answer anyone else gets.
fn not_the_owner(p: &Person) -> Option<Response> {
    (!p.owner()).then(|| {
        fail(
            StatusCode::FORBIDDEN,
            l(
                "Kişi listesi site sahibinindir.",
                "The list of people is the owner's.",
            ),
        )
    })
}

async fn people_list(State(g): State<Shared>, Extension(me): Extension<Person>) -> Response {
    if let Some(no) = not_the_owner(&me) {
        return no;
    }
    let all: Vec<Value> = people::list(&g.home)
        .iter()
        .map(|p| {
            json!({
                "id": p.id, "name": p.name, "role": p.role, "mail": p.mail,
                "checked": p.checked, "password": people::has_password(&g.home, &p.id),
            })
        })
        .collect();
    let invited: Vec<Value> = people::invitations(&g.home)
        .into_iter()
        .map(|(_, mail, name, ends)| json!({ "mail": mail, "name": name, "ends": ends }))
        .collect();
    Json(json!({ "people": all, "invited": invited, "sends": g.mail.is_some() })).into_response()
}

#[derive(serde::Deserialize)]
struct Invite {
    mail: String,
    #[serde(default)]
    name: String,
}

/// An invitation, mailed where the site can; its link either way, for the
/// owner to pass on by hand.
async fn person_invite(
    State(g): State<Shared>,
    Extension(me): Extension<Person>,
    Json(i): Json<Invite>,
) -> Response {
    if let Some(no) = not_the_owner(&me) {
        return no;
    }
    let code = match people::invite(&g.home, &i.mail, &i.name, &me.name) {
        Ok(c) => c,
        Err(why) => return fail(StatusCode::BAD_REQUEST, why),
    };
    let path = format!("/davet/{code}");
    let to = people::address(&i.mail).unwrap_or_default();
    let sent = send(&g, &to, mail::say::invitation(&me.name, &link(&g, &path))).await;
    Json(json!({ "path": path, "sent": sent })).into_response()
}

async fn person_uninvite(
    State(g): State<Shared>,
    Extension(me): Extension<Person>,
    Json(i): Json<Invite>,
) -> Response {
    if let Some(no) = not_the_owner(&me) {
        return no;
    }
    match people::uninvite(&g.home, &i.mail) {
        Ok(()) => Json(json!(true)).into_response(),
        Err(why) => fail(StatusCode::BAD_REQUEST, why),
    }
}

/// A link for a new password, for someone who can't get one by mail: good
/// for a week, for the owner to pass on by hand.
async fn person_reset(
    State(g): State<Shared>,
    Extension(me): Extension<Person>,
    Url(id): Url<String>,
) -> Response {
    if let Some(no) = not_the_owner(&me) {
        return no;
    }
    match people::reset(&g.home, &id, 7 * people::DAYS) {
        Ok(code) => Json(json!({ "path": format!("/parola/{code}"), "days": 7 })).into_response(),
        Err(e) => fail(StatusCode::BAD_REQUEST, e),
    }
}

async fn person_remove(
    State(g): State<Shared>,
    Extension(me): Extension<Person>,
    Url(id): Url<String>,
) -> Response {
    if let Some(no) = not_the_owner(&me) {
        return no;
    }
    if id == me.id {
        return fail(
            StatusCode::BAD_REQUEST,
            l(
                "Kendini listeden çıkaramazsın.",
                "You can't take yourself off the list.",
            ),
        );
    }
    match people::remove(&g.home, &id).and_then(|()| projects::forget(&g.home, &id)) {
        Ok(()) => {
            // Out at once, wherever they are signed in.
            g.door.sign_out_all(&id);
            g.rest().await;
            g.rooms.people_changed();
            Json(json!(true)).into_response()
        }
        Err(e) => fail(StatusCode::BAD_REQUEST, e),
    }
}

// ----------------------------------------------------------------- Google

/// The cookie that remembers, for the few minutes of a trip to Google, what
/// must come back with the browser.
const TRIP: &str = "dactile-giris";

/// An invitation being taken up with Google: kept by the gate for the trip.
struct Joining {
    code: String,
    id: String,
    name: String,
}

/// A trip to Google: its cookie, and where the browser goes.
fn set_out(g: &Gate, joining: Option<Joining>) -> Result<(String, String), Box<Response>> {
    let Some(google) = &g.google else {
        return Err(Box::new(no_longer_good()));
    };
    let (Ok(state), Ok(nonce)) = (door::random(), door::random()) else {
        return Err(Box::new(fail(
            StatusCode::INTERNAL_SERVER_ERROR,
            "no randomness",
        )));
    };
    if let Some(j) = joining {
        let mut trips = g.trips.lock().unwrap_or_else(|e| e.into_inner());
        let now = door::now();
        trips.retain(|_, (since, _)| now.saturating_sub(*since) < 600);
        if trips.len() > 1000 {
            trips.clear();
        }
        trips.insert(state.clone(), (now, j));
    }
    let secure = if g.secure { "; Secure" } else { "" };
    let cookie =
        format!("{TRIP}={state}.{nonce}; Path=/giris; HttpOnly; SameSite=Lax; Max-Age=600{secure}");
    Ok((cookie, google.start(&state, &nonce)))
}

/// To Google, to choose an account.
async fn to_google(State(g): State<Shared>) -> Response {
    match set_out(&g, None) {
        Ok((cookie, to)) => ([(header::SET_COOKIE, cookie)], Redirect::to(&to)).into_response(),
        Err(r) => *r,
    }
}

#[derive(serde::Deserialize)]
struct Back {
    #[serde(default)]
    code: String,
    #[serde(default)]
    state: String,
}

fn page(title: &str, body: &str) -> Html<String> {
    Html(format!(
        r#"<!doctype html><html lang="tr"><head><meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0"><meta name="robots" content="noindex">
<title>Dactile</title></head>
<body style="max-width:22rem;margin:20vh auto;padding:0 1rem;font-family:system-ui,sans-serif;background:#f6f1e7;color:#2b2522">
<h1 style="font-family:monospace;letter-spacing:.4em;margin:0 0 1.5rem">DACTILE</h1>
<p style="font-size:1.1rem;margin:0 0 1rem">{title}</p>{body}</body></html>"#
    ))
}

fn not_on_the_list() -> Response {
    (
        StatusCode::FORBIDDEN,
        page(
            "Bu Google hesabıyla açılmış bir Dactile hesabı yok.",
            r#"<p>Dactile yalnızca davetle açılır. Davet aldıysan postadaki bağlantıdan hesabını aç; hesabın varsa <a href="/">kullanıcı adınla gir</a>.</p>"#,
        ),
    )
        .into_response()
}

fn google_failed(why: &str) -> Response {
    let why = why.replace(['<', '>', '&'], " ");
    (
        StatusCode::BAD_REQUEST,
        page(
            "Google ile giriş tamamlanamadı.",
            &format!(r#"<p>{why}</p><p><a href="/">Girişe dön</a>.</p>"#),
        ),
    )
        .into_response()
}

/// Back from Google with a code: whose it is, Google says. Coming to take
/// up an invitation, the account is made with Google's address; otherwise
/// whoever has that address comes in, and no one else.
async fn from_google(
    State(g): State<Shared>,
    headers: HeaderMap,
    axum::extract::Query(back): axum::extract::Query<Back>,
) -> Response {
    let Some(google) = g.google.clone() else {
        return no_longer_good();
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
        return google_failed("");
    };
    let joining = g
        .trips
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .remove(&state)
        .map(|(_, j)| j);
    let asked = tokio::task::spawn_blocking(move || google.whose(&back.code, &nonce)).await;
    let mail = match asked {
        Ok(Ok(mail)) => mail,
        Ok(Err(why)) => {
            eprintln!("google: {why}");
            g.door.wrong(&from);
            return google_failed("");
        }
        Err(_) => return google_failed(""),
    };
    let person = match joining {
        Some(j) => {
            let home = g.home.clone();
            let made = people::accept(
                &home,
                &j.code,
                Account {
                    id: &j.id,
                    name: &j.name,
                    mail: "",
                    password: None,
                    google: Some(&mail),
                },
            );
            match made {
                Ok(p) => p,
                Err(why) => return google_failed(&why),
            }
        }
        // An address not checked yet could be anyone's typing.
        None => match people::find_by_mail(&g.home, &mail).filter(|p| p.checked) {
            Some(p) => p,
            None => {
                g.door.wrong(&from);
                return not_on_the_list();
            }
        },
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
    // Their server reads the plays from the disk: what the rooms have goes
    // there first.
    let call = rest.starts_with("/call/");
    if call {
        g.rooms.save_for(&person.id);
    }
    match g.client.request(req).await {
        Ok(answer) => {
            if call {
                // A play deleted or moved: its room closes.
                g.rooms.check_gone();
            }
            answer.map(Body::new).into_response()
        }
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

// ---------------------------------------------------------------- rooms

#[derive(serde::Deserialize)]
struct Play {
    path: String,
}

/// A project's play, written live: the page talks with its room.
async fn room_join(
    State(g): State<Shared>,
    Extension(me): Extension<Person>,
    headers: HeaderMap,
    axum::extract::Query(play): axum::extract::Query<Play>,
    ws: axum::extract::WebSocketUpgrade,
) -> Response {
    // A page of another site can open a WebSocket here with our cookie.
    if !own_page(&headers) {
        return fail(StatusCode::FORBIDDEN, "not from this site");
    }
    let who = room::Who {
        id: me.id,
        name: me.name,
    };
    let rooms = g.rooms.clone();
    ws.on_upgrade(move |socket| async move { rooms.join(&play.path, who, socket).await })
}

// ------------------------------------------------------------- projects

/// A project as its members see it: who is in it, and their role.
fn project_json(g: &Gate, p: &projects::Project, me: &str) -> Value {
    let everyone = people::list(&g.home);
    let members: Vec<Value> = p
        .members
        .iter()
        .map(|(id, role)| {
            let name = everyone
                .iter()
                .find(|x| &x.id == id)
                .map_or(id.as_str(), |x| x.name.as_str());
            json!({ "id": id, "name": name, "role": role })
        })
        .collect();
    json!({
        "id": p.id, "name": p.name, "mode": p.mode,
        "role": p.members.get(me), "members": members,
    })
}

fn project_answer(g: &Gate, done: Result<projects::Project, String>, me: &str) -> Response {
    match done {
        Ok(p) => Json(project_json(g, &p, me)).into_response(),
        Err(why) => fail(StatusCode::BAD_REQUEST, why),
    }
}

async fn project_list(State(g): State<Shared>, Extension(me): Extension<Person>) -> Response {
    let mine: Vec<Value> = projects::of(&g.home, &me.id)
        .iter()
        .map(|(p, _)| project_json(&g, p, &me.id))
        .collect();
    Json(json!(mine)).into_response()
}

/// Who can be put in a project: everyone with an account here.
async fn project_people(State(g): State<Shared>) -> Response {
    let all: Vec<Value> = people::list(&g.home)
        .iter()
        .map(|p| json!({ "id": p.id, "name": p.name }))
        .collect();
    Json(json!(all)).into_response()
}

#[derive(serde::Deserialize)]
struct Named {
    name: String,
}

async fn project_create(
    State(g): State<Shared>,
    Extension(me): Extension<Person>,
    Json(n): Json<Named>,
) -> Response {
    let made = projects::create(&g.home, &n.name, &me.id);
    if made.is_ok() {
        g.restart(std::slice::from_ref(&me.id)).await;
    }
    project_answer(&g, made, &me.id)
}

#[derive(serde::Deserialize)]
struct Member {
    who: String,
    role: Option<Role>,
}

async fn project_member(
    State(g): State<Shared>,
    Extension(me): Extension<Person>,
    Url(id): Url<String>,
    Json(m): Json<Member>,
) -> Response {
    let done = projects::set_member(&g.home, &id, &me.id, &m.who, m.role);
    if done.is_ok() {
        // What they reach changed: their server starts again with it.
        g.restart(std::slice::from_ref(&m.who)).await;
        g.rooms.project_changed(&id, Some(&me.id));
    }
    project_answer(&g, done, &me.id)
}

#[derive(serde::Deserialize)]
struct Writing {
    mode: Mode,
}

async fn project_mode(
    State(g): State<Shared>,
    Extension(me): Extension<Person>,
    Url(id): Url<String>,
    Json(w): Json<Writing>,
) -> Response {
    let done = projects::set_mode(&g.home, &id, &me.id, w.mode);
    if done.is_ok() {
        g.rooms.project_changed(&id, Some(&me.id));
    }
    project_answer(&g, done, &me.id)
}

async fn project_rename(
    State(g): State<Shared>,
    Extension(me): Extension<Person>,
    Url(id): Url<String>,
    Json(n): Json<Named>,
) -> Response {
    let done = projects::rename(&g.home, &id, &me.id, &n.name);
    if let Ok(p) = &done {
        let members: Vec<String> = p.members.keys().cloned().collect();
        g.restart(&members).await;
    }
    project_answer(&g, done, &me.id)
}

async fn project_leave(
    State(g): State<Shared>,
    Extension(me): Extension<Person>,
    Url(id): Url<String>,
) -> Response {
    match projects::leave(&g.home, &id, &me.id) {
        Ok(_) => {
            g.restart(std::slice::from_ref(&me.id)).await;
            g.rooms.project_changed(&id, None);
            Json(json!(true)).into_response()
        }
        Err(why) => fail(StatusCode::BAD_REQUEST, why),
    }
}
