//! Rooms: a project's play, written live by everyone who has it open.
//!
//! The first to open a project's play opens its room, here in the gate:
//! the play as a Yjs document (`yrs`), which the browsers keep in step with
//! over a WebSocket, in Yjs's own protocol. The gate knows who each is and
//! their role in the project, so the room takes changes only from those who
//! may write: in a project written **together**, every writer; in one
//! written **in turns**, whoever holds the pen. Readers watch.
//!
//! The play stays a `.tex` file: the room writes it a moment after the
//! writing stops, when asked (before a PDF), when the last one leaves and
//! when the gate stops, with its history as ever. Beside it,
//! `.<name>.dactile-y` keeps the document as Yjs has it, so editors cut off
//! by a restart come back and merge. When the play changed outside the room
//! (that file no longer says the same), the document starts anew under a
//! new name (its `epoch`): an editor of the old one starts again from it
//! rather than merge two histories, which would write everything twice.
//!
//! Besides Yjs's messages, the room and the page say a few things of their
//! own, as JSON: the room tells each who writes and who holds the pen
//! (`INFO`), and the page asks for the pen, to save, or for a moment alone
//! with the play while it moves scenes around (`ASK`).

use crate::door;
use crate::projects::{self, Mode, Project, Role};
use axum::extract::ws::{Message as Ws, WebSocket};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};
use tokio::sync::broadcast;
use yrs::sync::awareness::AwarenessUpdateEntry;
use yrs::sync::{AwarenessUpdate, Message, SyncMessage};
use yrs::updates::decoder::Decode;
use yrs::updates::encoder::Encode;
use yrs::{ClientID, Doc, GetString, ReadTxn, StateVector, Text, TextRef, Transact, Update};

/// The page asks: `{"do": "pen" | "drop" | "save" | "alone" | "done"}`.
pub const ASK: u8 = 100;
/// The room says who writes: see `Room::info`.
pub const INFO: u8 = 101;
/// The play is on the disk: `{"ok": true}`, or `{"ok": false, "error"}`.
pub const SAVED: u8 = 102;
/// The room is closing on this page: `{"why": "..."}`.
pub const BYE: u8 = 103;
/// What the page wrote was not taken: its copy is no longer the room's,
/// and it loads the play again.
pub const REFUSED: u8 = 104;

/// The play is written this long after the writing stops.
const QUIET: Duration = Duration::from_secs(2);
/// A pen whose holder has been gone this long is free again.
const PEN_AWAY: Duration = Duration::from_secs(30);
/// Moving scenes around keeps the others out at most this long.
const ALONE: Duration = Duration::from_secs(5);
const TICK: Duration = Duration::from_millis(500);

/// The name of the shared text in the document; the page uses the same.
pub const TEXT: &str = "play";

fn l(tr: &str, en: &str) -> String {
    dactile::lang::f(tr, en, &[])
}

/// Someone in a room, as the gate knows them.
#[derive(Clone)]
pub struct Who {
    pub id: String,
    pub name: String,
}

struct Conn {
    who: Who,
    role: Role,
    /// Their editors' ids in Yjs, for their cursors.
    clients: HashSet<ClientID>,
}

#[derive(Clone)]
enum Out {
    /// For everyone but `from`.
    Data { from: u64, bytes: Arc<Vec<u8>> },
    /// Who writes changed: everyone is told again.
    Info,
    /// The room closes on `who`, or on everyone.
    Bye { who: Option<String>, why: String },
}

struct Inner {
    doc: Doc,
    text: TextRef,
    epoch: String,
    conns: HashMap<u64, Conn>,
    /// The cursors: by editor, its clock, its state as said to all, and the
    /// connection it came on.
    cursors: HashMap<ClientID, (u32, Arc<str>, u64)>,
    mode: Mode,
    /// Who writes, in turns, and their name.
    pen: Option<Who>,
    /// Since when the pen's holder has had no connection here.
    pen_away: Option<Instant>,
    /// Someone moving scenes around, alone for a moment.
    alone: Option<(u64, Instant)>,
    /// The last change not yet on the disk.
    changed: Option<Instant>,
    /// The text as it was last written, or read.
    written: String,
    /// The play is gone from the disk.
    gone: bool,
}

