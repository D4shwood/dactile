//! A lasting way in: the trial site lets its holder in, again and again;
//! the real site has no such page.

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use dactile_server::{gate, people};
use std::sync::Arc;
use tower::ServiceExt;

async fn open(app: &axum::Router, path: &str) -> (StatusCode, Option<String>) {
    let r = app
        .clone()
        .oneshot(Request::get(path).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let cookie = r
        .headers()
        .get(header::SET_COOKIE)
        .map(|c| c.to_str().unwrap().split(';').next().unwrap().to_string());
    (r.status(), cookie)
}

async fn me(app: &axum::Router, cookie: &str) -> StatusCode {
    let r = Request::get("/api/me")
        .header(header::COOKIE, cookie)
        .body(Body::empty())
        .unwrap();
    app.clone().oneshot(r).await.unwrap().status()
}

#[tokio::test]
async fn a_lasting_key_works_on_the_trial_site_alone() {
    let dir = std::env::temp_dir().join(format!("dactile-keys-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let home = dir.canonicalize().unwrap();
    people::add(&home, "ada", "Ada Yıldız", true, None).unwrap();
    let key = people::key(&home, "ada").unwrap();
    let server = env!("CARGO_BIN_EXE_dactile-server").into();

    // The real site: no such page.
    let real = gate::router(Arc::new(gate::Gate::new(home.clone(), server, false)), None);
    assert_eq!(
        open(&real, &format!("/anahtar/{key}")).await,
        (StatusCode::NOT_FOUND, None)
    );

    // The trial site: in, and in again with the same link.
    let mut g = gate::Gate::new(
        home.clone(),
        env!("CARGO_BIN_EXE_dactile-server").into(),
        false,
    );
    g.stage = "pbe".into();
    let trial = gate::router(Arc::new(g), None);
    for _ in 0..2 {
        let (status, cookie) = open(&trial, &format!("/anahtar/{key}")).await;
        assert_eq!(status, StatusCode::SEE_OTHER);
        assert_eq!(me(&trial, &cookie.unwrap()).await, StatusCode::OK);
    }

    // A wrong one, or one a new key replaced, lets no one in.
    assert_eq!(open(&trial, "/anahtar/yanlis").await.1, None);
    let new = people::key(&home, "ada").unwrap();
    assert_eq!(open(&trial, &format!("/anahtar/{key}")).await.1, None);
    assert!(open(&trial, &format!("/anahtar/{new}")).await.1.is_some());
    // A password link is not a key.
    let once = people::reset(&home, "ada", 60).unwrap();
    assert_eq!(open(&trial, &format!("/anahtar/{once}")).await.1, None);

    // Off the list, the key goes with them.
    people::remove(&home, "ada").unwrap();
    assert_eq!(open(&trial, &format!("/anahtar/{new}")).await.1, None);
    let _ = std::fs::remove_dir_all(&dir);
}
