//! The gate as browsers meet it: two people sign in with their links, each
//! gets a Dactile of their own, and neither reaches the other's.

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use dactile_server::{gate, people};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use std::sync::Arc;
use tower::ServiceExt;

async fn send(app: &axum::Router, req: Request<Body>) -> (StatusCode, Vec<u8>, Option<String>) {
    let r = app.clone().oneshot(req).await.unwrap();
    let status = r.status();
    let cookie = r
        .headers()
        .get(header::SET_COOKIE)
        .map(|v| v.to_str().unwrap().split(';').next().unwrap().to_string());
    let body = r.into_body().collect().await.unwrap().to_bytes().to_vec();
    (status, body, cookie)
}

fn post(uri: &str, cookie: &str, body: Value) -> Request<Body> {
    Request::post(uri)
        .header(header::COOKIE, cookie)
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-dactile", "1")
        .body(Body::from(body.to_string()))
        .unwrap()
}

fn get(uri: &str, cookie: &str) -> Request<Body> {
    Request::get(uri)
        .header(header::COOKIE, cookie)
        .body(Body::empty())
        .unwrap()
}

fn json(b: &[u8]) -> Value {
    serde_json::from_slice(b).unwrap_or_else(|_| panic!("{}", String::from_utf8_lossy(b)))
}

/// Opens a link and presses its button: the cookie it gives.
async fn come_in(app: &axum::Router, token: &str) -> Option<String> {
    let page = format!("/giris/{token}");
    let (s, _, _) = send(app, get(&page, "")).await;
    if s != StatusCode::OK {
        return None;
    }
    let pressed = Request::post(&page).body(Body::empty()).unwrap();
    let (s, _, cookie) = send(app, pressed).await;
    (s == StatusCode::SEE_OTHER).then_some(cookie).flatten()
}

#[tokio::test]
async fn each_person_has_a_dactile_of_their_own() {
    let dir = std::env::temp_dir().join(format!("dactile-gate-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let home = dir.canonicalize().unwrap();
    people::add(&home, "ada", "Ada Yıldız", true).unwrap();
    people::add(&home, "baran", "Baran Deniz", false).unwrap();
    let server = env!("CARGO_BIN_EXE_dactile-server").into();
    let gate = Arc::new(gate::Gate::new(home.clone(), server, false));
    let app = gate::router(gate.clone(), None);

    // Signed out: nothing of anyone's, and no password to try.
    let (s, _, _) = send(&app, post("/api/call/home", "", json!({}))).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    let (s, _, _) = send(&app, post("/api/login", "", json!({"password": "x"}))).await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    let (_, b, _) = send(&app, get("/api/door", "")).await;
    assert_eq!(json(&b)["kind"], "link");
    assert!(come_in(&app, "not-a-link").await.is_none());

    // A link lets its person in, once.
    let link = people::invite(&home, "ada", 7).unwrap();
    let ada = come_in(&app, &link).await.expect("ada's cookie");
    assert!(come_in(&app, &link).await.is_none());
    let baran = come_in(&app, &people::invite(&home, "baran", 7).unwrap())
        .await
        .expect("baran's cookie");
    let (_, b, _) = send(&app, get("/api/me", &ada)).await;
    assert_eq!(json(&b)["name"], "Ada Yıldız");
    let (_, b, _) = send(&app, get("/api/me", &baran)).await;
    assert_eq!(json(&b)["id"], "baran");

    // Ada takes the example play: it is among hers, in her folder...
    let (s, b, _) = send(&app, post("/api/call/example", &ada, json!({}))).await;
    assert_eq!(s, StatusCode::OK, "{}", String::from_utf8_lossy(&b));
    let play = json(&b).as_str().unwrap().to_string();
    assert!(
        play.starts_with(&*home.join("people/ada/plays").to_string_lossy()),
        "{play}"
    );
    let (_, b, _) = send(&app, post("/api/call/home", &ada, json!({}))).await;
    assert_eq!(json(&b)["plays"].as_array().unwrap().len(), 1);
    assert_eq!(gate.awake().await, 1);

    // ...and not among Baran's, who can't open it by its path either.
    let (s, b, _) = send(&app, post("/api/call/home", &baran, json!({}))).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(json(&b)["plays"].as_array().unwrap().len(), 0);
    let (s, _, _) = send(&app, post("/api/call/open", &baran, json!({"path": play}))).await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    let file = format!("/api/file?path={}", play.replace(' ', "%20"));
    let (s, _, _) = send(&app, get(&file, &baran)).await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    let (s, b, _) = send(&app, get(&file, &ada)).await;
    assert_eq!(s, StatusCode::OK);
    assert!(String::from_utf8_lossy(&b).contains("\\documentclass"));
    assert_eq!(gate.awake().await, 2);

    // A write without the interface's header is turned away at the gate.
    let bare = Request::post("/api/call/home")
        .header(header::COOKIE, &ada)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from("{}"))
        .unwrap();
    let (s, _, _) = send(&app, bare).await;
    assert_eq!(s, StatusCode::FORBIDDEN);

    // Their servers answer to the gate alone: asked straight, with a made-up
    // word or none, they answer no one.
    let straight = reqwest_free_get(&gate, "baran").await;
    assert_eq!(straight, 401);

    // Taken off the list, Baran is out at once; Ada is not.
    people::remove(&home, "baran").unwrap();
    let (s, _, _) = send(&app, post("/api/call/home", &baran, json!({}))).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    gate.rest().await;
    assert_eq!(gate.awake().await, 1);
    let (s, _, _) = send(&app, post("/api/call/home", &ada, json!({}))).await;
    assert_eq!(s, StatusCode::OK);

    // Signed out, the cookie is no one's.
    let (s, _, _) = send(&app, post("/api/logout", &ada, json!({}))).await;
    assert_eq!(s, StatusCode::OK);
    let (s, _, _) = send(&app, get("/api/me", &ada)).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);

    drop(app);
    drop(gate);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// A person's own server asked directly, as another program on the machine
/// could: the status it answers with.
async fn reqwest_free_get(gate: &gate::Gate, id: &str) -> u16 {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let port = gate.port_of(id).await.expect("their server runs");
    let mut s = tokio::net::TcpStream::connect(("127.0.0.1", port))
        .await
        .unwrap();
    s.write_all(b"GET /api/version HTTP/1.1\r\nHost: x\r\nx-dactile-gate: guessed\r\nConnection: close\r\n\r\n")
        .await
        .unwrap();
    let mut answer = String::new();
    s.read_to_string(&mut answer).await.unwrap();
    answer.split(' ').nth(1).unwrap().parse().unwrap()
}