pub struct Room {
    pub path: PathBuf,
    pub project: String,
    inner: Mutex<Inner>,
    tx: broadcast::Sender<Out>,
}

/// The rooms open now, by their play.
pub struct Rooms {
    home: PathBuf,
    all: Mutex<HashMap<PathBuf, Arc<Room>>>,
    next: std::sync::atomic::AtomicU64,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// `.<name>.dactile-y` beside the play.
fn state_file(play: &Path) -> PathBuf {
    let name = play
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    play.with_file_name(format!(".{name}.dactile-y"))
}

/// A colour for someone's cursor, the same for them every time.
fn colour(id: &str) -> &'static str {
    const ALL: [&str; 8] = [
        "#8a2a2a", "#1f5a7a", "#2f6b3a", "#7a4f1f", "#5a2f7a", "#1f6b6b", "#7a1f5a", "#4a5a1f",
    ];
    let h = id
        .bytes()
        .fold(0u32, |h, b| h.wrapping_mul(31).wrapping_add(b as u32));
    ALL[h as usize % ALL.len()]
}

fn encode(m: &Message) -> Vec<u8> {
    m.encode_v1()
}

fn said(tag: u8, v: Value) -> Vec<u8> {
    encode(&Message::Custom(tag, v.to_string().into_bytes()))
}

impl Inner {
    /// The play from the disk, and the document Yjs had of it if it still
    /// says the same.
    fn load(path: &Path) -> Result<Inner, String> {
        let text = dactile::file::read(path).map_err(|e| e.to_string())?;
        let kept = std::fs::read(state_file(path)).ok().and_then(|b| {
            let at = b.iter().position(|c| *c == b'\n')?;
            let epoch = String::from_utf8(b[..at].to_vec()).ok()?;
            let doc = Doc::new();
            let t = doc.get_or_insert_text(TEXT);
            let update = Update::decode_v1(&b[at + 1..]).ok()?;
            doc.transact_mut().apply_update(update).ok()?;
            let same = t.get_string(&doc.transact()) == text;
            same.then_some((epoch, doc, t))
        });
        let (epoch, doc, t) = match kept {
            Some(k) => k,
            None => {
                let doc = Doc::new();
                let t = doc.get_or_insert_text(TEXT);
                t.insert(&mut doc.transact_mut(), 0, &text);
                (door::random().map_err(|e| e.to_string())?, doc, t)
            }
        };
        Ok(Inner {
            doc,
            text: t,
            epoch,
            conns: HashMap::new(),
            cursors: HashMap::new(),
            mode: Mode::Turns,
            pen: None,
            pen_away: None,
            alone: None,
            changed: None,
            written: text,
            gone: false,
        })
    }

    fn current(&self) -> String {
        self.text.get_string(&self.doc.transact())
    }

    /// Writes the play, if it changed, and the document beside it.
    fn write(&mut self, path: &Path) -> Result<(), String> {
        if self.gone {
            return Err(l("Oyun silinmiş.", "The play was deleted."));
        }
        if !path.exists() {
            self.gone = true;
            return Err(l("Oyun silinmiş.", "The play was deleted."));
        }
        let text = self.current();
        if text != self.written {
            dactile::file::write_atomic(path, &text).map_err(|e| e.to_string())?;
            let _ = dactile::file::snapshot(&dactile::file::history_dir(path), &text, false);
        }
        let mut state = self.epoch.clone().into_bytes();
        state.push(b'\n');
        state.extend(
            self.doc
                .transact()
                .encode_state_as_update_v1(&StateVector::default()),
        );
        dactile::file::write_atomic_bytes(&state_file(path), &state).map_err(|e| e.to_string())?;
        self.written = text;
        self.changed = None;
        Ok(())
    }

