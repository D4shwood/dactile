//! A desktop linked to an account: it asks for a code, someone signed in
//! says yes on a page of the site, and the desktop gets a token of its own,
//! which works as a browser's session does until it is taken off.

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use dactile_server::{gate, people};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use std::sync::Arc;
use tower::ServiceExt;

async fn send(app: &axum::Router, req: Request<Body>) -> (StatusCode, Value) {
    let r = app.clone().oneshot(req).await.unwrap();
    let status = r.status();
    let body = r.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
}

/// What the desktop sends: no cookie, its own header.
fn desk(method: &str, uri: &str, token: Option<&str>, body: Value) -> Request<Body> {
    let mut r = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-dactile", "1");
    if let Some(t) = token {
        r = r.header(header::AUTHORIZATION, format!("Bearer {t}"));
    }
    r.body(if method == "GET" {
        Body::empty()
    } else {
        Body::from(body.to_string())
    })
    .unwrap()
}

/// What a signed-in page of the site sends.
fn page(uri: &str, cookie: &str, own: bool) -> Request<Body> {
    Request::post(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::COOKIE, cookie)
        .header("x-dactile", "1")
        .header(
            "sec-fetch-site",
            if own { "same-origin" } else { "cross-site" },
        )
        .body(Body::from("{}"))
        .unwrap()
}

async fn sign_in(app: &axum::Router, who: &str, password: &str) -> String {
    let r = Request::post("/api/giris")
        .header(header::CONTENT_TYPE, "application/json")
        .header("sec-fetch-site", "same-origin")
        .body(Body::from(
            json!({ "who": who, "password": password }).to_string(),
        ))
        .unwrap();
    let r = app.clone().oneshot(r).await.unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    r.headers()
        .get(header::SET_COOKIE)
        .unwrap()
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string()
}

