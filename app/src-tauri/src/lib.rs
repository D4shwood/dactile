//! Dactile's desktop app: a Svelte front end over the `dactile` library.
//! Each command here passes a call from the window on to `play` or `view`.

pub mod complete;
pub mod notes;
pub mod people;
pub mod play;
pub mod spell;
pub mod view;

use dactile::config::Config;
use dactile::notes::Notes;
use notes::NoteCard;
use play::{BuildResult, Card, Change, Home, Open, Opened, Preview, Saved, Settings};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::State;

#[derive(Default)]
struct App {
    cfg: Config,
    open: Option<Open>,
    /// The command line's play was asked for.
    started: bool,
    /// The open play's notes.
    notes: Notes,
    /// The open play's own words, and the spell checker's answers.
    known: spell::Known,
}

impl App {
    /// The notes changed: written beside the play.
    fn keep_notes(&self) -> Res<()> {
        let open = self.open.as_ref().ok_or("no play open")?;
        self.notes.save(&open.path).map_err(err)
    }
}

type Shared<'a> = State<'a, Mutex<App>>;
type Res<T> = Result<T, String>;

fn err(e: anyhow::Error) -> String {
    format!("{e:#}")
}

#[tauri::command]
fn home(app: Shared) -> Home {
    let mut a = app.lock().unwrap();
    a.cfg = play::load_config();
    play::home(&a.cfg)
}

#[tauri::command]
fn settings(app: Shared) -> Settings {
    play::settings(&app.lock().unwrap().cfg)
}

#[tauri::command]
fn set_setting(app: Shared, key: String, value: serde_json::Value) -> Res<()> {
    play::set(&mut app.lock().unwrap().cfg, &key, &value).map_err(err)
}

#[tauri::command]
fn preview(path: String) -> Res<Preview> {
    play::preview(&PathBuf::from(path)).map_err(err)
}

#[tauri::command]
fn create(app: Shared, template: usize, title: String) -> Res<String> {
    let a = app.lock().unwrap();
    play::create(&a.cfg, template, &title)
        .map(|p| p.display().to_string())
        .map_err(err)
}

#[tauri::command]
fn example(app: Shared) -> Res<String> {
    play::example(&app.lock().unwrap().cfg)
        .map(|p| p.display().to_string())
        .map_err(err)
}

#[tauri::command]
fn open(app: Shared, path: String) -> Res<Opened> {
    let mut a = app.lock().unwrap();
    // The play open before goes first, and its lock with it.
    a.open = None;
    let (open, opened) = play::open(&mut a.cfg, &PathBuf::from(path)).map_err(err)?;
    a.notes = notes::load(&open.path);
    let words = dactile::sidecar::Sidecar::load(&open.path)
        .map(|s| s.words)
        .unwrap_or_default();
    a.known.set_words(&words);
    a.open = Some(open);
    Ok(opened)
}

#[tauri::command]
fn close(app: Shared) {
    app.lock().unwrap().open = None;
}

#[tauri::command]
fn save(app: Shared, text: String, force: bool) -> Res<Saved> {
    let mut a = app.lock().unwrap();
    let open = a.open.as_mut().ok_or("no play open")?;
    open.save(&text, force).map_err(err)
}

#[tauri::command]
fn save_template(text: String, name: String) -> Res<String> {
    dactile::templates::save(&text, &name)
        .map(|p| p.display().to_string())
        .map_err(err)
}

#[tauri::command]
fn view(text: String) -> view::View {
    view::view(&text)
}

#[tauri::command]
fn board(text: String) -> Vec<Card> {
    play::board(&text)
}

#[tauri::command]
fn relocate(text: String, card: usize, to: usize) -> Option<Change> {
    play::relocate(&text, card, to)
}

/// XeLaTeX takes seconds: off the main thread.
#[tauri::command]
async fn build(app: Shared<'_>) -> Res<BuildResult> {
    let path = {
        let a = app.lock().unwrap();
        a.open.as_ref().ok_or("no play open")?.path.clone()
    };
    tauri::async_runtime::spawn_blocking(move || play::build_pdf(&path))
        .await
        .map_err(|e| e.to_string())
}