    fn here(&self, id: &str) -> bool {
        self.conns.values().any(|c| c.who.id == id)
    }

    fn writes(&self, conn: u64) -> bool {
        let Some(c) = self.conns.get(&conn) else {
            return false;
        };
        if self.gone || !c.role.writes() {
            return false;
        }
        if self.alone.is_some_and(|(by, _)| by != conn) {
            return false;
        }
        match self.mode {
            Mode::Together => true,
            Mode::Turns => self.pen.as_ref().is_some_and(|p| p.id == c.who.id),
        }
    }

    /// In turns, a free pen goes to the first writer who comes.
    fn give_pen(&mut self) {
        if self.mode != Mode::Turns || self.pen.is_some() {
            return;
        }
        let first = self.conns.values().find(|c| c.role.writes());
        if let Some(c) = first {
            self.pen = Some(c.who.clone());
            self.pen_away = None;
        }
    }

    /// What the room says to `conn`: who writes, and whether they do.
    fn info(&self, conn: u64) -> Value {
        let c = self.conns.get(&conn);
        let mine = |w: &Option<Who>| w.as_ref().zip(c).is_some_and(|(p, c)| p.id == c.who.id);
        let mut people: Vec<&str> = self.conns.values().map(|c| c.who.name.as_str()).collect();
        people.sort();
        people.dedup();
        let role = c.map(|c| c.role);
        let take = role.is_some_and(Role::writes)
            && self.mode == Mode::Turns
            && !mine(&self.pen)
            && (self.pen.is_none() || role == Some(Role::Admin) || self.pen_away.is_some());
        json!({
            "epoch": self.epoch,
            "mode": self.mode,
            "role": role,
            "writes": self.writes(conn),
            "pen": self.pen.as_ref().map(|p| &p.name),
            "mine": mine(&self.pen),
            "take": take,
            "people": people,
            "alone": self.alone.and_then(|(by, _)| self.conns.get(&by)).map(|c| &c.who.name),
            "alone_mine": self.alone.is_some_and(|(by, _)| by == conn),
            "saved": self.changed.is_none(),
            "gone": self.gone,
        })
    }

    /// Every cursor, as one awareness message.
    fn cursors(&self) -> Option<Vec<u8>> {
        if self.cursors.is_empty() {
            return None;
        }
        let clients = self
            .cursors
            .iter()
            .map(|(id, (clock, json, _))| {
                (
                    *id,
                    AwarenessUpdateEntry {
                        clock: *clock,
                        json: json.clone(),
                    },
                )
            })
            .collect();
        Some(encode(&Message::Awareness(AwarenessUpdate { clients })))
    }
}

