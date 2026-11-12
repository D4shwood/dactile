//! Every command the window sends, with no window to it: the desktop app
//! calls them through Tauri, the web server over HTTP. Each passes a call on
//! to `play`, `view` and the rest, holding one person's open play.

use crate::notes::NoteCard;
use crate::play::{BuildResult, Card, Change, Home, Open, Opened, Preview, Saved, Settings};
use crate::{
    casting, complete, cover, importing, notes, people, play, plays, scenes, spell, summary, view,
};
use dactile::config::Config;
use dactile::notes::Notes;
use std::path::PathBuf;
use std::sync::Mutex;

/// One person's Dactile: their open play and the spell checker.
pub struct Ctx {
    pub app: Mutex<App>,
    pub spell: spell::Shared,
}

impl Default for Ctx {
    fn default() -> Self {
        Ctx {
            app: Mutex::new(App::default()),
            spell: spell::Shared::default(),
        }
    }
}

impl std::ops::Deref for Ctx {
    type Target = Mutex<App>;
    fn deref(&self) -> &Mutex<App> {
        &self.app
    }
}

type Shared = Ctx;

#[derive(Default)]
pub struct App {
    pub cfg: Config,
    pub open: Option<Open>,
    /// The command line's play was asked for.
    pub started: bool,
    /// The open play's notes.
    pub notes: Notes,
    /// When the notes file was last read or written: another time on disk
    /// means someone else wrote it.
    pub notes_time: Option<std::time::SystemTime>,
    /// The open play's own words, and the spell checker's answers.
    pub known: spell::Known,
    /// The draft kept before the play's shape first changed.
    pub kept: Option<PathBuf>,
    /// A document being imported, while its preview is open.
    pub importing: Option<(PathBuf, dactile::import::shape::Draft)>,
    /// The play last thrown away, to bring back.
    pub thrown: Option<Vec<dactile::trash::Moved>>,
}

impl App {
    /// The notes changed: written beside the play.
    fn keep_notes(&mut self) -> Res<()> {
        let open = self.open.as_ref().ok_or("no play open")?;
        self.notes.save(&open.path).map_err(err)?;
        self.notes_time = dactile::file::modified(&dactile::notes::path_for(&open.path));
        Ok(())
    }

    /// The notes as the file has them now.
    fn read_notes(&mut self) {
        let Some(path) = self.open.as_ref().map(|o| o.path.clone()) else {
            return;
        };
        self.notes_time = dactile::file::modified(&dactile::notes::path_for(&path));
        self.notes = match Notes::load(&path) {
            Ok(n) => n,
            // What can't be read is not written over: one line stands for it.
            Err(e) => Notes {
                list: Vec::new(),
                unread: vec![format!("{e:#}")],
            },
        };
    }

    /// Reads the notes again if someone else wrote the file; whether they
    /// had changed.
    fn fresh_notes(&mut self) -> bool {
        let Some(path) = self.open.as_ref().map(|o| o.path.clone()) else {
            return false;
        };
        if dactile::file::modified(&dactile::notes::path_for(&path)) == self.notes_time {
            return false;
        }
        self.read_notes();
        true
    }

    /// Before a change to the note at an index: the notes as the panel shows
    /// them, or a word that they changed and the panel is to be read again.
    fn same_notes(&mut self) -> Res<()> {
        if self.fresh_notes() {
            return Err(dactile::lang::l(
                "Not dosyası dışarıda değişti ve yeniden okundu; listeye bir daha bakıp tekrar deneyin.",
                "The notes file changed elsewhere and was read again; look at the list and try again.",
            )
            .into());
        }
        Ok(())
    }
}

pub type Res<T> = Result<T, String>;

pub fn err(e: anyhow::Error) -> String {
    format!("{e:#}")
}

pub fn home(app: &Shared) -> Home {
    let mut a = app.lock().unwrap();
    a.cfg = play::load_config();
    play::home(&a.cfg)
}

/// The home screen's menu: a play's family to the imported tab, or back.
pub fn play_imported(app: &Shared, path: String, on: bool) -> Res<()> {
    let cfg = app.lock().unwrap().cfg.clone();
    let plays = dactile::library::list(&cfg);
    dactile::library::set_imported(&plays, std::path::Path::new(&path), on).map_err(err)
}