/// The play named on the command line (`dactile-desktop oyun.tex`), once.
#[tauri::command]
fn startup(app: Shared) -> Option<String> {
    let mut a = app.lock().unwrap();
    if a.started {
        return None;
    }
    a.started = true;
    std::env::args_os()
        .skip(1)
        .map(PathBuf::from)
        .find(|p| p.extension().is_some_and(|e| e == "tex"))
        .and_then(|p| p.canonicalize().ok())
        .map(|p| p.display().to_string())
}

#[tauri::command]
fn notes_list(app: Shared, text: String) -> Vec<NoteCard> {
    notes::cards(&text, &app.lock().unwrap().notes)
}

#[tauri::command]
fn note_add(
    app: Shared,
    text: String,
    at: usize,
    author: String,
    body: String,
) -> Res<Vec<NoteCard>> {
    let mut a = app.lock().unwrap();
    notes::add(&mut a.notes, &text, at, &author, &body).map_err(err)?;
    a.keep_notes()?;
    Ok(notes::cards(&text, &a.notes))
}

/// "open", "done", "hidden", "archive", "unarchive" or "delete".
#[tauri::command]
fn note_set(app: Shared, text: String, i: usize, state: String) -> Res<Vec<NoteCard>> {
    let mut a = app.lock().unwrap();
    notes::set_state(&mut a.notes, i, &state).map_err(err)?;
    a.keep_notes()?;
    Ok(notes::cards(&text, &a.notes))
}

#[tauri::command]
fn note_reply(
    app: Shared,
    text: String,
    i: usize,
    author: String,
    reply: String,
) -> Res<Vec<NoteCard>> {
    let mut a = app.lock().unwrap();
    notes::reply(&mut a.notes, i, &author, &reply).map_err(err)?;
    a.keep_notes()?;
    Ok(notes::cards(&text, &a.notes))
}

/// A pasted reply's numbered items as notes: how many came.
#[tauri::command]
fn notes_paste(app: Shared, reply: String, author: String) -> Res<usize> {
    let mut a = app.lock().unwrap();
    let n = notes::paste(&mut a.notes, &reply, &author);
    a.keep_notes()?;
    Ok(n)
}

/// A file's text, for notes from a file.
#[tauri::command]
async fn pick_text() -> Option<String> {
    let f = rfd::AsyncFileDialog::new()
        .add_filter("Metin", &["md", "txt"])
        .pick_file()
        .await?;
    std::fs::read_to_string(f.path()).ok()
}

#[tauri::command]
fn note_fix(app: Shared, text: String, i: usize) -> Res<Vec<Change>> {
    let mut a = app.lock().unwrap();
    let changes = notes::fix(&mut a.notes, &text, i)?;
    a.keep_notes()?;
    Ok(changes)
}

#[tauri::command]
fn note_authors(app: Shared) -> Vec<String> {
    app.lock().unwrap().notes.authors()
}

/// The misspelt words between two places in the text.
#[tauri::command]
fn misspelt(
    app: Shared,
    sp: State<'_, spell::Shared>,
    text: String,
    from: usize,
    to: usize,
) -> spell::Checked {
    spell::misspelt(&sp, &mut app.lock().unwrap().known, &text, from, to)
}

#[tauri::command]
fn suggest(sp: State<'_, spell::Shared>, word: String) -> Vec<String> {
    spell::suggest(&sp, &word)
}

/// A word the play keeps as spelt right, in its `.dactile.toml`.
#[tauri::command]
fn learn(app: Shared, word: String) -> Res<()> {
    let mut a = app.lock().unwrap();
    let path = a.open.as_ref().ok_or("no play open")?.path.clone();
    let mut side = dactile::sidecar::Sidecar::load(&path).map_err(err)?;
    side.add_word(&word);
    side.save(&path).map_err(err)?;
    a.known.set_words(&side.words);
    Ok(())
}

/// The menu after a backslash (or a `#`, with `hash`).
#[tauri::command]
fn complete(text: String, at: usize, query: String, hash: bool) -> Vec<complete::Item> {
    complete::items(&text, at, &query, hash)
}

