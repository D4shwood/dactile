//! A project's play written live: pages talk with its room over a real
//! WebSocket, as Yjs does, and the room lets through only those who write.

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use dactile_server::room::{ASK, BYE, INFO, REFUSED, TEXT};
use dactile_server::{gate, people, projects};
use futures_util::{SinkExt, StreamExt};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Duration;
use tokio_tungstenite::tungstenite::Message as Ws;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tower::ServiceExt;
use yrs::sync::awareness::AwarenessUpdateEntry;
use yrs::sync::{AwarenessUpdate, Message, SyncMessage};
use yrs::updates::decoder::Decode;
use yrs::updates::encoder::Encode;
use yrs::{ClientID, Doc, GetString, ReadTxn, Text, TextRef, Transact, Update};

type Socket =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

async fn sign_in(app: &axum::Router, who: &str) -> String {
    let r = Request::post("/api/giris")
        .header(header::CONTENT_TYPE, "application/json")
        .header("sec-fetch-site", "same-origin")
        .body(Body::from(
            json!({ "who": who, "password": format!("{who} için parola") }).to_string(),
        ))
        .unwrap();
    let r = app.clone().oneshot(r).await.unwrap();
    assert_eq!(r.status(), StatusCode::OK, "{who} signs in");
    r.headers()[header::SET_COOKIE]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string()
}

async fn post(app: &axum::Router, uri: &str, cookie: &str, body: Value) -> StatusCode {
    let r = Request::post(uri)
        .header(header::COOKIE, cookie)
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-dactile", "1")
        .body(Body::from(body.to_string()))
        .unwrap();
    let r = app.clone().oneshot(r).await.unwrap();
    let s = r.status();
    let _ = r.into_body().collect().await;
    s
}

/// A page with the play open: its own copy of the document.
struct Page {
    doc: Doc,
    text: TextRef,
    ws: Socket,
    info: Value,
    /// Names on the others' cursors, as they came.
    names: Vec<String>,
    bye: Option<String>,
    refused: bool,
}

impl Page {
    async fn open(addr: &str, cookie: &str, play: &str, doc: Option<Doc>) -> Page {
        let url = format!("ws://{addr}/api/oda?path={}", percent(play));
        let mut req = url.into_client_request().unwrap();
        req.headers_mut().insert("cookie", cookie.parse().unwrap());
        req.headers_mut()
            .insert("sec-fetch-site", "same-origin".parse().unwrap());
        let (ws, _) = tokio_tungstenite::connect_async(req).await.unwrap();
        let doc = doc.unwrap_or_default();
        let text = doc.get_or_insert_text(TEXT);
        let mut page = Page {
            doc,
            text,
            ws,
            info: Value::Null,
            names: vec![],
            bye: None,
            refused: false,
        };
        let sv = page.doc.transact().state_vector();
        page.send(Message::Sync(SyncMessage::SyncStep1(sv))).await;
        page.listen(Duration::from_millis(300)).await;
        page
    }

    async fn send(&mut self, m: Message) {
        let _ = self.ws.send(Ws::Binary(m.encode_v1().into())).await;
    }

    async fn ask(&mut self, what: &str) {
        let body = json!({ "do": what }).to_string().into_bytes();
        self.send(Message::Custom(ASK, body)).await;
    }

    /// Takes in what comes for a while.
    async fn listen(&mut self, wait: Duration) {
        let until = tokio::time::Instant::now() + wait;
        while let Ok(Some(Ok(m))) = tokio::time::timeout_at(until, self.ws.next()).await {
            let Ws::Binary(b) = m else {
                if matches!(m, Ws::Close(_)) {
                    break;
                }
                continue;
            };
            match Message::decode_v1(&b).unwrap() {
                Message::Sync(SyncMessage::SyncStep1(sv)) => {
                    let u = self.doc.transact().encode_state_as_update_v1(&sv);
                    self.send(Message::Sync(SyncMessage::SyncStep2(u))).await;
                }
                Message::Sync(SyncMessage::SyncStep2(u) | SyncMessage::Update(u)) => {
                    let u = Update::decode_v1(&u).unwrap();
                    self.doc.transact_mut().apply_update(u).unwrap();
                }
                Message::Awareness(a) => {
                    for e in a.clients.values() {
                        if let Ok(v) = serde_json::from_str::<Value>(&e.json)
                            && let Some(n) = v["user"]["name"].as_str()
                        {
                            self.names.push(n.to_string());
                        }
                    }
                }
                Message::Custom(REFUSED, _) => self.refused = true,
                Message::Custom(INFO, d) => self.info = serde_json::from_slice(&d).unwrap(),
                Message::Custom(BYE, d) => {
                    let v: Value = serde_json::from_slice(&d).unwrap();
                    self.bye = v["why"].as_str().map(String::from);
                }
                _ => {}
            }
        }
    }