pub fn settings(app: &Shared) -> Settings {
    play::settings(&app.lock().unwrap().cfg)
}

pub fn set_setting(app: &Shared, key: String, value: serde_json::Value) -> Res<()> {
    play::set(&mut app.lock().unwrap().cfg, &key, &value).map_err(err)
}

pub fn preview(_app: &Shared, path: String) -> Res<Preview> {
    play::preview(&PathBuf::from(path)).map_err(err)
}

pub fn create(
    app: &Shared,
    template: usize,
    title: String,
    details: std::collections::BTreeMap<String, String>,
) -> Res<String> {
    let a = app.lock().unwrap();
    play::create(&a.cfg, template, &title, &details)
        .map(|p| p.display().to_string())
        .map_err(err)
}

pub fn example(app: &Shared) -> Res<String> {
    play::example(&app.lock().unwrap().cfg)
        .map(|p| p.display().to_string())
        .map_err(err)
}

pub fn open(app: &Shared, path: String) -> Res<Opened> {
    let mut a = app.lock().unwrap();
    // The play open before goes first, and its lock with it.
    a.open = None;
    let (open, opened) = play::open(&mut a.cfg, &PathBuf::from(path)).map_err(err)?;
    a.kept = None;
    let words = dactile::sidecar::Sidecar::load(&open.path)
        .map(|s| s.words)
        .unwrap_or_default();
    a.known.set_words(&words);
    a.open = Some(open);
    a.read_notes();
    Ok(opened)
}

pub fn close(app: &Shared) {
    app.lock().unwrap().open = None;
}

pub fn save(app: &Shared, text: String, force: bool) -> Res<Saved> {
    let mut a = app.lock().unwrap();
    let open = a.open.as_mut().ok_or("no play open")?;
    open.save(&text, force).map_err(err)
}

pub fn save_template(_app: &Shared, text: String, name: String) -> Res<String> {
    dactile::templates::save(&text, &name)
        .map(|p| p.display().to_string())
        .map_err(err)
}

pub fn view(_app: &Shared, text: String) -> view::View {
    view::view(&text)
}

pub fn board(_app: &Shared, text: String) -> Vec<Card> {
    play::board(&text)
}

pub fn relocate(_app: &Shared, text: String, card: usize, to: usize) -> Option<Change> {
    play::relocate(&text, card, to)
}

/// XeLaTeX takes seconds: the caller runs this off its main thread.
pub fn build(app: &Shared) -> Res<BuildResult> {
    let path = open_path(app)?;
    Ok(play::build_pdf(&path))
}

pub fn notes_list(app: &Shared, text: String) -> Vec<NoteCard> {
    let mut a = app.lock().unwrap();
    a.fresh_notes();
    notes::cards(&text, &a.notes)
}

pub fn note_add(
    app: &Shared,
    text: String,
    at: usize,
    author: String,
    body: String,
) -> Res<Vec<NoteCard>> {
    let mut a = app.lock().unwrap();
    a.fresh_notes();
    notes::add(&mut a.notes, &text, at, &author, &body).map_err(err)?;
    a.keep_notes()?;
    Ok(notes::cards(&text, &a.notes))
}

/// "open", "done", "hidden", "archive", "unarchive" or "delete".
pub fn note_set(app: &Shared, text: String, i: usize, state: String) -> Res<Vec<NoteCard>> {
    let mut a = app.lock().unwrap();
    a.same_notes()?;
    notes::set_state(&mut a.notes, i, &state).map_err(err)?;
    a.keep_notes()?;
    Ok(notes::cards(&text, &a.notes))
}

pub fn note_reply(
    app: &Shared,
    text: String,
    i: usize,
    author: String,
    reply: String,
) -> Res<Vec<NoteCard>> {
    let mut a = app.lock().unwrap();
    a.same_notes()?;
    notes::reply(&mut a.notes, i, &author, &reply).map_err(err)?;
    a.keep_notes()?;
    Ok(notes::cards(&text, &a.notes))
}

/// A pasted reply's numbered items as notes: how many came.
pub fn notes_paste(app: &Shared, reply: String, author: String) -> Res<usize> {
    let mut a = app.lock().unwrap();
    a.fresh_notes();
    let n = notes::paste(&mut a.notes, &reply, &author);
    a.keep_notes()?;
    Ok(n)
}

