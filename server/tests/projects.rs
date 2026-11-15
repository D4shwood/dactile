//! A project shared through the gate: its admin puts people in it, its
//! writers write its plays (live, through their rooms), a reader only reads
//! them.

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use dactile_server::{gate, people};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use std::sync::Arc;
use tower::ServiceExt;

async fn send(app: &axum::Router, req: Request<Body>) -> (StatusCode, Value, Option<String>) {
    let r = app.clone().oneshot(req).await.unwrap();
    let status = r.status();
    let cookie = r
        .headers()
        .get(header::SET_COOKIE)
        .map(|v| v.to_str().unwrap().split(';').next().unwrap().to_string());
    let body = r.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&body).unwrap_or(Value::Null),
        cookie,
    )
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

async fn sign_in(app: &axum::Router, who: &str) -> String {
    let r = Request::post("/api/giris")
        .header(header::CONTENT_TYPE, "application/json")
        .header("sec-fetch-site", "same-origin")
        .body(Body::from(
            json!({ "who": who, "password": format!("{who} için parola") }).to_string(),
        ))
        .unwrap();
    let (s, _, cookie) = send(app, r).await;
    assert_eq!(s, StatusCode::OK, "{who} signs in");
    cookie.unwrap()
}

#[tokio::test]
async fn a_project_is_written_by_its_writers_and_read_by_its_readers() {
    let dir = std::env::temp_dir().join(format!("dactile-projects-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let home = dir.canonicalize().unwrap();
    for (id, name, owner) in [
        ("ada", "Ada Yıldız", true),
        ("baran", "Baran Deniz", false),
        ("cem", "Cem Ak", false),
        ("deniz", "Deniz Su", false),
    ] {
        people::add(&home, id, name, owner, None).unwrap();
        people::set_password(&home, id, &format!("{id} için parola")).unwrap();
    }
    let server = env!("CARGO_BIN_EXE_dactile-server").into();
    let gate = Arc::new(gate::Gate::new(home.clone(), server, false));
    let app = gate::router(gate.clone(), None);
    let ada = sign_in(&app, "ada").await;
    let baran = sign_in(&app, "baran").await;
    let cem = sign_in(&app, "cem").await;
    let deniz = sign_in(&app, "deniz").await;

    // Ada makes a project and puts Baran in to write, Cem to read.
    let (s, p, _) = send(
        &app,
        post(
            "/api/projects",
            &baran,
            json!({"name": "Kar Masalı Provası"}),
        ),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{p}");
    let (s, p, _) = send(
        &app,
        post("/api/projects", &ada, json!({"name": "Kar Masalı Provası"})),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    // A second of the same name is a project of its own.
    let id = p["id"].as_str().unwrap().to_string();
    assert_eq!(id, "kar-masali-provasi-2");
    assert_eq!(p["role"], "admin");
    let member = |who: &str, role: Value| json!({ "who": who, "role": role });
    let at = |what: &str| format!("/api/projects/{id}/{what}");
    // Only the admin puts people in.
    let (s, _, _) = send(
        &app,
        post(&at("member"), &baran, member("cem", json!("writer"))),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    let (s, _, _) = send(
        &app,
        post(&at("member"), &ada, member("baran", json!("writer"))),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let (s, p, _) = send(
        &app,
        post(&at("member"), &ada, member("cem", json!("reader"))),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(p["members"].as_array().unwrap().len(), 3);

    // Ada starts a play in it: it is the project's, not among hers.
    let (_, h, _) = send(&app, post("/api/call/home", &ada, json!({}))).await;
    let folder = h["projects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == id.as_str())
        .unwrap()["folder"]
        .as_str()
        .unwrap()
        .to_string();
    let new = json!({"template": 0, "title": "Kar Masalı", "details": {}, "folder": folder});
    let (s, play, _) = send(&app, post("/api/call/create", &ada, new.clone())).await;
    assert_eq!(s, StatusCode::OK, "{play}");
    let play = play.as_str().unwrap().to_string();
    assert!(play.starts_with(&folder), "{play}");
    let (_, h, _) = send(&app, post("/api/call/home", &ada, json!({}))).await;
    assert_eq!(h["plays"].as_array().unwrap().len(), 0);

    // Everyone in it sees it, with their role; Deniz, out of it, doesn't.
    let (_, h, _) = send(&app, post("/api/call/home", &cem, json!({}))).await;
    let theirs = &h["projects"][0];
    assert_eq!(
        (
            theirs["role"].clone(),
            theirs["plays"].as_array().unwrap().len()
        ),
        (json!("reader"), 1)
    );
    let (_, h, _) = send(&app, post("/api/call/home", &deniz, json!({}))).await;
    assert_eq!(h["projects"].as_array().unwrap().len(), 0);
    let (s, _, _) = send(&app, post("/api/call/open", &deniz, json!({"path": play}))).await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    let (_, l, _) = send(&app, get("/api/projects", &deniz)).await;
    assert_eq!(l.as_array().unwrap().len(), 0);

    // Baran writes it, live: his server takes no lock and never saves it;
    // its room does (`tests/rooms.rs`), as here.
    let (s, o, _) = send(&app, post("/api/call/open", &baran, json!({"path": play}))).await;
    assert_eq!(s, StatusCode::OK, "{o}");
    assert_eq!(
        (
            o["read_only"].clone(),
            o["reader"].clone(),
            o["live"].clone()
        ),
        (json!(false), json!(false), json!(true))
    );
    let text = o["text"].as_str().unwrap().to_string() + "\n% Baran'dan\n";
    let (s, _, _) = send(
        &app,
        post(
            "/api/call/save",
            &baran,
            json!({"text": text.clone(), "force": false}),
        ),
    )
    .await;
    assert_ne!(s, StatusCode::OK);
    std::fs::write(&play, &text).unwrap();

    // Ada opens it meanwhile: she writes it too, with him.
    let (_, o, _) = send(&app, post("/api/call/open", &ada, json!({"path": play}))).await;
    assert_eq!(
        (o["read_only"].clone(), o["live"].clone()),
        (json!(false), json!(true))
    );

    // Cem reads it, takes no lock, and can't write there.
    let (s, o, _) = send(&app, post("/api/call/open", &cem, json!({"path": play}))).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(
        (o["read_only"].clone(), o["reader"].clone()),
        (json!(true), json!(true))
    );
    assert!(o["text"].as_str().unwrap().contains("Baran'dan"));
    for (name, args) in [
        ("save", json!({"text": "x", "force": true})),
        ("build", json!({})),
        ("play_throw", json!({"path": play})),
    ] {
        let (s, _, _) = send(&app, post(&format!("/api/call/{name}"), &cem, args)).await;
        assert_eq!(s, StatusCode::FORBIDDEN, "{name}");
    }
    let (s, _, _) = send(&app, post("/api/call/create", &cem, new.clone())).await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    // The file comes to him to read.
    let file = format!("/api/file?path={}", play.replace(' ', "%20"));
    let (s, _, _) = send(&app, get(&file, &cem)).await;
    assert_eq!(s, StatusCode::OK);

    // Made a writer, Cem writes; taken out, he reaches it no more.
    let (s, _, _) = send(
        &app,
        post(&at("member"), &ada, member("cem", json!("writer"))),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    send(&app, post("/api/call/close", &ada, json!({}))).await;
    let (_, o, _) = send(&app, post("/api/call/open", &cem, json!({"path": play}))).await;
    assert_eq!(o["read_only"], false, "{o}");
    let (s, _, _) = send(&app, post(&at("member"), &ada, member("cem", Value::Null))).await;
    assert_eq!(s, StatusCode::OK);
    let (s, _, _) = send(&app, post("/api/call/open", &cem, json!({"path": play}))).await;
    assert_eq!(s, StatusCode::FORBIDDEN);

    // The admin says how it is written; she can't leave it with no admin.
    let (s, p, _) = send(&app, post(&at("mode"), &ada, json!({"mode": "together"}))).await;
    assert_eq!((s, p["mode"].clone()), (StatusCode::OK, json!("together")));
    let (s, _, _) = send(&app, post(&at("leave"), &ada, json!({}))).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    let (s, _, _) = send(&app, post(&at("leave"), &baran, json!({}))).await;
    assert_eq!(s, StatusCode::OK);

    drop(app);
    drop(gate);
    std::fs::remove_dir_all(&dir).unwrap();
}