    /// Types `what` at `at`, and sends it as Yjs would.
    async fn write(&mut self, at: u32, what: &str) {
        let before = self.doc.transact().state_vector();
        self.text.insert(&mut self.doc.transact_mut(), at, what);
        let u = self.doc.transact().encode_state_as_update_v1(&before);
        self.send(Message::Sync(SyncMessage::Update(u))).await;
    }

    fn now(&self) -> String {
        self.text.get_string(&self.doc.transact())
    }
}

fn percent(s: &str) -> String {
    s.bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"/-_.".contains(&b) {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}

async fn serve(g: Arc<gate::Gate>) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap().to_string();
    let app = gate::router(g, None);
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    addr
}

const SHORT: Duration = Duration::from_millis(400);

#[tokio::test(flavor = "multi_thread")]
async fn a_play_is_written_live_by_those_who_may() {
    let dir = std::env::temp_dir().join(format!("dactile-rooms-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let home = dir.canonicalize().unwrap();
    for (id, name, owner) in [
        ("ada", "Ada Yıldız", true),
        ("baran", "Baran Deniz", false),
        ("cem", "Cem Ak", false),
    ] {
        people::add(&home, id, name, owner, None).unwrap();
        people::set_password(&home, id, &format!("{id} için parola")).unwrap();
    }
    let p = projects::create(&home, "Kar Masalı", "ada").unwrap();
    projects::set_member(&home, &p.id, "ada", "baran", Some(projects::Role::Writer)).unwrap();
    projects::set_member(&home, &p.id, "ada", "cem", Some(projects::Role::Reader)).unwrap();
    projects::set_mode(&home, &p.id, "ada", projects::Mode::Together).unwrap();
    let play = projects::folder(&home, &p.id).join("kar.tex");
    std::fs::write(&play, "Kar yağıyor.\n").unwrap();
    let path = play.display().to_string();

    let server = env!("CARGO_BIN_EXE_dactile-server").into();
    let g = Arc::new(gate::Gate::new(home.clone(), server, false));
    let app = gate::router(g.clone(), None);
    let addr = serve(g.clone()).await;
    let ada = sign_in(&app, "ada").await;
    let baran = sign_in(&app, "baran").await;
    let cem = sign_in(&app, "cem").await;

    // Someone not in it, or a play outside it, gets no room.
    let a = Page::open(
        &addr,
        &ada,
        &home.join("people/x.tex").display().to_string(),
        None,
    )
    .await;
    assert!(a.bye.is_some());

    // Together: Ada and Baran write at once, Cem watches.
    let mut a = Page::open(&addr, &ada, &path, None).await;
    let mut b = Page::open(&addr, &baran, &path, None).await;
    let mut c = Page::open(&addr, &cem, &path, None).await;
    assert_eq!(a.now(), "Kar yağıyor.\n");
    assert_eq!(
        (a.info["writes"].clone(), c.info["writes"].clone()),
        (json!(true), json!(false))
    );
    a.write(0, "Işık söner. ").await;
    b.write(b.now().chars().count() as u32, "Çığlık: ğüşöç!\n")
        .await;
    for p in [&mut a, &mut b, &mut c] {
        p.listen(SHORT).await;
    }
    let both = a.now();
    assert!(
        both.contains("Işık söner.") && both.contains("Çığlık: ğüşöç!"),
        "{both}"
    );
    assert_eq!((b.now(), c.now()), (both.clone(), both.clone()));

    // What Cem sends goes nowhere, and his page hears it must start again;
    // having only read, it never did.
    assert!(!c.refused);
    c.write(0, "Okur yazdı. ").await;
    a.listen(SHORT).await;
    c.listen(SHORT).await;
    assert!(!a.now().contains("Okur"));
    assert!(c.refused);

    // The name on a cursor is the gate's, whatever the page says.
    let fake = AwarenessUpdate {
        clients: [(
            ClientID::new(c.doc.client_id().get()),
            AwarenessUpdateEntry {
                clock: 1,
                json: r#"{"user":{"name":"Ada Yıldız"}}"#.into(),
            },
        )]
        .into(),
    };
    c.send(Message::Awareness(fake)).await;
    a.listen(SHORT).await;
    assert_eq!(a.names.last().map(String::as_str), Some("Cem Ak"));

    // A moment after the writing stops, the play is on the disk.
    tokio::time::sleep(Duration::from_millis(2600)).await;
    assert_eq!(std::fs::read_to_string(&play).unwrap(), both);
    assert!(play.with_file_name(".kar.tex.dactile-y").exists());

    // In turns: the pen is Ada's, who said so; Baran's typing goes nowhere.
    let at = |what: &str| format!("/api/projects/{}/{what}", p.id);
    assert_eq!(
        post(&app, &at("mode"), &ada, json!({"mode": "turns"})).await,
        StatusCode::OK
    );
    for p in [&mut a, &mut b] {
        p.listen(SHORT).await;
    }
    assert_eq!(
        (a.info["mine"].clone(), b.info["pen"].clone()),
        (json!(true), json!("Ada Yıldız"))
    );
    assert_eq!(b.info["writes"], false);
    b.write(0, "Sırasız. ").await;
    a.listen(SHORT).await;
    assert!(!a.now().contains("Sırasız"));
    // Baran can't take it while Ada has it; she gives it up, and it is his.
    b.ask("pen").await;
    b.listen(SHORT).await;
    assert_eq!(b.info["mine"], false);
    a.ask("drop").await;
    b.listen(SHORT).await;
    assert_eq!(b.info["mine"], true);
    // His page dropped what it typed without the pen: it starts again.
    let mut b = Page::open(&addr, &baran, &path, None).await;
    b.write(0, "Sıra bende. ").await;
    a.listen(SHORT).await;
    assert!(a.now().starts_with("Sıra bende. "), "{}", a.now());

    // Saved when asked.
    a.ask("save").await;
    a.listen(SHORT).await;
    assert!(
        std::fs::read_to_string(&play)
            .unwrap()
            .starts_with("Sıra bende. ")
    );

    // Taken out of the project, Baran is out of the room.
    assert_eq!(
        post(
            &app,
            &at("member"),
            &ada,
            json!({"who": "baran", "role": null})
        )
        .await,
        StatusCode::OK
    );
    b.listen(SHORT).await;
    assert!(b.bye.is_some());

    // The gate starts again: Ada's page, cut off, wrote meanwhile; back, it
    // merges, nothing twice.
    let epoch = a.info["epoch"].clone();
    drop(c);
    g.rooms.save_all();
    let mut kept = Page { ws: a.ws, ..a };
    let _ = kept.ws.close(None).await;
    kept.write(0, "Kopukken. ").await;
    let g2 = Arc::new(gate::Gate::new(
        home.clone(),
        env!("CARGO_BIN_EXE_dactile-server").into(),
        false,
    ));
    let app2 = gate::router(g2.clone(), None);
    let addr2 = serve(g2.clone()).await;
    let ada2 = sign_in(&app2, "ada").await;
    let mut back = Page::open(&addr2, &ada2, &path, Some(kept.doc)).await;
    assert_eq!(back.info["epoch"], epoch);
    back.listen(SHORT).await;
    let mut d = Page::open(&addr2, &ada2, &path, None).await;
    d.listen(SHORT).await;
    assert_eq!(d.now().matches("Kopukken.").count(), 1, "{}", d.now());
    assert_eq!(d.now().matches("Sıra bende.").count(), 1, "{}", d.now());
    drop((back, d));

    // Changed outside the room: the document starts anew, under a new name.
    tokio::time::sleep(Duration::from_millis(3000)).await;
    std::fs::write(&play, "Başka bir metin.\n").unwrap();
    let e = Page::open(&addr2, &ada2, &path, None).await;
    assert_ne!(e.info["epoch"], epoch);
    assert_eq!(e.now(), "Başka bir metin.\n");

    drop((app, app2, g, g2));
    std::fs::remove_dir_all(&dir).unwrap();
}