pub fn note_fix(app: &Shared, text: String, i: usize) -> Res<Vec<Change>> {
    let mut a = app.lock().unwrap();
    a.same_notes()?;
    let changes = notes::fix(&mut a.notes, &text, i)?;
    a.keep_notes()?;
    Ok(changes)
}

pub fn note_fix_many(app: &Shared, text: String, which: Vec<usize>) -> Res<people::Made> {
    let mut a = app.lock().unwrap();
    a.same_notes()?;
    let made = notes::fix_many(&mut a.notes, &text, &which);
    a.keep_notes()?;
    Ok(made)
}

pub fn note_authors(app: &Shared) -> Vec<String> {
    let mut a = app.lock().unwrap();
    a.fresh_notes();
    a.notes.authors()
}

/// The misspelt words between two places in the text.
pub fn misspelt(app: &Shared, text: String, from: usize, to: usize) -> spell::Checked {
    spell::misspelt(&app.spell, &mut app.lock().unwrap().known, &text, from, to)
}

pub fn suggest(app: &Shared, word: String) -> Vec<String> {
    spell::suggest(&app.spell, &word)
}

/// A word the play keeps as spelt right, in its `.dactile.toml`.
pub fn learn(app: &Shared, word: String) -> Res<()> {
    let mut a = app.lock().unwrap();
    let path = a.open.as_ref().ok_or("no play open")?.path.clone();
    let mut side = dactile::sidecar::Sidecar::load(&path).map_err(err)?;
    side.add_word(&word);
    side.save(&path).map_err(err)?;
    a.known.set_words(&side.words);
    Ok(())
}

/// The menu after a backslash (or a `#`, with `hash`).
pub fn complete(
    _app: &Shared,
    text: String,
    at: usize,
    query: String,
    hash: bool,
) -> Vec<complete::Item> {
    complete::items(&text, at, &query, hash)
}

pub fn heading(
    _app: &Shared,
    text: String,
    at: usize,
    act: bool,
    place: String,
    time: String,
) -> Change {
    complete::heading(&text, at, act, &place, &time)
}

pub fn fields(_app: &Shared, text: String) -> complete::Fields {
    complete::fields(&text)
}

pub fn verbs(_app: &Shared, text: String) -> Vec<String> {
    complete::verbs(&text)
}

pub fn cast(_app: &Shared, text: String, by_lines: bool) -> Vec<people::Row> {
    people::list(&text, by_lines)
}

pub fn suggest_alias(_app: &Shared, text: String, name: String) -> String {
    people::suggest_alias(&text, &name)
}

pub fn character_add(
    _app: &Shared,
    text: String,
    form: people::Form,
    at: Option<usize>,
) -> Res<people::Made> {
    people::add(&text, &form, at)
}

pub fn renames(_app: &Shared, text: String, old: String, new: String) -> usize {
    people::renames(&text, &old, &new)
}