impl Room {
    fn inner(&self) -> MutexGuard<'_, Inner> {
        lock(&self.inner)
    }

    fn tell(&self, out: Out) {
        let _ = self.tx.send(out);
    }

    /// The play as the room has it now.
    pub fn text(&self) -> String {
        self.inner().current()
    }

    /// Writes the play now.
    pub fn save(&self) -> Result<(), String> {
        let r = self.inner().write(&self.path);
        self.tell(Out::Info);
        r
    }

    /// One message from `conn`; what goes back to it alone.
    fn hear(&self, conn: u64, bytes: &[u8]) -> Vec<Vec<u8>> {
        let Ok(msg) = Message::decode_v1(bytes) else {
            return vec![];
        };
        let mut i = self.inner();
        match msg {
            Message::Sync(SyncMessage::SyncStep1(sv)) => {
                let update = i.doc.transact().encode_state_as_update_v1(&sv);
                vec![encode(&Message::Sync(SyncMessage::SyncStep2(update)))]
            }
            Message::Sync(SyncMessage::SyncStep2(u) | SyncMessage::Update(u)) => {
                if !i.writes(conn) {
                    // Nothing goes on from them. A page that wrote anyway
                    // has a copy of its own now: it starts again.
                    let empty = Update::decode_v1(&u).is_ok_and(|u| u.is_empty());
                    let mut back = vec![said(INFO, i.info(conn))];
                    if !empty {
                        back.push(said(REFUSED, json!({})));
                    }
                    return back;
                }
                let Ok(update) = Update::decode_v1(&u) else {
                    return vec![];
                };
                let before = i.doc.transact().state_vector();
                if i.doc.transact_mut().apply_update(update).is_err() {
                    return vec![];
                }
                if i.doc.transact().state_vector() == before {
                    return vec![];
                }
                let was_saved = i.changed.is_none();
                i.changed = Some(Instant::now());
                drop(i);
                let bytes = Arc::new(encode(&Message::Sync(SyncMessage::Update(u))));
                self.tell(Out::Data { from: conn, bytes });
                if was_saved {
                    self.tell(Out::Info);
                }
                vec![]
            }
            Message::Awareness(update) => {
                let Some(who) = i.conns.get(&conn).map(|c| c.who.clone()) else {
                    return vec![];
                };
                let mut out = HashMap::new();
                for (id, entry) in update.clients {
                    // An editor's cursor is said only by the page it is on.
                    if i.cursors.get(&id).is_some_and(|(_, _, by)| *by != conn) {
                        continue;
                    }
                    if i.cursors.get(&id).is_some_and(|(c, _, _)| *c > entry.clock) {
                        continue;
                    }
                    let json: Arc<str> = match serde_json::from_str::<Value>(&entry.json) {
                        Ok(Value::Object(mut state)) => {
                            // The name is the gate's, not what the page says.
                            let c = colour(&who.id);
                            state.insert(
                                "user".into(),
                                json!({ "name": who.name, "color": c, "colorLight": format!("{c}33") }),
                            );
                            Value::Object(state).to_string().into()
                        }
                        _ => "null".into(),
                    };
                    if &*json == "null" {
                        i.cursors.remove(&id);
                        if let Some(c) = i.conns.get_mut(&conn) {
                            c.clients.remove(&id);
                        }
                    } else {
                        i.cursors.insert(id, (entry.clock, json.clone(), conn));
                        if let Some(c) = i.conns.get_mut(&conn) {
                            c.clients.insert(id);
                        }
                    }
                    out.insert(
                        id,
                        AwarenessUpdateEntry {
                            clock: entry.clock,
                            json,
                        },
                    );
                }
                drop(i);
                if !out.is_empty() {
                    let bytes = Arc::new(encode(&Message::Awareness(AwarenessUpdate {
                        clients: out,
                    })));
                    self.tell(Out::Data { from: conn, bytes });
                }
                vec![]
            }
            Message::AwarenessQuery => i.cursors().into_iter().collect(),
            Message::Custom(ASK, data) => {
                let asked: Value = serde_json::from_slice(&data).unwrap_or_default();
                drop(i);
                self.asked(conn, asked["do"].as_str().unwrap_or(""))
            }
            _ => vec![],
        }
    }

    fn asked(&self, conn: u64, what: &str) -> Vec<Vec<u8>> {
        let mut i = self.inner();
        let Some(c) = i.conns.get(&conn) else {
            return vec![];
        };
        let (who, role) = (c.who.clone(), c.role);
        match what {
            "pen" => {
                let free = i.pen.is_none() || i.pen_away.is_some() || role == Role::Admin;
                if i.mode == Mode::Turns && role.writes() && free {
                    i.pen = Some(who);
                    i.pen_away = None;
                }
            }
            "drop" => {
                if i.pen.as_ref().is_some_and(|p| p.id == who.id) {
                    i.pen = None;
                    // Someone else waiting gets it.
                    let next = i
                        .conns
                        .values()
                        .find(|c| c.role.writes() && c.who.id != who.id)
                        .map(|c| c.who.clone());
                    i.pen = next;
                }
            }
            "save" => {
                let r = i.write(&self.path);
                drop(i);
                self.tell(Out::Info);
                return vec![said(
                    SAVED,
                    match r {
                        Ok(()) => json!({ "ok": true }),
                        Err(e) => json!({ "ok": false, "error": e }),
                    },
                )];
            }
            "alone" => {
                if i.alone.is_none() && i.writes(conn) {
                    i.alone = Some((conn, Instant::now()));
                }
            }
            "done" => {
                if i.alone.is_some_and(|(by, _)| by == conn) {
                    i.alone = None;
                }
            }
            _ => return vec![],
        }
        drop(i);
        self.tell(Out::Info);
        vec![]
    }

    /// `conn` is gone: its cursors with it, and the pen in a while.
    fn leave(&self, conn: u64) {
        let mut i = self.inner();
        let Some(c) = i.conns.remove(&conn) else {
            return;
        };
        if i.alone.is_some_and(|(by, _)| by == conn) {
            i.alone = None;
        }
        let mut out = HashMap::new();
        for id in c.clients {
            if let Some((clock, _, _)) = i.cursors.remove(&id) {
                out.insert(
                    id,
                    AwarenessUpdateEntry {
                        clock: clock + 1,
                        json: "null".into(),
                    },
                );
            }
        }
        if i.pen.as_ref().is_some_and(|p| p.id == c.who.id) && !i.here(&c.who.id) {
            i.pen_away = Some(Instant::now());
        }
        drop(i);
        if !out.is_empty() {
            let bytes = Arc::new(encode(&Message::Awareness(AwarenessUpdate {
                clients: out,
            })));
            self.tell(Out::Data { from: conn, bytes });
        }
        self.tell(Out::Info);
    }

    /// Every half second: writes the play once the writing stopped, frees
    /// a pen left behind. Says whether the room is empty and done.
    fn tick(&self) -> bool {
        let mut i = self.inner();
        let mut told = false;
        if i.changed.is_some_and(|t| t.elapsed() >= QUIET) && !i.gone {
            if let Err(e) = i.write(&self.path) {
                eprintln!("{}: {e}", self.path.display());
            }
            told = true;
        }
        if i.pen_away.is_some_and(|t| t.elapsed() >= PEN_AWAY) {
            i.pen = None;
            i.pen_away = None;
            i.give_pen();
            told = true;
        }
        if i.alone.is_some_and(|(_, t)| t.elapsed() >= ALONE) {
            i.alone = None;
            told = true;
        }
        let done = i.conns.is_empty() && (i.changed.is_none() || i.gone);
        drop(i);
        if told {
            self.tell(Out::Info);
        }
        done
    }

    /// The project's members, roles or way of writing changed.
    fn project_is(&self, p: Option<&Project>, by: Option<&str>) {
        let mut i = self.inner();
        let Some(p) = p else {
            drop(i);
            self.tell(Out::Bye {
                who: None,
                why: l("Proje yok artık.", "The project is gone."),
            });
            return;
        };
        let was = i.mode;
        i.mode = p.mode;
        let mut out = vec![];
        for c in i.conns.values_mut() {
            match p.members.get(&c.who.id) {
                Some(r) => c.role = *r,
                None => out.push(c.who.id.clone()),
            }
        }
        if was != p.mode {
            i.pen = None;
            i.pen_away = None;
            // Into turns: the pen is theirs who said so, if they are here.
            if p.mode == Mode::Turns
                && let Some(by) = by
                && let Some(c) = i.conns.values().find(|c| c.who.id == by && c.role.writes())
            {
                i.pen = Some(c.who.clone());
            }
        }
        if let Some(pen) = i.pen.clone()
            && !p.members.get(&pen.id).is_some_and(|r| r.writes())
        {
            i.pen = None;
        }
        i.give_pen();
        drop(i);
        for who in out {
            self.tell(Out::Bye {
                who: Some(who),
                why: l(
                    "Artık bu projede değilsin.",
                    "You are no longer in this project.",
                ),
            });
        }
        self.tell(Out::Info);
    }

    /// Talks with one page until it goes.
    async fn serve(
        self: Arc<Self>,
        conn: u64,
        mut ws: WebSocket,
        mut rx: broadcast::Receiver<Out>,
    ) {
        let me = self.inner().conns.get(&conn).map(|c| c.who.id.clone());
        let first = {
            let i = self.inner();
            let mut first = vec![said(INFO, i.info(conn))];
            let sv = i.doc.transact().state_vector();
            first.push(encode(&Message::Sync(SyncMessage::SyncStep1(sv))));
            first.extend(i.cursors());
            first
        };
        let mut open = true;
        for b in first {
            open &= ws.send(Ws::Binary(b.into())).await.is_ok();
        }
        while open {
            tokio::select! {
                got = ws.recv() => match got {
                    Some(Ok(Ws::Binary(b))) => {
                        for back in self.hear(conn, &b) {
                            if ws.send(Ws::Binary(back.into())).await.is_err() {
                                open = false;
                            }
                        }
                    }
                    Some(Ok(Ws::Close(_))) | None | Some(Err(_)) => open = false,
                    Some(Ok(_)) => {}
                },
                out = rx.recv() => match out {
                    Ok(Out::Data { from, bytes }) if from != conn => {
                        open = ws.send(Ws::Binary(bytes.to_vec().into())).await.is_ok();
                    }
                    Ok(Out::Data { .. }) => {}
                    Ok(Out::Info) => {
                        let info = self.inner().info(conn);
                        open = ws.send(Ws::Binary(said(INFO, info).into())).await.is_ok();
                    }
                    Ok(Out::Bye { who, why }) if who.is_none() || who == me => {
                        let _ = ws.send(Ws::Binary(said(BYE, json!({ "why": why })).into())).await;
                        let _ = ws.send(Ws::Close(None)).await;
                        open = false;
                    }
                    Ok(Out::Bye { .. }) => {}
                    // Fell behind: what it missed is lost to it. It comes
                    // back and catches up.
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        let _ = ws.send(Ws::Close(None)).await;
                        open = false;
                    }
                    Err(broadcast::error::RecvError::Closed) => open = false,
                },
            }
        }
        self.leave(conn);
    }
}

