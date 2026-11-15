//! Signing in with Google, against a stand-in for Google's answering side:
//! the address on the list comes in, and nothing else does. Needs `curl`,
//! as the gate does; without it the test passes having checked nothing.

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use dactile_server::google::Google;
use dactile_server::{gate, people};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};
use tower::ServiceExt;

/// A token as Google makes them: only the middle part is read.
fn token(claims: &Value) -> String {
    const LETTERS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let bytes = claims.to_string().into_bytes();
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (b[0] as u32) << 16 | (b[1] as u32) << 8 | b[2] as u32;
        for i in 0..=chunk.len() {
            out.push(LETTERS[(n >> (18 - 6 * i)) as usize & 63] as char);
        }
    }
    format!("e30.{out}.imza")
}

async fn send(app: &axum::Router, req: Request<Body>) -> (StatusCode, Vec<String>, String) {
    let r = app.clone().oneshot(req).await.unwrap();
    let status = r.status();
    let cookies = r
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .map(|v| v.to_str().unwrap().split(';').next().unwrap().to_string())
        .collect();
    let to = r
        .headers()
        .get(header::LOCATION)
        .map(|v| v.to_str().unwrap().to_string())
        .unwrap_or_default();
    let _ = r.into_body().collect().await;
    (status, cookies, to)
}

fn get(uri: &str, cookie: &str) -> Request<Body> {
    Request::get(uri)
        .header(header::COOKIE, cookie)
        .body(Body::empty())
        .unwrap()
}

fn param(url: &str, name: &str) -> String {
    url.split(['?', '&'])
        .find_map(|p| p.strip_prefix(&format!("{name}=")))
        .unwrap_or_default()
        .to_string()
}

#[tokio::test(flavor = "multi_thread")]
async fn the_address_on_the_list_comes_in_and_no_other() {
    let has_curl = std::process::Command::new("curl")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success());
    if !has_curl {
        eprintln!("no curl: nothing checked");
        return;
    }
    let dir = std::env::temp_dir().join(format!("dactile-google-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let home = dir.canonicalize().unwrap();
    people::add(&home, "ada", "Ada Yıldız", true, Some("ada@example.com")).unwrap();

    // The stand-in: it answers a code with whatever claims the test last set,
    // and remembers what it was asked.
    let claims = Arc::new(Mutex::new(json!({})));
    let asked = Arc::new(Mutex::new(String::new()));
    let (c, a) = (claims.clone(), asked.clone());
    let stand_in = axum::Router::new().route(
        "/token",
        axum::routing::post(move |body: String| {
            *a.lock().unwrap() = body;
            let answer = json!({ "id_token": token(&c.lock().unwrap()) });
            async move { axum::Json(answer) }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move { axum::serve(listener, stand_in).await });

    let mut g = gate::Gate::new(home.clone(), "/bin/false".into(), false);
    g.google = Some(Google {
        id: "bizim-site".into(),
        secret: "cok-gizli".into(),
        site: "https://oyun.example".into(),
        auth: "https://accounts.google.com/o/oauth2/v2/auth".into(),
        token: format!("http://127.0.0.1:{port}/token"),
    });
    let app = gate::router(Arc::new(g), None);

    // Setting out: to Google, with a state and a nonce, both kept in a cookie.
    let set_out = || async {
        let (s, cookies, to) = send(&app, get("/giris/google", "")).await;
        assert_eq!(s, StatusCode::SEE_OTHER);
        assert!(to.starts_with("https://accounts.google.com/"), "{to}");
        (cookies[0].clone(), param(&to, "state"), param(&to, "nonce"))
    };
    let far = 4_000_000_000u64;
    let good = |nonce: &str, mail: &str| {
        json!({
            "iss": "https://accounts.google.com", "aud": "bizim-site", "nonce": nonce,
            "exp": far, "email": mail, "email_verified": true,
        })
    };
    let back = |state: &str| format!("/giris/google/donus?code=bir-kod&state={state}");

    // Ada's address: in, as Ada.
    let (trip, state, nonce) = set_out().await;
    *claims.lock().unwrap() = good(&nonce, "Ada@Example.com");
    let (s, cookies, to) = send(&app, get(&back(&state), &trip)).await;
    assert_eq!((s, to.as_str()), (StatusCode::SEE_OTHER, "/"));
    let session = cookies.iter().find(|c| c.starts_with("dactile=")).unwrap();
    let r = app.clone().oneshot(get("/api/me", session)).await.unwrap();
    let me: Value =
        serde_json::from_slice(&r.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(me["id"], "ada");
    // Google was asked with the code and the secret; the browser never saw it.
    let body = asked.lock().unwrap().clone();
    assert!(
        body.contains("code=bir-kod") && body.contains("client_secret=cok-gizli"),
        "{body}"
    );
    assert!(body.contains("grant_type=authorization_code"));

    // An address not on the list: turned away.
    let (trip, state, nonce) = set_out().await;
    *claims.lock().unwrap() = good(&nonce, "baran@example.com");
    let (s, cookies, _) = send(&app, get(&back(&state), &trip)).await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    assert!(!cookies.iter().any(|c| c.starts_with("dactile=")));

    // An answer meant for another sign-in, or another site: no.
    let (trip, state, _) = set_out().await;
    *claims.lock().unwrap() = good("baska-giris", "ada@example.com");
    let (s, _, _) = send(&app, get(&back(&state), &trip)).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);

    // Coming back without having set out from this browser: no.
    let (_, state, nonce) = set_out().await;
    *claims.lock().unwrap() = good(&nonce, "ada@example.com");
    let (s, _, _) = send(&app, get(&back(&state), "")).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    let (trip, _, _) = set_out().await;
    let (s, _, _) = send(&app, get(&back("someone-elses-state"), &trip)).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);

    // An invitation taken up with Google: the user name is chosen first,
    // the account made with Google's address when it comes back.
    let code = people::invite(&home, "ece@example.com", "Ece", "Ada").unwrap();
    let r = Request::post(format!("/api/davet/{code}/google"))
        .header(header::CONTENT_TYPE, "application/json")
        .header("sec-fetch-site", "same-origin")
        .body(Body::from(
            json!({ "id": "ece", "name": "Ece Su" }).to_string(),
        ))
        .unwrap();
    let r = app.clone().oneshot(r).await.unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    let trip = r.headers()[header::SET_COOKIE]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string();
    let to: Value =
        serde_json::from_slice(&r.into_body().collect().await.unwrap().to_bytes()).unwrap();
    let to = to["to"].as_str().unwrap();
    *claims.lock().unwrap() = good(&param(to, "nonce"), "Ece@Gmail.com");
    let (s, cookies, where_to) = send(&app, get(&back(&param(to, "state")), &trip)).await;
    assert_eq!((s, where_to.as_str()), (StatusCode::SEE_OTHER, "/"));
    assert!(cookies.iter().any(|c| c.starts_with("dactile=")));
    let ece = people::find(&home, "ece").unwrap();
    assert_eq!(
        (ece.mail.as_str(), ece.checked, ece.name.as_str()),
        ("ece@gmail.com", true, "Ece Su")
    );
    assert!(
        people::peek(&home, &code).is_none(),
        "the invitation is used up"
    );
    // Next time Google alone lets her in.
    let (trip, state, nonce) = set_out().await;
    *claims.lock().unwrap() = good(&nonce, "ece@gmail.com");
    let (s, _, _) = send(&app, get(&back(&state), &trip)).await;
    assert_eq!(s, StatusCode::SEE_OTHER);

    std::fs::remove_dir_all(&dir).unwrap();
}