#[tokio::test]
async fn a_desktop_is_linked_by_a_yes_in_the_browser() {
    let dir = std::env::temp_dir().join(format!("dactile-devices-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let home = dir.canonicalize().unwrap();
    people::add(&home, "ada", "Ada Yıldız", true, None).unwrap();
    people::set_password(&home, "ada", "kar-masali-12").unwrap();
    let server = env!("CARGO_BIN_EXE_dactile-server").into();
    let app = gate::router(Arc::new(gate::Gate::new(home.clone(), server, false)), None);
    let cookie = sign_in(&app, "ada", "kar-masali-12").await;

    // The door says what this site is, before anyone is linked.
    let (s, door) = send(&app, desk("GET", "/api/door", None, Value::Null)).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(door["api"], json!(dactile_server::API));

    // The desktop asks; until a yes it waits.
    let (s, asked) = send(
        &app,
        desk(
            "POST",
            "/api/cihaz",
            None,
            json!({ "name": "Ada'nın masaüstü" }),
        ),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let poll = asked["poll"].as_str().unwrap().to_string();
    let code = asked["code"].as_str().unwrap().to_string();
    assert_eq!(code.len(), 8);
    assert_eq!(asked["page"], json!(format!("/cihaz/{code}")));
    let wait = || desk("GET", &format!("/api/cihaz/{poll}"), None, Value::Null);
    assert_eq!(send(&app, wait()).await.0, StatusCode::ACCEPTED);

    // The page shows the computer's name, to someone signed in only.
    let (s, _) = send(
        &app,
        desk("GET", &format!("/api/cihaz/bak/{code}"), None, Value::Null),
    )
    .await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    let look = Request::get(format!("/api/cihaz/bak/{}", code.to_lowercase()))
        .header(header::COOKIE, &cookie)
        .body(Body::empty())
        .unwrap();
    let (s, seen) = send(&app, look).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(seen["name"], json!("Ada'nın masaüstü"));

    // Another site's page can't say yes for them.
    let yes = format!("/api/cihaz/onay/{code}");
    assert_eq!(
        send(&app, page(&yes, &cookie, false)).await.0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(send(&app, wait()).await.0, StatusCode::ACCEPTED);
    assert_eq!(
        send(&app, page(&yes, &cookie, true)).await.0,
        StatusCode::OK
    );

    // The token comes once.
    let (s, got) = send(&app, wait()).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(got["id"], json!("ada"));
    let token = got["token"].as_str().unwrap().to_string();
    assert_eq!(send(&app, wait()).await.0, StatusCode::NOT_FOUND);
    // A used code can't be said yes to again.
    assert_eq!(
        send(&app, page(&yes, &cookie, true)).await.0,
        StatusCode::NOT_FOUND
    );

    // With it the desktop is Ada: its writes need its header, as a page's do.
    let (s, me) = send(&app, desk("GET", "/api/me", Some(&token), Value::Null)).await;
    assert_eq!((s, me["id"].clone()), (StatusCode::OK, json!("ada")));
    let (s, v) = send(&app, desk("GET", "/api/version", Some(&token), Value::Null)).await;
    assert_eq!(
        (s, v["api"].clone()),
        (StatusCode::OK, json!(dactile_server::API))
    );
    let bare = Request::post("/api/oda/bilet")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(r#"{"path":"x.tex"}"#))
        .unwrap();
    assert_eq!(send(&app, bare).await.0, StatusCode::FORBIDDEN);
    let (s, t) = send(
        &app,
        desk(
            "POST",
            "/api/oda/bilet",
            Some(&token),
            json!({ "path": "x.tex" }),
        ),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(t["ticket"].as_str().unwrap().len(), 64);
    // (Without a WebSocket's own headers nothing opens at all.)
    assert!(
        send(
            &app,
            desk("GET", "/api/oda/bilet/yanlis", None, Value::Null)
        )
        .await
        .0
        .is_client_error()
    );

    // It is in Ada's list, and taken off from there it is out.
    let list = || {
        Request::get("/api/cihazlar")
            .header(header::COOKIE, &cookie)
            .body(Body::empty())
            .unwrap()
    };
    let (_, mine) = send(&app, list()).await;
    assert_eq!(mine.as_array().unwrap().len(), 1);
    assert_eq!(mine[0]["name"], json!("Ada'nın masaüstü"));
    let id = mine[0]["id"].as_str().unwrap();
    assert!(!token.ends_with(id) && token.starts_with(id));
    let off = format!("/api/cihazlar/{id}/kaldir");
    assert_eq!(
        send(&app, page(&off, &cookie, true)).await.0,
        StatusCode::OK
    );
    assert_eq!(
        send(&app, desk("GET", "/api/me", Some(&token), Value::Null))
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert!(send(&app, list()).await.1.as_array().unwrap().is_empty());

    // A no leaves the desktop out.
    let (_, asked) = send(
        &app,
        desk("POST", "/api/cihaz", None, json!({ "name": "x" })),
    )
    .await;
    let no = format!("/api/cihaz/ret/{}", asked["code"].as_str().unwrap());
    assert_eq!(send(&app, page(&no, &cookie, true)).await.0, StatusCode::OK);
    let poll = asked["poll"].as_str().unwrap();
    let (s, _) = send(
        &app,
        desk("GET", &format!("/api/cihaz/{poll}"), None, Value::Null),
    )
    .await;
    assert_eq!(s, StatusCode::FORBIDDEN);

    // Off the list, a linked desktop goes with the browsers.
    let (_, asked) = send(&app, desk("POST", "/api/cihaz", None, json!({}))).await;
    let yes = format!("/api/cihaz/onay/{}", asked["code"].as_str().unwrap());
    send(&app, page(&yes, &cookie, true)).await;
    let poll = asked["poll"].as_str().unwrap();
    let (_, got) = send(
        &app,
        desk("GET", &format!("/api/cihaz/{poll}"), None, Value::Null),
    )
    .await;
    let token = got["token"].as_str().unwrap().to_string();
    people::remove(&home, "ada").unwrap();
    assert_eq!(
        send(&app, desk("GET", "/api/me", Some(&token), Value::Null))
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    let _ = std::fs::remove_dir_all(&dir);
}