impl Rooms {
    pub fn new(home: PathBuf) -> Arc<Rooms> {
        Arc::new(Rooms {
            home,
            all: Mutex::new(HashMap::new()),
            next: std::sync::atomic::AtomicU64::new(1),
        })
    }

    /// The play at `path`, if it is a project's `who` is in: the play, the
    /// project and their role there.
    pub fn find(&self, path: &str, who: &str) -> Result<(PathBuf, Project, Role), String> {
        let not = || {
            l(
                "Bu oyun senin projelerinden birinde değil.",
                "This play isn't in a project of yours.",
            )
        };
        let play = Path::new(path).canonicalize().map_err(|_| not())?;
        if play.extension().is_none_or(|e| e != "tex") {
            return Err(not());
        }
        for (p, role) in projects::of(&self.home, who) {
            let Ok(dir) = projects::folder(&self.home, &p.id).canonicalize() else {
                continue;
            };
            if play.starts_with(&dir) {
                return Ok((play, p, role));
            }
        }
        Err(not())
    }

    /// Opens the room of `path` (or comes into the one open) for `who`, and
    /// talks with their page until it goes.
    pub async fn join(self: &Arc<Self>, path: &str, who: Who, ws: WebSocket) {
        let (room, conn, rx) = match self.enter(path, who) {
            Ok(r) => r,
            Err(why) => {
                let mut ws = ws;
                let _ = ws
                    .send(Ws::Binary(said(BYE, json!({ "why": why })).into()))
                    .await;
                let _ = ws.send(Ws::Close(None)).await;
                return;
            }
        };
        room.serve(conn, ws, rx).await;
    }