#[tauri::command]
fn heading(text: String, at: usize, act: bool, place: String, time: String) -> Change {
    complete::heading(&text, at, act, &place, &time)
}

#[tauri::command]
fn fields(text: String) -> complete::Fields {
    complete::fields(&text)
}

#[tauri::command]
fn verbs(text: String) -> Vec<String> {
    complete::verbs(&text)
}

#[tauri::command]
fn cast(text: String, by_lines: bool) -> Vec<people::Row> {
    people::list(&text, by_lines)
}

#[tauri::command]
fn suggest_alias(text: String, name: String) -> String {
    people::suggest_alias(&text, &name)
}

#[tauri::command]
fn character_add(text: String, form: people::Form, at: Option<usize>) -> Res<people::Made> {
    people::add(&text, &form, at)
}

#[tauri::command]
fn renames(text: String, old: String, new: String) -> usize {
    people::renames(&text, &old, &new)
}

/// A character changed; its cards in `.dactile.toml` follow a new alias.
#[tauri::command]
fn character_edit(app: Shared, text: String, old: String, form: people::Form) -> Res<people::Made> {
    let made = people::edit(&text, &old, &form)?;
    let alias = form.alias.trim();
    if alias != old
        && let Some(open) = app.lock().unwrap().open.as_ref()
        && let Ok(mut side) = dactile::sidecar::Sidecar::load(&open.path)
    {
        side.rename(&old, alias);
        side.save(&open.path).map_err(err)?;
    }
    Ok(made)
}

#[tauri::command]
fn character_remove(text: String, alias: String) -> Res<people::Made> {
    people::remove(&text, &alias)
}

#[tauri::command]
fn character_merge(text: String, from: String, into: String) -> Res<people::Made> {
    people::merge(&text, &from, &into)
}

#[tauri::command]
fn character_shift(text: String, alias: String, up: bool) -> Option<people::Made> {
    people::shift(&text, &alias, up)
}

/// An error in the window, written where `npm run tauri dev` shows it.
#[tauri::command]
fn log(message: String) {
    eprintln!("[window] {message}");
}

/// A play picked in the system's file dialog.
#[tauri::command]
async fn pick() -> Option<String> {
    rfd::AsyncFileDialog::new()
        .add_filter("Oyun", &["tex"])
        .pick_file()
        .await
        .map(|f| f.path().display().to_string())
}

/// The PDF's bytes, for the preview.
#[tauri::command]
fn pdf(path: String) -> Res<tauri::ipc::Response> {
    std::fs::read(path)
        .map(tauri::ipc::Response::new)
        .map_err(|e| e.to_string())
}

/// In a debug build with `DACTILE_DEV_EVAL=file`: JavaScript written to that
/// file runs in the window, and the file goes. For trying the app out from
/// a script, with no hands on it.
fn dev_eval(app: tauri::AppHandle) {
    use tauri::Manager;
    let Some(path) = std::env::var_os("DACTILE_DEV_EVAL").filter(|_| cfg!(debug_assertions)) else {
        return;
    };
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_millis(300));
            let Ok(js) = std::fs::read_to_string(&path) else {
                continue;
            };
            let _ = std::fs::remove_file(&path);
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.eval(js);
            }
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(Mutex::new(App::default()))
        .manage(spell::Shared::default())
        .setup(|app| {
            use tauri::Manager;
            spell::load(app.state::<spell::Shared>().inner().clone());
            dev_eval(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            home,
            settings,
            set_setting,
            preview,
            create,
            example,
            open,
            close,
            save,
            save_template,
            view,
            board,
            relocate,
            build,
            pick,
            startup,
            log,
            notes_list,
            note_add,
            note_set,
            note_reply,
            notes_paste,
            pick_text,
            note_fix,
            note_authors,
            pdf,
            misspelt,
            suggest,
            learn,
            complete,
            verbs,
            heading,
            fields,
            cast,
            suggest_alias,
            character_add,
            renames,
            character_edit,
            character_remove,
            character_merge,
            character_shift,
        ])
        .run(tauri::generate_context!())
        .expect("the window could not open");
}
