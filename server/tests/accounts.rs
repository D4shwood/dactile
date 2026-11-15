//! The door's own ways, as a browser meets them: a wrong password, a
//! forgotten one, a new address, and the account's own settings. Mail is
//! written to a folder, and read back from it as a person would open it.

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use dactile_server::mail::Mail;
use dactile_server::{gate, people};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
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

/// What a page of the site sends.
fn page(uri: &str, body: Value) -> Request<Body> {
    Request::post(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .header("sec-fetch-site", "same-origin")
        .header("x-forwarded-for", "198.51.100.4")
        .body(Body::from(body.to_string()))
        .unwrap()
}

/// What the app sends, signed in.
fn app_post(uri: &str, cookie: &str, body: Value) -> Request<Body> {
    Request::post(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::COOKIE, cookie)
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

async fn sign_in(app: &axum::Router, who: &str, password: &str) -> Option<String> {
    let (s, _, cookie) = send(
        app,
        page("/api/giris", json!({ "who": who, "password": password })),
    )
    .await;
    (s == StatusCode::OK).then_some(cookie).flatten()
}

/// The link in the newest mail to `to`, as a path: `/parola/…`.
fn link_in_mail(outbox: &Path, to: &str) -> Option<String> {
    let mut mails: Vec<PathBuf> = std::fs::read_dir(outbox)
        .ok()?
        .map(|f| f.unwrap().path())
        .collect();
    mails.sort();
    let text = mails
        .iter()
        .rev()
        .map(|m| std::fs::read_to_string(m).unwrap())
        .find(|t| t.contains(&format!("To: {to}\r\n")))?;
    let at = text.find("http://site.example")? + "http://site.example".len();
    Some(text[at..].split_whitespace().next()?.to_string())
}

fn mails(outbox: &Path) -> usize {
    std::fs::read_dir(outbox).map_or(0, |d| d.count())
}

#[tokio::test]
async fn passwords_forgotten_addresses_changed() {
    let dir = std::env::temp_dir().join(format!("dactile-accounts-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let home = dir.canonicalize().unwrap();
    let outbox = home.join("outbox");
    people::add(&home, "ada", "Ada Yıldız", true, Some("ada@example.com")).unwrap();
    people::set_password(&home, "ada", "ada'nın parolası").unwrap();
    let mut g = gate::Gate::new(home.clone(), "/bin/false".into(), false);
    g.site = "http://site.example".into();
    g.mail = Some(Mail {
        url: format!("file://{}", outbox.display()),
        user: String::new(),
        pass: String::new(),
        from: "Dactile <d@example.com>".into(),
    });
    let app = gate::router(Arc::new(g), None);

    // By name or by address; a wrong password says no more than that.
    assert!(sign_in(&app, "ada", "ada'nın parolası").await.is_some());
    assert!(
        sign_in(&app, "ADA@example.com", "ada'nın parolası")
            .await
            .is_some()
    );
    let (s, said, _) = send(
        &app,
        page("/api/giris", json!({ "who": "kimse", "password": "x" })),
    )
    .await;
    let (s2, said2, _) = send(
        &app,
        page("/api/giris", json!({ "who": "ada", "password": "x" })),
    )
    .await;
    assert_eq!(
        (s, s2),
        (StatusCode::UNAUTHORIZED, StatusCode::UNAUTHORIZED)
    );
    assert_eq!(said, said2);

    // Five wrong for one account, from anywhere: it waits, others don't.
    for i in 0..4 {
        let r = Request::post("/api/giris")
            .header(header::CONTENT_TYPE, "application/json")
            .header("sec-fetch-site", "same-origin")
            .header("x-forwarded-for", format!("203.0.113.{i}"))
            .body(Body::from(
                json!({ "who": "ada", "password": "yanlış" }).to_string(),
            ))
            .unwrap();
        send(&app, r).await;
    }
    let (s, _, _) = send(
        &app,
        page(
            "/api/giris",
            json!({ "who": "ada", "password": "ada'nın parolası" }),
        ),
    )
    .await;
    assert_eq!(s, StatusCode::TOO_MANY_REQUESTS);

    // Forgotten: the same answer for an address someone has and one no one
    // has; mail only to the one someone has.
    people::add(
        &home,
        "baran",
        "Baran Deniz",
        false,
        Some("baran@example.com"),
    )
    .unwrap();
    people::set_password(&home, "baran", "baran'ın parolası").unwrap();
    let elsewhere = Request::post("/api/parola")
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-forwarded-for", "192.0.2.1")
        .body(Body::from(
            json!({ "mail": "kimse@example.com" }).to_string(),
        ))
        .unwrap();
    let (s, _, _) = send(&app, elsewhere).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(mails(&outbox), 0);
    let r = Request::post("/api/parola")
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-forwarded-for", "192.0.2.1")
        .body(Body::from(
            json!({ "mail": "Baran@example.com" }).to_string(),
        ))
        .unwrap();
    let (s, _, _) = send(&app, r).await;
    assert_eq!(s, StatusCode::OK);
    let path = link_in_mail(&outbox, "baran@example.com").expect("a mail to Baran");
    assert!(path.starts_with("/parola/"), "{path}");
    let old_session = sign_in(&app, "baran", "baran'ın parolası").await.unwrap();
    // The link greets them; a short password is refused; the right one is
    // theirs, and every other session of theirs ends.
    let (s, who, _) = send(&app, get(&format!("/api{path}"), "")).await;
    assert_eq!(
        (s, who["name"].as_str()),
        (StatusCode::OK, Some("Baran Deniz"))
    );
    let (s, _, _) = send(
        &app,
        page(&format!("/api{path}"), json!({ "password": "kisa" })),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    let (s, _, now) = send(
        &app,
        page(
            &format!("/api{path}"),
            json!({ "password": "baran'ın yeni parolası" }),
        ),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let now = now.expect("signed in");
    let (s, _, _) = send(&app, get("/api/me", &old_session)).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    let (s, _, _) = send(&app, get("/api/me", &now)).await;
    assert_eq!(s, StatusCode::OK);
    let (s, _, _) = send(
        &app,
        page(
            &format!("/api{path}"),
            json!({ "password": "bir kez daha deneme" }),
        ),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST, "the link works once");
    assert!(
        sign_in(&app, "baran", "baran'ın yeni parolası")
            .await
            .is_some()
    );

    // The account: a new name; a new password needs the one now.
    let (s, _, _) = send(
        &app,
        app_post("/api/hesap", &now, json!({ "name": "Baran D." })),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(people::find(&home, "baran").unwrap().name, "Baran D.");
    let wrong = json!({ "old": "tahmin", "new": "bambaşka bir parola" });
    let (s, _, _) = send(&app, app_post("/api/hesap/parola", &now, wrong)).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    let right = json!({ "old": "baran'ın yeni parolası", "new": "bambaşka bir parola" });
    let (s, _, again) = send(&app, app_post("/api/hesap/parola", &now, right)).await;
    assert_eq!(s, StatusCode::OK);
    let again = again.expect("still in, here");
    let (s, _, _) = send(&app, get("/api/me", &now)).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED, "elsewhere out");
    let (s, _, _) = send(&app, get("/api/me", &again)).await;
    assert_eq!(s, StatusCode::OK);

    // A new address is theirs once they open what was sent to it.
    let (s, _, _) = send(
        &app,
        app_post(
            "/api/hesap/eposta",
            &again,
            json!({ "mail": "ada@example.com" }),
        ),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST, "someone else's");
    let (s, _, _) = send(
        &app,
        app_post(
            "/api/hesap/eposta",
            &again,
            json!({ "mail": "baran@yeni.example" }),
        ),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(
        people::find(&home, "baran").unwrap().mail,
        "baran@example.com"
    );
    let path = link_in_mail(&outbox, "baran@yeni.example").expect("a mail to the new address");
    assert!(path.starts_with("/dogrula/"), "{path}");
    let (s, _, _) = send(&app, page(&format!("/api{path}"), json!({}))).await;
    assert_eq!(s, StatusCode::OK);
    let baran = people::find(&home, "baran").unwrap();
    assert_eq!(
        (baran.mail.as_str(), baran.checked),
        ("baran@yeni.example", true)
    );

    // Out everywhere.
    let (s, _, _) = send(&app, app_post("/api/hesap/cik", &again, json!({}))).await;
    assert_eq!(s, StatusCode::OK);
    let (s, _, _) = send(&app, get("/api/me", &again)).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);

    // A link made by the owner, for someone mail doesn't reach; and an
    // old sign-in link goes where links set passwords now.
    let ada = {
        people::set_password(&home, "ada", "ada'nın parolası").unwrap();
        let r = Request::post("/api/giris")
            .header(header::CONTENT_TYPE, "application/json")
            .header("sec-fetch-site", "same-origin")
            .header("x-forwarded-for", "198.51.100.99")
            .body(Body::from(
                json!({ "who": "ada@example.com", "password": "ada'nın parolası" }).to_string(),
            ))
            .unwrap();
        // The account still waits from the wrong guesses above.
        let (s, _, _) = send(&app, r).await;
        assert_eq!(s, StatusCode::TOO_MANY_REQUESTS);
        // Signed in by a link of the owner's own making instead.
        let code = people::reset(&home, "ada", people::HOUR).unwrap();
        let (s, _, c) = send(
            &app,
            page(
                &format!("/api/parola/{code}"),
                json!({ "password": "ada'nın yeni parolası" }),
            ),
        )
        .await;
        assert_eq!(s, StatusCode::OK);
        c.unwrap()
    };
    let (s, made, _) = send(&app, app_post("/api/people/baran/reset", &ada, json!({}))).await;
    assert_eq!(s, StatusCode::OK);
    assert!(made["path"].as_str().unwrap().starts_with("/parola/"));
    let (s, _, _) = send(&app, app_post("/api/people/baran/reset", &now, json!({}))).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    let r = app.clone().oneshot(get("/giris/abc123", "")).await.unwrap();
    assert_eq!(r.headers()[header::LOCATION], "/parola/abc123");

    std::fs::remove_dir_all(&dir).unwrap();
}

#[tokio::test]
async fn without_mail_the_owner_passes_links_on() {
    let dir = std::env::temp_dir().join(format!("dactile-nomail-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let home = dir.canonicalize().unwrap();
    people::add(&home, "ada", "Ada", true, Some("ada@example.com")).unwrap();
    people::set_password(&home, "ada", "ada'nın parolası").unwrap();
    let app = gate::router(
        Arc::new(gate::Gate::new(home.clone(), "/bin/false".into(), false)),
        None,
    );
    let (_, door, _) = send(&app, get("/api/door", "")).await;
    assert_eq!(door["mail"], false);
    let ada = sign_in(&app, "ada", "ada'nın parolası").await.unwrap();
    // The invitation is made, not sent: its link is the owner's to pass on.
    let (s, made, _) = send(
        &app,
        app_post(
            "/api/people/davet",
            &ada,
            json!({ "mail": "baran@example.com" }),
        ),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(made["sent"], false);
    assert!(made["path"].as_str().unwrap().starts_with("/davet/"));
    // Taken back, it is gone.
    let (s, _, _) = send(
        &app,
        app_post(
            "/api/people/davet/geri",
            &ada,
            json!({ "mail": "baran@example.com" }),
        ),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let (s, _, _) = send(
        &app,
        get(&format!("/api{}", made["path"].as_str().unwrap()), ""),
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    // A forgotten password: the site says it can't mail, and to whom to go.
    let (s, _, _) = send(
        &app,
        page("/api/parola", json!({ "mail": "ada@example.com" })),
    )
    .await;
    assert_eq!(s, StatusCode::SERVICE_UNAVAILABLE);
    std::fs::remove_dir_all(&dir).unwrap();
}