/// A character changed; its cards in `.dactile.toml` follow a new alias.
pub fn character_edit(
    app: &Shared,
    text: String,
    old: String,
    form: people::Form,
) -> Res<people::Made> {
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

pub fn character_remove(_app: &Shared, text: String, alias: String) -> Res<people::Made> {
    people::remove(&text, &alias)
}

pub fn character_merge(
    _app: &Shared,
    text: String,
    from: String,
    into: String,
) -> Res<people::Made> {
    people::merge(&text, &from, &into)
}

pub fn cast_others(_app: &Shared, text: String) -> String {
    people::others(&text)
}

pub fn cast_others_set(_app: &Shared, text: String, words: String) -> Option<people::Made> {
    people::set_others(&text, &words)
}

pub fn character_shift(
    _app: &Shared,
    text: String,
    alias: String,
    up: bool,
) -> Option<people::Made> {
    people::shift(&text, &alias, up)
}

fn sidecar(app: &Shared) -> Res<(PathBuf, dactile::sidecar::Sidecar)> {
    let a = app.lock().unwrap();
    let path = a.open.as_ref().ok_or("no play open")?.path.clone();
    let side = dactile::sidecar::Sidecar::load(&path).map_err(err)?;
    Ok((path, side))
}

pub fn summaries(app: &Shared, text: String) -> Res<Vec<summary::Summary>> {
    Ok(summary::list(&text, &sidecar(app)?.1))
}

pub fn summary_here(app: &Shared, text: String, at: usize) -> Res<summary::Here> {
    summary::here(&text, at, &sidecar(app)?.1)
}

pub fn summary_set(app: &Shared, text: String, at: usize, summary: String) -> Res<Option<Change>> {
    let (path, _) = sidecar(app)?;
    summary::set(&text, at, &summary, &path).map_err(err)
}

pub fn scene_split(app: &Shared, text: String, at: usize) -> Res<people::Made> {
    let mut a = app.lock().unwrap();
    let path = a.open.as_ref().ok_or("no play open")?.path.clone();
    scenes::split(&text, at, &path, &mut a.kept)
}

/// The play as it stands on the page, as a Word document beside it.
pub fn docx_export(_app: &Shared, path: String, text: String) -> Res<String> {
    let out =
        dactile::word::export(std::path::Path::new(&path), &text).map_err(|e| e.to_string())?;
    Ok(out.display().to_string())
}

/// The play and its open notes: the play as it stands, and its notes, or
/// what says there are none.
fn with_notes(app: &Shared) -> Res<(PathBuf, Notes)> {
    let mut a = app.lock().unwrap();
    a.fresh_notes();
    let path = a.open.as_ref().ok_or("no play open")?.path.clone();
    let open = a
        .notes
        .list
        .iter()
        .any(|n| n.state == dactile::notes::State::Open && !n.archived);
    if !open {
        return Err(dactile::lang::l("Açık not yok.", "No open notes.").into());
    }
    Ok((path, a.notes.clone()))
}

/// The play with its open notes in the margin and listed at the end, a PDF
/// of its own beside it: where it is.
pub fn notes_pdf(app: &Shared, text: String) -> Res<String> {
    let (path, notes) = with_notes(app)?;
    let doc = dactile::render::Doc::new(&text);
    let tex = dactile::notes::copy_tex(&text, &doc, &notes, false);
    let aux = dactile::build::casts(&doc.outline);
    let out = dactile::notes::copy_path(&path);
    dactile::build::build_aside(&path, &tex, &aux, &out)
        .map(|b| b.pdf.display().to_string())
        .map_err(|e| e.message)
}

/// The play with its open notes as Word's comments, beside it: where it is.
pub fn notes_docx(app: &Shared, text: String) -> Res<String> {
    let (path, notes) = with_notes(app)?;
    dactile::word::export_with_notes(&path, &text, &notes)
        .map(|p| p.display().to_string())
        .map_err(err)
}

pub fn cover_read(_app: &Shared, text: String) -> cover::CoverForm {
    cover::read(&text)
}

pub fn cover_write(_app: &Shared, text: String, form: cover::CoverForm) -> people::Made {
    cover::write(&text, &form)
}

pub fn play_font_size_write(_app: &Shared, text: String, size: String) -> people::Made {
    cover::font_size(&text, &size)
}

pub fn play_font_write(_app: &Shared, text: String, font: String) -> people::Made {
    cover::font(&text, &font)
}

pub fn scene_align_next(_app: &Shared, text: String) -> Res<people::Made> {
    scenes::next_align(&text)
}

pub fn scene_join(app: &Shared, text: String, at: usize) -> Res<people::Made> {
    let mut a = app.lock().unwrap();
    let path = a.open.as_ref().ok_or("no play open")?.path.clone();
    scenes::join(&text, at, &path, &mut a.kept)
}

/// An error in the window, written where `npm run tauri dev` shows it.
pub fn casting_roles(app: &Shared, text: String) -> Res<casting::Roles> {
    let default = app.lock().unwrap().cfg.wpm;
    let side = sidecar(app)?.1;
    Ok(casting::roles(&text, &side, casting::wpm(&side, default)))
}

pub fn casting_edit(app: &Shared, edit: casting::Edit) -> Res<()> {
    let (path, mut side) = sidecar(app)?;
    casting::edit(&mut side, edit)?;
    side.save(&path).map_err(err)
}

pub fn casting_csv(app: &Shared, text: String) -> Res<String> {
    let (path, side) = sidecar(app)?;
    let out = casting::table_csv(&text, &side, &path).map_err(err)?;
    Ok(out.display().to_string())
}

pub fn casting_pdf(app: &Shared, text: String) -> Res<String> {
    let (path, side) = sidecar(app)?;
    casting::table_pdf(&text, &side, &path).map(|p| p.display().to_string())
}

pub fn casting_copies(app: &Shared, text: String, kind: casting::Kind) -> Res<(String, usize)> {
    let default = app.lock().unwrap().cfg.wpm;
    let (path, side) = sidecar(app)?;
    let wpm = casting::wpm(&side, default);
    casting::copies(&text, &side, &path, wpm, kind).map(|(dir, n)| (dir.display().to_string(), n))
}

pub fn memo(app: &Shared, text: String, role: String, at: usize) -> Res<casting::Memo> {
    Ok(casting::memo(&text, &role, at, &sidecar(app)?.1))
}

pub fn import_stop(_app: &Shared) {
    importing::STOP.store(true, std::sync::atomic::Ordering::Relaxed);
}

pub fn import_edit(app: &Shared, edit: importing::Edit) -> Res<importing::Shown> {
    let mut a = app.lock().unwrap();
    let (file, draft) = a.importing.as_mut().ok_or("no import open")?;
    importing::edit(draft, edit)?;
    Ok(importing::shown(file, draft))
}

pub fn import_doc(app: &Shared) -> Res<Vec<importing::Para>> {
    let a = app.lock().unwrap();
    let (_, draft) = a.importing.as_ref().ok_or("no import open")?;
    Ok(importing::document(draft))
}

/// The play written: where it is, to open.
pub fn import_write(app: &Shared) -> Res<String> {
    let mut a = app.lock().unwrap();
    let (file, draft) = a.importing.as_ref().ok_or("no import open")?;
    let path = importing::write(file, draft, &a.cfg, None).map_err(err)?;
    a.importing = None;
    Ok(path.display().to_string())
}

pub fn import_volume(app: &Shared) -> Res<Vec<importing::PlayLine>> {
    let a = app.lock().unwrap();
    let (_, draft) = a.importing.as_ref().ok_or("no import open")?;
    Ok(importing::volume(draft))
}

/// The chosen plays of the volume written; the import stays open for the
/// list of what became of them.
pub fn import_write_all(
    app: &Shared,
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
pub fn book_pdf(app: &Shared, path: String) -> Res<String> {
    let setting = app.lock().unwrap().cfg.lang.clone();
    dactile::book::pdf(std::path::Path::new(&path), &setting)
        .map(|p| p.display().to_string())
        .map_err(err)
}

/// These plays as the parts of a new book, in this order: where it is.
pub fn book_gather(app: &Shared, paths: Vec<String>, title: String) -> Res<String> {
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

pub fn import_drop(app: &Shared) {
    app.lock().unwrap().importing = None;
}

pub fn fork_start(
    _app: &Shared,
    path: String,
    draft: Option<String>,
    today: String,
) -> plays::Start {
    plays::fork_start(std::path::Path::new(&path), draft.as_deref(), &today)
}

pub fn fork_file(_app: &Shared, path: String, draft: String) -> String {
    dactile::library::fork_name(std::path::Path::new(&path), &draft)
}

pub fn copy_start(_app: &Shared, title: String) -> plays::Start {
    plays::copy_start(&title)
}

pub fn title_file(_app: &Shared, title: String) -> String {
    dactile::library::file_name(&title)
}

pub fn rename_start(_app: &Shared, path: String) -> plays::Start {
    plays::rename_start(std::path::Path::new(&path))
}

pub fn play_make(
    app: &Shared,
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

pub fn play_rename(app: &Shared, path: String, title: String, file: String) -> Res<String> {
    let cfg = &mut app.lock().unwrap().cfg;
    plays::rename(std::path::Path::new(&path), &title, &file, cfg).map(|p| p.display().to_string())
}

pub fn play_drafts(app: &Shared, path: String) -> usize {
    plays::drafts(std::path::Path::new(&path), &app.lock().unwrap().cfg)
}

pub fn play_throw(app: &Shared, path: String) -> Res<()> {
    let mut a = app.lock().unwrap();
    let moved = plays::throw(std::path::Path::new(&path), &mut a.cfg)?;
    a.thrown = Some(moved);
    Ok(())
}

/// Brings back the play last thrown away: where it is.
pub fn play_restore(app: &Shared) -> Res<String> {
    let mut a = app.lock().unwrap();
    let moved = a.thrown.take().ok_or("nothing thrown away")?;
    plays::restore(&moved, &mut a.cfg)?;
    Ok(moved[0].from.display().to_string())
}

/// The open play's own settings.
#[derive(serde::Serialize)]
pub struct PlaySettings {
    /// Its pace, if it has one of its own.
    wpm: Option<u32>,
    default_wpm: u32,
    /// The words its spell checker accepts.
    words: Vec<String>,
    /// The play it is a draft of.
    parent: Option<String>,
}

pub fn play_settings(app: &Shared) -> Res<PlaySettings> {
    let default_wpm = app.lock().unwrap().cfg.wpm;
    let side = sidecar(app)?.1;
    Ok(PlaySettings {
        wpm: side.wpm,
        default_wpm,
        words: side.words,
        parent: side.parent,
    })
}

/// Its pace: none, the default again.
pub fn play_wpm(app: &Shared, wpm: Option<u32>) -> Res<()> {
    let (path, mut side) = sidecar(app)?;
    match wpm {
        Some(wpm) => casting::edit(&mut side, casting::Edit::Wpm { wpm })?,
        None => side.wpm = None,
    }
    side.save(&path).map_err(err)
}

/// A word out of the play's dictionary.
pub fn play_unlearn(app: &Shared, word: String) -> Res<()> {
    let (path, mut side) = sidecar(app)?;
    side.words.retain(|w| *w != word);
    side.save(&path).map_err(err)?;
    app.lock().unwrap().known.set_words(&side.words);
    Ok(())
}

/// Where a play's things are. It reads the plays beside it.
pub fn files(_app: &Shared, path: String) -> Res<plays::Files> {
    Ok(plays::files(std::path::Path::new(&path)))
}

/// Where a line of the open play is in its PDF. Runs a program.
pub fn sync_view(app: &Shared, line: usize, column: usize) -> Res<Vec<dactile::synctex::Place>> {
    let path = open_path(app)?;
    dactile::synctex::view(&path, line, column).map_err(err)
}

/// The open play's line behind a point of its PDF, in points from the page's
/// top left.
pub fn sync_edit(app: &Shared, pdf: String, page: usize, x: f32, y: f32) -> Res<usize> {
    let path = open_path(app)?;
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
}

fn open_path(app: &Shared) -> Res<PathBuf> {
    let a = app.lock().unwrap();
    Ok(a.open.as_ref().ok_or("no play open")?.path.clone())
}

/// The coloured keyboard, in this app's keys and the chosen layout.
pub fn keys_view(_app: &Shared) -> KeysView {
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
pub struct KeysView {
    rows: Vec<Vec<&'static str>>,
    binds: Vec<KeyView>,
    cats: Vec<(&'static str, &'static str)>,
}

#[derive(serde::Serialize)]
pub struct KeyView {
    cap: &'static str,
    ctrl: bool,
    alt: bool,
    cat: &'static str,
    combo: String,
    what: &'static str,
}

/// What the commands do, group by group.
pub fn guide_view(_app: &Shared) -> Vec<GuideGroup> {
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
pub struct GuideGroup {
    name: &'static str,
    entries: Vec<GuideEntry>,
}

#[derive(serde::Serialize)]
pub struct GuideEntry {
    name: &'static str,
    what: &'static str,
    example: &'static str,
}

/// A command's answer as JSON, whether it can fail or not.
struct Answer<T>(T);

trait Fallible {
    fn json(&self) -> Res<serde_json::Value>;
}

impl<T: serde::Serialize> Fallible for Answer<Res<T>> {
    fn json(&self) -> Res<serde_json::Value> {
        match &self.0 {
            Ok(v) => serde_json::to_value(v).map_err(|e| e.to_string()),
            Err(e) => Err(e.clone()),
        }
    }
}

trait Plain {
    fn json(&self) -> Res<serde_json::Value>;
}

impl<T: serde::Serialize> Plain for &Answer<T> {
    fn json(&self) -> Res<serde_json::Value> {
        serde_json::to_value(&self.0).map_err(|e| e.to_string())
    }
}

macro_rules! commands {
    ($( $name:ident ( $($arg:ident : $ty:ty),* ); )*) => {
        /// The names `dispatch` answers to.
        pub const NAMES: &[&str] = &[$(stringify!($name)),*];

        /// Runs the command `name` with its arguments, a JSON object.
        pub fn dispatch(app: &Ctx, name: &str, args: serde_json::Value) -> Res<serde_json::Value> {
            match name {
                $(stringify!($name) => {
                    #[derive(serde::Deserialize)]
                    #[serde(rename_all = "camelCase")]
                    struct Args { $($arg: $ty,)* }
                    let Args { $($arg,)* } = serde_json::from_value(args)
                        .map_err(|e| format!("{}: {e}", stringify!($name)))?;
                    (&Answer($name(app, $($arg),*))).json()
                })*
                _ => Err(format!("no command {name}")),
            }
        }
    };
}

// Every command, as the window names it: its arguments come as one JSON
// object in the window's camelCase, its answer goes back as JSON.
commands! {
        home();
        play_imported(path: String, on: bool);
        settings();
        set_setting(key: String, value: serde_json::Value);
        preview(path: String);
        create(template: usize, title: String, details: std::collections::BTreeMap<String, String>);
        example();
        open(path: String);
        close();
        save(text: String, force: bool);
        save_template(text: String, name: String);
        view(text: String);
        board(text: String);
        relocate(text: String, card: usize, to: usize);
        build();
        notes_list(text: String);
        note_add(text: String, at: usize, author: String, body: String);
        note_set(text: String, i: usize, state: String);
        note_reply(text: String, i: usize, author: String, reply: String);
        notes_paste(reply: String, author: String);
        note_fix(text: String, i: usize);
        note_fix_many(text: String, which: Vec<usize>);
        note_authors();
        misspelt(text: String, from: usize, to: usize);
        suggest(word: String);
        learn(word: String);
        complete(text: String, at: usize, query: String, hash: bool);
        heading(text: String, at: usize, act: bool, place: String, time: String);
        fields(text: String);
        verbs(text: String);
        cast(text: String, by_lines: bool);
        suggest_alias(text: String, name: String);
        character_add(text: String, form: people::Form, at: Option<usize>);
        renames(text: String, old: String, new: String);
        character_edit(text: String, old: String, form: people::Form);
        character_remove(text: String, alias: String);
        character_merge(text: String, from: String, into: String);
        cast_others(text: String);
        cast_others_set(text: String, words: String);
        character_shift(text: String, alias: String, up: bool);
        summaries(text: String);
        summary_here(text: String, at: usize);
        summary_set(text: String, at: usize, summary: String);
        scene_split(text: String, at: usize);
        docx_export(path: String, text: String);
        notes_pdf(text: String);
        notes_docx(text: String);
        cover_read(text: String);
        cover_write(text: String, form: cover::CoverForm);
        play_font_size_write(text: String, size: String);
        play_font_write(text: String, font: String);
        scene_align_next(text: String);
        scene_join(text: String, at: usize);
        casting_roles(text: String);
        casting_edit(edit: casting::Edit);
        casting_csv(text: String);
        casting_pdf(text: String);
        casting_copies(text: String, kind: casting::Kind);
        memo(text: String, role: String, at: usize);
        import_stop();
        import_edit(edit: importing::Edit);
        import_doc();
        import_write();
        import_volume();
        import_write_all(picked: Vec<usize>, book: Option<importing::Book>);
        book_pdf(path: String);
        book_gather(paths: Vec<String>, title: String);
        import_drop();
        fork_start(path: String, draft: Option<String>, today: String);
        fork_file(path: String, draft: String);
        copy_start(title: String);
        title_file(title: String);
        rename_start(path: String);
        play_make(path: String, title: Option<String>, draft: Option<String>, file: String);
        play_rename(path: String, title: String, file: String);
        play_drafts(path: String);
        play_throw(path: String);
        play_restore();
        play_settings();
        play_wpm(wpm: Option<u32>);
        play_unlearn(word: String);
        files(path: String);
        sync_view(line: usize, column: usize);
        sync_edit(pdf: String, page: usize, x: f32, y: f32);
        keys_view();
        guide_view();
}