    #[allow(clippy::type_complexity)]
    fn enter(
        self: &Arc<Self>,
        path: &str,
        who: Who,
    ) -> Result<(Arc<Room>, u64, broadcast::Receiver<Out>), String> {
        let (play, project, role) = self.find(path, &who.id)?;
        let mut all = lock(&self.all);
        let room = match all.get(&play) {
            Some(r) => r.clone(),
            None => {
                let mut inner = Inner::load(&play)?;
                inner.mode = project.mode;
                let room = Arc::new(Room {
                    path: play.clone(),
                    project: project.id.clone(),
                    inner: Mutex::new(inner),
                    tx: broadcast::channel(1024).0,
                });
                all.insert(play.clone(), room.clone());
                let (rooms, r) = (self.clone(), room.clone());
                tokio::spawn(async move {
                    loop {
                        tokio::time::sleep(TICK).await;
                        let mut all = lock(&rooms.all);
                        if r.tick() {
                            if all.get(&r.path).is_some_and(|x| Arc::ptr_eq(x, &r)) {
                                all.remove(&r.path);
                            }
                            break;
                        }
                    }
                });
                room
            }
        };
        let conn = self.next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let rx = room.tx.subscribe();
        {
            let mut i = room.inner();
            if i.gone {
                return Err(l("Oyun silinmiş.", "The play was deleted."));
            }
            i.conns.insert(
                conn,
                Conn {
                    who: who.clone(),
                    role,
                    clients: HashSet::new(),
                },
            );
            if i.pen.as_ref().is_some_and(|p| p.id == who.id) {
                i.pen_away = None;
            }
            i.give_pen();
        }
        drop(all);
        room.tell(Out::Info);
        Ok((room, conn, rx))
    }

