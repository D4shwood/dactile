//! Dactile's desktop app: a Svelte front end over the `dactile` library.
//! Each command here passes a call from the window on to `play` or `view`.

pub mod casting;
pub mod complete;
pub mod cover;
pub mod importing;
pub mod notes;
pub mod people;
pub mod play;
pub mod plays;
pub mod scenes;
pub mod spell;
pub mod summary;
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
    /// The draft kept before the play's shape first changed.
    kept: Option<PathBuf>,
    /// A document being imported, while its preview is open.
    importing: Option<(PathBuf, dactile::import::shape::Draft)>,
    /// The play last thrown away, to bring back.
    thrown: Option<Vec<dactile::trash::Moved>>,
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

/// The home screen's menu: a play's family to the imported tab, or back.
#[tauri::command]
fn play_imported(app: Shared, path: String, on: bool) -> Res<()> {
    let cfg = app.lock().unwrap().cfg.clone();
    let plays = dactile::library::list(&cfg);
    dactile::library::set_imported(&plays, std::path::Path::new(&path), on).map_err(err)
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
fn create(
    app: Shared,
    template: usize,
    title: String,
    details: std::collections::BTreeMap<String, String>,
) -> Res<String> {
    let a = app.lock().unwrap();
    play::create(&a.cfg, template, &title, &details)
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
    a.kept = None;
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

/// The play named on the command line (`dactile oyun.tex`), once.
#[tauri::command]
fn startup(app: Shared) -> Option<String> {
    let mut a = app.lock().unwrap();
    if a.started {
        return None;
    }
    a.started = true;
    // An AppImage runs from inside itself, and says where it was started.
    let here = std::env::var_os("OWD")
        .map(PathBuf::from)
        .unwrap_or_default();
    std::env::args_os()
        .skip(1)
        .map(PathBuf::from)
        .find(|p| p.extension().is_some_and(|e| e == "tex"))
        .and_then(|p| here.join(p).canonicalize().ok())
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
fn note_fix_many(app: Shared, text: String, which: Vec<usize>) -> Res<people::Made> {
    let mut a = app.lock().unwrap();
    let made = notes::fix_many(&mut a.notes, &text, &which);
    a.keep_notes()?;
    Ok(made)
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
fn cast_others(text: String) -> String {
    people::others(&text)
}

#[tauri::command]
fn cast_others_set(text: String, words: String) -> Option<people::Made> {
    people::set_others(&text, &words)
}

#[tauri::command]
fn character_shift(text: String, alias: String, up: bool) -> Option<people::Made> {
    people::shift(&text, &alias, up)
}

fn sidecar(app: &Shared) -> Res<(PathBuf, dactile::sidecar::Sidecar)> {
    let a = app.lock().unwrap();
    let path = a.open.as_ref().ok_or("no play open")?.path.clone();
    let side = dactile::sidecar::Sidecar::load(&path).map_err(err)?;
    Ok((path, side))
}

#[tauri::command]
fn summaries(app: Shared, text: String) -> Res<Vec<summary::Summary>> {
    Ok(summary::list(&text, &sidecar(&app)?.1))
}

#[tauri::command]
fn summary_here(app: Shared, text: String, at: usize) -> Res<summary::Here> {
    summary::here(&text, at, &sidecar(&app)?.1)
}

#[tauri::command]
fn summary_set(app: Shared, text: String, at: usize, summary: String) -> Res<Option<Change>> {
    let (path, _) = sidecar(&app)?;
    summary::set(&text, at, &summary, &path).map_err(err)
}

#[tauri::command]
fn scene_split(app: Shared, text: String, at: usize) -> Res<people::Made> {
    let mut a = app.lock().unwrap();
    let path = a.open.as_ref().ok_or("no play open")?.path.clone();
    scenes::split(&text, at, &path, &mut a.kept)
}

/// The play as it stands on the page, as a Word document beside it.
#[tauri::command]
fn docx_export(path: String, text: String) -> Res<String> {
    let out =
        dactile::word::export(std::path::Path::new(&path), &text).map_err(|e| e.to_string())?;
    Ok(out.display().to_string())
}

#[tauri::command]
fn cover_read(text: String) -> cover::CoverForm {
    cover::read(&text)
}

#[tauri::command]
fn cover_write(text: String, form: cover::CoverForm) -> people::Made {
    cover::write(&text, &form)
}

#[tauri::command]
fn play_font_size_write(text: String, size: String) -> people::Made {
    cover::font_size(&text, &size)
}

#[tauri::command]
fn play_font_write(text: String, font: String) -> people::Made {
    cover::font(&text, &font)
}

#[tauri::command]
fn scene_align_next(text: String) -> Res<people::Made> {
    scenes::next_align(&text)
}

#[tauri::command]
fn scene_join(app: Shared, text: String, at: usize) -> Res<people::Made> {
    let mut a = app.lock().unwrap();
    let path = a.open.as_ref().ok_or("no play open")?.path.clone();
    scenes::join(&text, at, &path, &mut a.kept)
}

/// An error in the window, written where `npm run tauri dev` shows it.
#[tauri::command]
fn casting_roles(app: Shared, text: String) -> Res<casting::Roles> {
    let default = app.lock().unwrap().cfg.wpm;
    let side = sidecar(&app)?.1;
    Ok(casting::roles(&text, &side, casting::wpm(&side, default)))
}

#[tauri::command]
fn casting_edit(app: Shared, edit: casting::Edit) -> Res<()> {
    let (path, mut side) = sidecar(&app)?;
    casting::edit(&mut side, edit)?;
    side.save(&path).map_err(err)
}

#[tauri::command]
fn casting_csv(app: Shared, text: String) -> Res<String> {
    let (path, side) = sidecar(&app)?;
    let out = casting::table_csv(&text, &side, &path).map_err(err)?;
    Ok(out.display().to_string())
}

#[tauri::command]
async fn casting_pdf(app: Shared<'_>, text: String) -> Res<String> {
    let (path, side) = sidecar(&app)?;
    tauri::async_runtime::spawn_blocking(move || casting::table_pdf(&text, &side, &path))
        .await
        .map_err(|e| e.to_string())?
        .map(|p| p.display().to_string())
}

#[tauri::command]
async fn casting_copies(
    app: Shared<'_>,
    text: String,
    kind: casting::Kind,
) -> Res<(String, usize)> {
    let default = app.lock().unwrap().cfg.wpm;
    let (path, side) = sidecar(&app)?;
    let wpm = casting::wpm(&side, default);
    tauri::async_runtime::spawn_blocking(move || casting::copies(&text, &side, &path, wpm, kind))
        .await
        .map_err(|e| e.to_string())?
        .map(|(dir, n)| (dir.display().to_string(), n))
}

#[tauri::command]
fn memo(app: Shared, text: String, role: String, at: usize) -> Res<casting::Memo> {
    Ok(casting::memo(&text, &role, at, &sidecar(&app)?.1))
}

#[tauri::command]
async fn pick_import() -> Option<String> {
    rfd::AsyncFileDialog::new()
        .add_filter(
            dactile::lang::l("Oyun belgeleri", "Play documents"),
            &dactile::import::EXTENSIONS,
        )
        .pick_file()
        .await
        .map(|f| f.path().display().to_string())
}

/// Reads a document (a PDF takes a while) and keeps the draft for the
/// preview.
#[tauri::command]
async fn import_read(
    app: Shared<'_>,
    handle: tauri::AppHandle,
    path: String,
    ocr: bool,
) -> Res<importing::Shown> {
    use tauri::Emitter;
    let lang = play::lang_of(&app.lock().unwrap().cfg);
    let file = PathBuf::from(path);
    let from = file.clone();
    let draft = tauri::async_runtime::spawn_blocking(move || {
        importing::read(&from, lang, ocr, &|done, total| {
            let _ = handle.emit("ocr", (done, total));
        })
    })
    .await
    .map_err(|e| e.to_string())??;
    let shown = importing::shown(&file, &draft);
    app.lock().unwrap().importing = Some((file, draft));
    Ok(shown)
}

#[tauri::command]
fn import_stop() {
    importing::STOP.store(true, std::sync::atomic::Ordering::Relaxed);
}

#[tauri::command]
fn import_edit(app: Shared, edit: importing::Edit) -> Res<importing::Shown> {
    let mut a = app.lock().unwrap();
    let (file, draft) = a.importing.as_mut().ok_or("no import open")?;
    importing::edit(draft, edit)?;
    Ok(importing::shown(file, draft))
}

#[tauri::command]
fn import_doc(app: Shared) -> Res<Vec<importing::Para>> {
    let a = app.lock().unwrap();
    let (_, draft) = a.importing.as_ref().ok_or("no import open")?;
    Ok(importing::document(draft))
}

/// The play written: where it is, to open.
#[tauri::command]
fn import_write(app: Shared) -> Res<String> {
    let mut a = app.lock().unwrap();
    let (file, draft) = a.importing.as_ref().ok_or("no import open")?;
    let path = importing::write(file, draft, &a.cfg).map_err(err)?;
    a.importing = None;
    Ok(path.display().to_string())
}

#[tauri::command]
fn import_volume(app: Shared) -> Res<Vec<importing::PlayLine>> {
    let a = app.lock().unwrap();
    let (_, draft) = a.importing.as_ref().ok_or("no import open")?;
    Ok(importing::volume(draft))
}

/// The chosen plays of the volume written; the import stays open for the
/// list of what became of them.
#[tauri::command]
fn import_write_all(
    app: Shared,
    picked: Vec<usize>,
    book: Option<importing::Book>,
) -> Res<Vec<importing::Written>> {
    let a = app.lock().unwrap();
    let (file, draft) = a.importing.as_ref().ok_or("no import open")?;
    Ok(importing::write_all(
        file,
        draft,
        &picked,
        book.as_ref(),
        &a.cfg,
    ))
}

/// The book as one PDF, a cover and contents before its plays: where it is.
#[tauri::command]
async fn book_pdf(app: Shared<'_>, path: String) -> Res<String> {
    let lang = play::lang_of(&app.lock().unwrap().cfg);
    tauri::async_runtime::spawn_blocking(move || {
        dactile::book::pdf(std::path::Path::new(&path), lang)
            .map(|p| p.display().to_string())
            .map_err(err)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// These plays as the parts of a new book, in this order: where it is.
#[tauri::command]
fn book_gather(app: Shared, paths: Vec<String>, title: String) -> Res<String> {
    let a = app.lock().unwrap();
    let first = paths.first().ok_or("no plays")?;
    let first = std::path::Path::new(first);
    let dir = first.parent().ok_or("no folder")?;
    let author = dactile::library::read(first, a.cfg.wpm).map_or(String::new(), |p| p.author);
    let book = dactile::book::make(dir, &title, &author, "", play::lang_of(&a.cfg)).map_err(err)?;
    for (at, p) in paths.iter().enumerate() {
        dactile::book::join(&book, std::path::Path::new(p), at).map_err(err)?;
    }
    Ok(book.display().to_string())
}

#[tauri::command]
fn import_drop(app: Shared) {
    app.lock().unwrap().importing = None;
}

#[tauri::command]
fn fork_start(path: String, draft: Option<String>, today: String) -> plays::Start {
    plays::fork_start(std::path::Path::new(&path), draft.as_deref(), &today)
}

#[tauri::command]
fn fork_file(path: String, draft: String) -> String {
    dactile::library::fork_name(std::path::Path::new(&path), &draft)
}

#[tauri::command]
fn copy_start(title: String) -> plays::Start {
    plays::copy_start(&title)
}

#[tauri::command]
fn title_file(title: String) -> String {
    dactile::library::file_name(&title)
}

#[tauri::command]
fn rename_start(path: String) -> plays::Start {
    plays::rename_start(std::path::Path::new(&path))
}

#[tauri::command]
fn play_make(
    app: Shared,
    path: String,
    title: Option<String>,
    draft: Option<String>,
    file: String,
) -> Res<String> {
    let cfg = &mut app.lock().unwrap().cfg;
    plays::make(
        std::path::Path::new(&path),
        title.as_deref(),
        draft.as_deref(),
        &file,
        cfg,
    )
    .map(|p| p.display().to_string())
}

#[tauri::command]
fn play_rename(app: Shared, path: String, title: String, file: String) -> Res<String> {
    let cfg = &mut app.lock().unwrap().cfg;
    plays::rename(std::path::Path::new(&path), &title, &file, cfg).map(|p| p.display().to_string())
}

#[tauri::command]
fn play_drafts(app: Shared, path: String) -> usize {
    plays::drafts(std::path::Path::new(&path), &app.lock().unwrap().cfg)
}

#[tauri::command]
fn play_throw(app: Shared, path: String) -> Res<()> {
    let mut a = app.lock().unwrap();
    let moved = plays::throw(std::path::Path::new(&path), &mut a.cfg)?;
    a.thrown = Some(moved);
    Ok(())
}

/// Brings back the play last thrown away: where it is.
#[tauri::command]
fn play_restore(app: Shared) -> Res<String> {
    let mut a = app.lock().unwrap();
    let moved = a.thrown.take().ok_or("nothing thrown away")?;
    plays::restore(&moved, &mut a.cfg)?;
    Ok(moved[0].from.display().to_string())
}

/// The open play's own settings.
#[derive(serde::Serialize)]
struct PlaySettings {
    /// Its pace, if it has one of its own.
    wpm: Option<u32>,
    default_wpm: u32,
    /// The words its spell checker accepts.
    words: Vec<String>,
    /// The play it is a draft of.
    parent: Option<String>,
}

#[tauri::command]
fn play_settings(app: Shared) -> Res<PlaySettings> {
    let default_wpm = app.lock().unwrap().cfg.wpm;
    let side = sidecar(&app)?.1;
    Ok(PlaySettings {
        wpm: side.wpm,
        default_wpm,
        words: side.words,
        parent: side.parent,
    })
}

/// Its pace: none, the default again.
#[tauri::command]
fn play_wpm(app: Shared, wpm: Option<u32>) -> Res<()> {
    let (path, mut side) = sidecar(&app)?;
    match wpm {
        Some(wpm) => casting::edit(&mut side, casting::Edit::Wpm { wpm })?,
        None => side.wpm = None,
    }
    side.save(&path).map_err(err)
}

/// A word out of the play's dictionary.
#[tauri::command]
fn play_unlearn(app: Shared, word: String) -> Res<()> {
    let (path, mut side) = sidecar(&app)?;
    side.words.retain(|w| *w != word);
    side.save(&path).map_err(err)?;
    app.lock().unwrap().known.set_words(&side.words);
    Ok(())
}

/// A file or a folder, in the desktop's own program for it.
#[tauri::command]
fn launch(path: String) -> Res<()> {
    dactile::build::launch(std::path::Path::new(&path)).map_err(err)
}

/// Where a play's things are. It reads the plays beside it: off the main
/// thread.
#[tauri::command]
async fn files(path: String) -> Res<plays::Files> {
    tauri::async_runtime::spawn_blocking(move || plays::files(std::path::Path::new(&path)))
        .await
        .map_err(|e| e.to_string())
}

/// A file selected in the desktop's file manager. It may wait for an answer
/// over D-Bus: off the main thread.
#[tauri::command]
async fn reveal(path: String) -> Res<()> {
    tauri::async_runtime::spawn_blocking(move || {
        dactile::build::reveal(std::path::Path::new(&path)).map_err(err)
    })
    .await
    .map_err(|e| e.to_string())?
}

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

/// The window over the whole screen, for the PDF viewer.
#[tauri::command]
fn full_screen(window: tauri::WebviewWindow, on: bool) -> Res<()> {
    window.set_fullscreen(on).map_err(|e| e.to_string())
}

/// Pages of a PDF to the default printer. `lp` may take a moment: off the
/// main thread.
#[tauri::command]
async fn pdf_print(path: String, first: usize, last: usize, copies: usize) -> Res<()> {
    tauri::async_runtime::spawn_blocking(move || {
        dactile::build::print(std::path::Path::new(&path), first, last, copies).map_err(err)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// A web address from a PDF's link, in the desktop's browser.
#[tauri::command]
fn open_url(url: String) -> Res<()> {
    dactile::build::open_url(&url).map_err(err)
}

/// Where a line of the open play is in its PDF. Runs a program: off the main
/// thread.
#[tauri::command]
async fn sync_view(
    app: Shared<'_>,
    line: usize,
    column: usize,
) -> Res<Vec<dactile::synctex::Place>> {
    let path = open_path(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        dactile::synctex::view(&path, line, column).map_err(err)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// The open play's line behind a point of its PDF, in points from the page's
/// top left.
#[tauri::command]
async fn sync_edit(app: Shared<'_>, pdf: String, page: usize, x: f32, y: f32) -> Res<usize> {
    let path = open_path(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        let found = dactile::synctex::edit(std::path::Path::new(&pdf), page, x, y).map_err(err)?;
        // A class file's line is no use to the writer.
        if found.file.file_name() != path.file_name() {
            return Err(dactile::lang::l(
                "PDF'in bu yeri oyunun bir satırından gelmiyor.",
                "This place in the PDF doesn't come from a line of the play.",
            )
            .to_string());
        }
        Ok(found.line)
    })
    .await
    .map_err(|e| e.to_string())?
}

fn open_path(app: &Shared) -> Res<PathBuf> {
    let a = app.lock().unwrap();
    Ok(a.open.as_ref().ok_or("no play open")?.path.clone())
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

/// The coloured keyboard, in this app's keys and the chosen layout.
#[tauri::command]
fn keys_view() -> KeysView {
    use dactile::keys::{self, BINDS, Cat, FKEYS, Only};
    let layout = dactile::config::Config::load().map_or("q".to_string(), |c| c.keyboard);
    let [a, b, c] = keys::rows(&layout);
    KeysView {
        rows: vec![
            FKEYS.to_vec(),
            a.to_vec(),
            b.to_vec(),
            c.to_vec(),
            keys::special(Only::Desktop),
        ],
        binds: BINDS
            .iter()
            .filter(|b| b.only.fits(Only::Desktop))
            .map(|b| KeyView {
                cap: b.cap(&layout),
                ctrl: b.ctrl,
                alt: b.alt,
                cat: b.cat.id(),
                combo: b.combo(&layout),
                what: b.what(),
            })
            .collect(),
        cats: Cat::ALL.iter().map(|c| (c.id(), c.name())).collect(),
    }
}

#[derive(serde::Serialize)]
struct KeysView {
    rows: Vec<Vec<&'static str>>,
    binds: Vec<KeyView>,
    cats: Vec<(&'static str, &'static str)>,
}

#[derive(serde::Serialize)]
struct KeyView {
    cap: &'static str,
    ctrl: bool,
    alt: bool,
    cat: &'static str,
    combo: String,
    what: &'static str,
}

/// What the commands do, group by group.
#[tauri::command]
fn guide_view() -> Vec<GuideGroup> {
    use dactile::guide::{ENTRIES, Group};
    Group::ALL
        .iter()
        .map(|g| GuideGroup {
            name: g.name(),
            entries: ENTRIES
                .iter()
                .filter(|e| e.group == *g)
                .map(|e| GuideEntry {
                    name: e.name,
                    what: e.what(),
                    example: e.example,
                })
                .collect(),
        })
        .collect()
}

#[derive(serde::Serialize)]
struct GuideGroup {
    name: &'static str,
    entries: Vec<GuideEntry>,
}

#[derive(serde::Serialize)]
struct GuideEntry {
    name: &'static str,
    what: &'static str,
    example: &'static str,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Wayland desktops match a window to its menu entry by this name alone:
    // left at the binary's name, the window gets the generic W icon.
    #[cfg(target_os = "linux")]
    glib::set_prgname(Some("io.github.d4shwood.dactile"));
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
            keys_view,
            play_imported,
            guide_view,
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
            casting_roles,
            casting_edit,
            casting_csv,
            casting_pdf,
            casting_copies,
            memo,
            pick_import,
            import_read,
            import_stop,
            import_edit,
            import_doc,
            import_write,
            import_volume,
            import_write_all,
            book_pdf,
            book_gather,
            import_drop,
            fork_start,
            fork_file,
            copy_start,
            title_file,
            rename_start,
            play_make,
            play_rename,
            play_drafts,
            play_throw,
            play_restore,
            play_settings,
            play_wpm,
            play_unlearn,
            launch,
            reveal,
            files,
            notes_list,
            note_add,
            note_set,
            note_reply,
            notes_paste,
            pick_text,
            note_fix,
            note_fix_many,
            note_authors,
            pdf,
            full_screen,
            pdf_print,
            open_url,
            sync_view,
            sync_edit,
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
            cast_others,
            cast_others_set,
            character_shift,
            summaries,
            summary_here,
            summary_set,
            scene_split,
            scene_join,
            scene_align_next,
            play_font_write,
            play_font_size_write,
            cover_read,
            cover_write,
            docx_export,
        ])
        .run(tauri::generate_context!())
        .expect("the window could not open");
}
