//! The server as the browser meets it: sign in, upload a play, open it, get
//! its file back, and be turned away outside the folder or signed out.
//! One test: `prepare` sets the process's settings folders.

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use std::sync::Arc;
use tower::ServiceExt;

const PLAY: &str = include_str!("../../examples/ornek-oyun.tex");

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

fn json(b: &[u8]) -> Value {
    serde_json::from_slice(b).unwrap()
}

#[tokio::test]
async fn a_signed_in_browser_works_inside_its_folder_only() {
    let dir = std::env::temp_dir().join(format!("dactile-server-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let root = dactile_server::prepare(&dir).unwrap();
    let server = Arc::new(dactile_server::Server::new(
        root.clone(),
        "kar-yağıyor-1234".into(),
        false,
    ));
    let app = dactile_server::router(server, None);

    // Signed out: nothing but the sign-in.
    let (s, _, _) = send(&app, post("/api/call/home", "", json!({}))).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    let (s, _, _) = send(&app, post("/api/login", "", json!({"password": "yanlış"}))).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    let (s, _, cookie) = send(
        &app,
        post("/api/login", "", json!({"password": "kar-yağıyor-1234"})),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let cookie = cookie.unwrap();

    // A write without the interface's header is refused.
    let bare = Request::post("/api/call/home")
        .header(header::COOKIE, &cookie)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from("{}"))
        .unwrap();
    assert_eq!(send(&app, bare).await.0, StatusCode::FORBIDDEN);

    // A play uploaded gets its own folder among the plays.
    let up = Request::post("/api/upload/play")
        .header(header::COOKIE, &cookie)
        .header("x-dactile", "1")
        .header("x-name", "Kar%20Masal%C4%B1.tex")
        .body(Body::from(PLAY))
        .unwrap();
    let (s, b, _) = send(&app, up).await;
    assert_eq!(s, StatusCode::OK, "{}", String::from_utf8_lossy(&b));
    let path = json(&b).as_str().unwrap().to_string();
    assert!(
        path.starts_with(&root.join("plays").display().to_string()),
        "{path}"
    );
    let (s, _, _) = send(
        &app,
        Request::post("/api/upload/play")
            .header(header::COOKIE, &cookie)
            .header("x-dactile", "1")
            .header("x-name", "betik.sh")
            .body(Body::from("echo"))
            .unwrap(),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);

    // The home screen lists it; it opens and saves.
    let (s, b, _) = send(&app, post("/api/call/home", &cookie, json!({}))).await;
    assert_eq!(s, StatusCode::OK);
    assert!(json(&b).to_string().contains("Kar Masal"), "{}", json(&b));
    let (s, b, _) = send(&app, post("/api/call/open", &cookie, json!({"path": path}))).await;
    assert_eq!(s, StatusCode::OK, "{}", String::from_utf8_lossy(&b));
    let (s, b, _) = send(
        &app,
        post(
            "/api/call/save",
            &cookie,
            json!({"text": PLAY, "force": false}),
        ),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{}", String::from_utf8_lossy(&b));

    // Its file comes back; one outside the folder doesn't, nor a folder.
    let get = |p: &str| {
        let q: String =
            percent_encoding::utf8_percent_encode(p, percent_encoding::NON_ALPHANUMERIC)
                .to_string();
        Request::get(format!("/api/file?path={q}&download=true"))
            .header(header::COOKIE, &cookie)
            .body(Body::empty())
            .unwrap()
    };
    let (s, b, _) = send(&app, get(&path)).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(String::from_utf8(b).unwrap(), PLAY);
    assert_eq!(
        send(&app, get("/etc/passwd")).await.0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        send(&app, get(&root.join("plays").display().to_string()))
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
    let (s, _, _) = send(
        &app,
        post("/api/call/open", &cookie, json!({"path": "/etc/hosts"})),
    )
    .await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    let (s, _, _) = send(
        &app,
        post(
            "/api/call/set_setting",
            &cookie,
            json!({"key": "plays_dir", "value": "/"}),
        ),
    )
    .await;
    assert_eq!(s, StatusCode::FORBIDDEN);

    // A command's own error comes as its message.
    let (s, b, _) = send(&app, post("/api/call/no_such", &cookie, json!({}))).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    assert!(json(&b).is_string());

    // Signed out again, the cookie is no more.
    send(&app, post("/api/logout", &cookie, json!({}))).await;
    let (s, _, _) = send(&app, post("/api/call/home", &cookie, json!({}))).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    std::fs::remove_dir_all(&dir).unwrap();

    // A new server on the same folder knows the session of the old one...
    let (s, _, kept) = send(
        &app,
        post("/api/login", "", json!({"password": "kar-yağıyor-1234"})),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let kept = kept.unwrap();
    let again = dactile_server::router(
        Arc::new(dactile_server::Server::new(
            root.clone(),
            "kar-yağıyor-1234".into(),
            false,
        )),
        None,
    );
    let (s, _, _) = send(&again, post("/api/call/home", &kept, json!({}))).await;
    assert_eq!(s, StatusCode::OK);

    // ...and an address that keeps guessing has to wait, the right password
    // or not; another address doesn't.
    let guess = |from: &str, password: &str| {
        Request::post("/api/login")
            .header(header::CONTENT_TYPE, "application/json")
            .header("x-forwarded-for", from)
            .body(Body::from(json!({ "password": password }).to_string()))
            .unwrap()
    };
    for _ in 0..5 {
        let (s, _, _) = send(&again, guess("203.0.113.7", "yanlış")).await;
        assert_eq!(s, StatusCode::UNAUTHORIZED);
    }
    let (s, _, _) = send(&again, guess("203.0.113.7", "kar-yağıyor-1234")).await;
    assert_eq!(s, StatusCode::TOO_MANY_REQUESTS);
    let (s, _, _) = send(&again, guess("203.0.113.8", "kar-yağıyor-1234")).await;
    assert_eq!(s, StatusCode::OK);
}