    /// The room of a play, if it is open.
    pub fn open(&self, play: &Path) -> Option<Arc<Room>> {
        let play = play.canonicalize().ok()?;
        lock(&self.all).get(&play).cloned()
    }

    fn of(&self, project: Option<&str>) -> Vec<Arc<Room>> {
        lock(&self.all)
            .values()
            .filter(|r| project.is_none_or(|p| r.project == p))
            .cloned()
            .collect()
    }

    /// A project changed (its members, their roles, how it is written; or
    /// it is gone): its rooms hear of it. `by` changed it.
    pub fn project_changed(&self, id: &str, by: Option<&str>) {
        let p = projects::find(&self.home, id);
        for r in self.of(Some(id)) {
            r.project_is(p.as_ref(), by);
        }
    }

    /// Someone left the site: every room hears it.
    pub fn people_changed(&self) {
        for r in self.of(None) {
            let p = projects::find(&self.home, &r.project);
            r.project_is(p.as_ref(), None);
        }
    }

    /// Plays may have gone (deleted, moved): their rooms close.
    pub fn check_gone(&self) {
        for r in self.of(None) {
            if !r.path.exists() {
                r.inner().gone = true;
                r.tell(Out::Bye {
                    who: None,
                    why: l(
                        "Oyun silindi ya da taşındı.",
                        "The play was deleted or moved.",
                    ),
                });
            }
        }
    }

    /// Writes what `who` and the others wrote in the rooms they are in,
    /// before their server reads the plays from the disk (for a PDF).
    pub fn save_for(&self, who: &str) {
        for r in self.of(None) {
            let mut i = r.inner();
            if i.changed.is_some() && i.here(who) {
                if let Err(e) = i.write(&r.path) {
                    eprintln!("{}: {e}", r.path.display());
                }
                drop(i);
                r.tell(Out::Info);
            }
        }
    }

    /// Writes every open play, as the gate stops.
    pub fn save_all(&self) {
        for r in self.of(None) {
            if let Err(e) = r.inner().write(&r.path) {
                eprintln!("{}: {e}", r.path.display());
            }
        }
    }
}
