//! Importing on the home screen: the desktop's file chooser (or a path
//! typed where there is none), the reading in the background, and a
//! preview of the names found to put right before the play is written.

mod mark;
#[cfg(test)]
mod tests;

use super::popup::{self, Field, Form, FormKind, Outcome};
use super::theme::{BEIGE, CLARET, FAINT, FG, MUTED, WARN};
use dactile::config::Config;
use dactile::import::ocr::{self, Job};
use dactile::import::pdf::NoText;
use dactile::import::shape::{Draft, Summary, shape};
use dactile::lang;
use mark::Marking;
use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};

pub enum Importing {
    Picking {
        rx: Receiver<Option<PathBuf>>,
        since: Instant,
        /// What to ask if no chooser comes up.
        ask: Option<Box<Form>>,
        /// What is chosen is opened as it is, not read as a document.
        open: bool,
    },
    Reading {
        rx: Receiver<Result<Read, String>>,
        file: PathBuf,
        stop: Arc<AtomicBool>,
        /// Pages read from their images, and of how many: none, it isn't
        /// reading them.
        pages: Arc<(AtomicUsize, AtomicUsize)>,
        /// Reading again: what to go back to if that comes to nothing.
        back: Option<Box<Preview>>,
    },
    Preview(Box<Preview>),
    /// For a moment, between a preview and its reading again.
    Between,
}

/// What the reading brings back.
pub struct Read {
    draft: Draft,
    /// A PDF that can be read again from its page images.
    ocr: bool,
}

#[derive(Clone, Copy, PartialEq)]
enum View {
    /// The plays of a volume of several.
    Volume,
    People,
    Document,
    Play,
    /// The cast a person a line, to put right.
    Cast,
}

pub struct Preview {
    file: PathBuf,
    draft: Draft,
    /// The volume's plays, a line each, and those to write.
    plays: Vec<Summary>,
    chosen: Vec<bool>,
    /// The line on in the volume's list.
    play_sel: usize,
    /// The plays go under one book; what stands before them only if asked.
    as_book: bool,
    front: bool,
    /// The line on in the cast list.
    cast_sel: usize,
    ocr: bool,
    /// `o` pressed once: it asks before the marks start over.
    again: bool,
    view: View,
    doc: Marking,
    sel: usize,
    /// `m` pressed on this row: the next ⏎ merges it into the chosen one.
    merging: Option<usize>,
    form: Option<Form>,
    error: Option<String>,
}

/// What the home screen does next.
pub enum Step {
    Stay,
    /// Gone, with a message.
    Done(Option<(String, bool)>),
    /// No chooser came up: ask for a path.
    Ask(Box<Form>),
    /// Written here: open it.
    Open(PathBuf),
    /// Read the document again from its page images.
    Again,
}

/// A desktop to show a file chooser on (through its portal), not a
/// terminal over SSH.
pub fn can_choose() -> bool {
    let bus = std::env::var_os("DBUS_SESSION_BUS_ADDRESS").is_some()
        || std::env::var_os("XDG_RUNTIME_DIR").is_some_and(|d| Path::new(&d).join("bus").exists());
    let screen =
        std::env::var_os("WAYLAND_DISPLAY").is_some() || std::env::var_os("DISPLAY").is_some();
    bus && screen
}

/// The desktop's chooser for a document, in the background.
pub fn pick() -> Importing {
    let dir = dirs::download_dir()
        .filter(|d| d.is_dir())
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("/"));
    let dialog = rfd::FileDialog::new()
        .set_title(lang::l("İçe aktarılacak oyun", "A play to import"))
        .add_filter(
            lang::l("Oyun belgeleri", "Play documents"),
            &dactile::import::EXTENSIONS,
        )
        .set_directory(dir);
    choosing(dialog, path_form(), false)
}

/// The chooser for a `.tex` to open, `form` asking for it where none comes.
pub fn pick_tex(dir: PathBuf, form: Form) -> Importing {
    let dialog = rfd::FileDialog::new()
        .set_title(lang::l("Açılacak oyun", "A play to open"))
        .add_filter(lang::l("Oyunlar", "Plays"), &["tex"])
        .set_directory(dir);
    choosing(dialog, form, true)
}

fn choosing(dialog: rfd::FileDialog, form: Form, open: bool) -> Importing {
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        let _ = tx.send(dialog.pick_file());
    });
    Importing::Picking {
        rx,
        since: Instant::now(),
        ask: Some(Box::new(form)),
        open,
    }
}

/// Reading and shaping `file`, in the background (a PDF takes a while).
pub fn read(file: PathBuf, lang: lang::Lang) -> Importing {
    reading(file, lang, false, None)
}

fn reading(file: PathBuf, lang: lang::Lang, force: bool, back: Option<Box<Preview>>) -> Importing {
    let (tx, rx) = channel();
    let stop = Arc::new(AtomicBool::new(false));
    let pages = Arc::new((AtomicUsize::new(0), AtomicUsize::new(0)));
    let (f, s, p) = (file.clone(), stop.clone(), pages.clone());
    std::thread::spawn(move || {
        let progress = |done: usize, total: usize| {
            p.0.store(done, Ordering::Relaxed);
            p.1.store(total, Ordering::Relaxed);
        };
        let job = Job {
            progress: &progress,
            stop: &s,
        };
        let _ = tx.send(shaped(&f, lang, force, &job));
    });
    Importing::Reading {
        rx,
        file,
        stop,
        pages,
        back,
    }
}

fn shaped(file: &Path, lang: lang::Lang, force: bool, job: &Job) -> Result<Read, String> {
    let src = dactile::import::read_as(file, force, job);
    if job.stop.load(Ordering::Relaxed) {
        return Err(lang::l("Okuma durduruldu.", "The reading was stopped.").into());
    }
    let src = src.map_err(|e| {
        if e.downcast_ref::<NoText>().is_some() {
            lang::f(
                "{} (OCR araçları bu makinede yok: tesseract, poppler)",
                "{} (the OCR tools aren't on this machine: tesseract, poppler)",
                &[&format!("{e:#}")],
            )
        } else {
            format!("{e:#}")
        }
    })?;
    let pdf = dactile::import::Format::of(file) == Some(dactile::import::Format::Pdf);
    Ok(Read {
        draft: shape(&src, lang),
        ocr: pdf && ocr::ready(),
    })
}

/// The form asking for a path, where no chooser is.
pub fn path_form() -> Form {
    let dir = dirs::download_dir()
        .filter(|d| d.is_dir())
        .or_else(dirs::home_dir)
        .map(|d| format!("{}/", dactile::config::tilde(&d)))
        .unwrap_or_default();
    Form {
        kind: FormKind::ImportPath,
        title: lang::l(
            "içe aktar (.docx .odt .pdf .md .txt)",
            "import (.docx .odt .pdf .md .txt)",
        )
        .into(),
        fields: vec![Field::text("Dosya", &dir, Vec::new())],
        focus: 0,
        at: 0..0,
        error: None,
        confirmed: false,
    }
}

impl Importing {
    /// Whether it waits on something: the screen is drawn more often.
    pub fn busy(&self) -> bool {
        !matches!(self, Importing::Preview(_) | Importing::Between)
    }

    /// The chooser's or the reader's answer, when it comes.
    pub fn tick(&mut self, lang: lang::Lang) -> Step {
        match self {
            Importing::Picking {
                rx,
                since,
                ask,
                open,
            } => match rx.try_recv() {
                Ok(Some(path)) if *open => Step::Open(path),
                Ok(Some(path)) => {
                    *self = read(path, lang);
                    Step::Stay
                }
                // Back at once with nothing: no chooser came up.
                Ok(None) if since.elapsed() < Duration::from_millis(400) => {
                    ask.take().map_or(Step::Done(None), Step::Ask)
                }
                Ok(None) => Step::Done(None),
                Err(std::sync::mpsc::TryRecvError::Empty) => Step::Stay,
                Err(_) => ask.take().map_or(Step::Done(None), Step::Ask),
            },
            Importing::Reading {
                rx,
                file,
                stop,
                back,
                ..
            } => match rx.try_recv() {
                Ok(Ok(read)) => {
                    *self = Importing::Preview(Box::new(Preview::new(file.clone(), read)));
                    Step::Stay
                }
                Ok(Err(e)) => match back.take() {
                    Some(mut p) => {
                        p.error = Some(e);
                        *self = Importing::Preview(p);
                        Step::Stay
                    }
                    None => Step::Done(Some((e, !stop.load(Ordering::Relaxed)))),
                },
                Err(std::sync::mpsc::TryRecvError::Empty) => Step::Stay,
                Err(_) => Step::Done(None),
            },
            Importing::Preview(_) | Importing::Between => Step::Stay,
        }
    }

    pub fn key(&mut self, k: KeyEvent, cfg: &Config) -> Step {
        match self {
            Importing::Preview(p) => match p.key(k, cfg) {
                Step::Again => {
                    let old = std::mem::replace(self, Importing::Between);
                    if let Importing::Preview(p) = old {
                        let (file, lang) = (p.file.clone(), p.draft.lang);
                        *self = reading(file, lang, true, Some(p));
                    }
                    Step::Stay
                }
                step => step,
            },
            // The pages under way end first: another reading would share
            // their scratch folder.
            Importing::Reading { stop, pages, .. } if k.code == KeyCode::Esc => {
                stop.store(true, Ordering::Relaxed);
                if pages.1.load(Ordering::Relaxed) > 0 {
                    Step::Stay
                } else {
                    Step::Done(None)
                }
            }
            // A chooser left open closes on its own.
            Importing::Picking { .. } if k.code == KeyCode::Esc => Step::Done(None),
            _ => Step::Stay,
        }
    }

    pub fn draw(&self, f: &mut Frame, area: Rect) {
        match self {
            Importing::Picking { .. } => note(
                f,
                area,
                lang::l(
                    "Dosya seçiciden bir belge seç…  (Esc vazgeçer)",
                    "Choose a document in the file chooser…  (Esc gives up)",
                ),
            ),
            Importing::Reading {
                file, stop, pages, ..
            } => {
                let done = pages.0.load(Ordering::Relaxed);
                let total = pages.1.load(Ordering::Relaxed);
                let s = if stop.load(Ordering::Relaxed) {
                    lang::l(
                        "Durduruluyor: okunan sayfalar bitiyor…",
                        "Stopping: the pages under way are ending…",
                    )
                    .to_string()
                } else if total > 0 {
                    lang::f(
                        "OCR {}/{} sayfa  (Esc durdurur)",
                        "OCR {}/{} pages  (Esc stops)",
                        &[&done, &total],
                    )
                } else {
                    lang::f(
                        "{} okunuyor…  (Esc vazgeçer)",
                        "Reading {}…  (Esc gives up)",
                        &[&file.file_name().unwrap_or_default().to_string_lossy()],
                    )
                };
                note(f, area, &s);
            }
            Importing::Preview(p) => p.draw(f, area),
            Importing::Between => {}
        }
    }
}

fn note(f: &mut Frame, area: Rect, s: &str) {
    let w = (s.chars().count() as u16 + 6).min(area.width);
    let r = Rect {
        x: area.x + (area.width - w) / 2,
        y: area.y + area.height / 3,
        width: w,
        height: 3,
    };
    popup::float(f, r);
    f.render_widget(popup::panel(lang::l("içe aktar", "import")), r);
    f.render_widget(
        Paragraph::new(Span::styled(s.to_string(), Style::default().fg(FG))),
        popup::inner(r),
    );
}

impl Preview {
    fn new(file: PathBuf, read: Read) -> Preview {
        let plays = read.draft.summaries();
        Preview {
            file,
            ocr: read.ocr,
            again: false,
            view: if plays.len() > 1 {
                View::Volume
            } else {
                View::People
            },
            doc: Marking::new(&read.draft),
            draft: read.draft,
            chosen: vec![true; plays.len()],
            plays,
            play_sel: 0,
            as_book: true,
            front: false,
            cast_sel: 0,
            sel: 0,
            merging: None,
            form: None,
            error: None,
        }
    }

    /// Another play of the volume. What was put right stays with its play,
    /// for when `v` comes back to it.
    fn next_play(&mut self) {
        let d = &self.draft;
        let at = d.play.map_or(0, |p| p + 1) % d.source.plays.len();
        self.open_play(at);
    }

    fn open_play(&mut self, at: usize) {
        self.draft.go_to_play(at);
        self.doc.refresh(&self.draft);
        self.doc.go_to_play(&self.draft);
        self.sel = 0;
        self.merging = None;
    }

    fn cast_form(&mut self, at: Option<usize>, line: &str) {
        self.form = Some(Form {
            kind: FormKind::ImportCastLine(at),
            title: lang::l(
                "AD | not · LİSTEDEKİ = REPLİKTEKİ | not · + toplu",
                "NAME | note · ON THE LIST = OVER THE LINES | note · + a group",
            )
            .into(),
            fields: vec![Field::text("Satır", line, Vec::new())],
            focus: 0,
            at: 0..0,
            error: None,
            confirmed: false,
        });
    }

    fn set_cast(&mut self, text: &str) {
        self.draft.set_cast(text);
        self.doc.refresh(&self.draft);
        let n = self.draft.cast_text().lines().count();
        self.cast_sel = self.cast_sel.min(n.saturating_sub(1));
    }

    /// The cast list, under the head.
    fn cast_lines(&self, room: usize) -> Vec<Line<'static>> {
        let text = self.draft.cast_text();
        let first = self.cast_sel.saturating_sub(room.saturating_sub(1));
        let (silent, missing) = self.draft.cast_check();
        let mut out: Vec<Line> = text
            .lines()
            .enumerate()
            .skip(first)
            .take(room.saturating_sub(2).max(1))
            .map(|(i, l)| {
                let style = if i == self.cast_sel {
                    Style::default().fg(BEIGE).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(FG)
                };
                let mark = if i == self.cast_sel { ">" } else { " " };
                Line::from(Span::styled(format!("{mark} {l}"), style))
            })
            .collect();
        let say = |what: &str, names: &[String]| {
            Line::from(Span::styled(
                format!("{what}: {}", names.join(", ")),
                Style::default().fg(MUTED),
            ))
        };
        if !silent.is_empty() {
            out.push(say(lang::l("Hiç konuşmuyor", "Never speaks"), &silent));
        }
        if !missing.is_empty() {
            out.push(say(lang::l("Listede yok", "Not listed"), &missing));
        }
        out
    }

    /// The volume changed: its list, and the document, again.
    fn volume_changed(&mut self) {
        let was = self.chosen.len();
        self.plays = self.draft.summaries();
        if self.plays.len() != was {
            self.chosen = vec![true; self.plays.len()];
        }
        self.play_sel = self.play_sel.min(self.plays.len().saturating_sub(1));
        self.doc.refresh(&self.draft);
        self.doc.go_to_play(&self.draft);
        self.sel = 0;
        self.merging = None;
        if self.plays.len() < 2 && self.view == View::Volume {
            self.view = View::People;
        }
    }

    /// Marking reads the play again, and the list of people can shrink.
    fn people_changed(&mut self) {
        self.sel = self.sel.min(self.draft.people.len().saturating_sub(1));
        self.merging = None;
    }

    fn next_view(&mut self, on: bool) {
        let mut views = vec![View::People, View::Document, View::Play, View::Cast];
        if self.draft.source.plays.len() > 1 {
            views.insert(0, View::Volume);
        }
        let at = views.iter().position(|v| *v == self.view).unwrap_or(0);
        let by = if on { 1 } else { views.len() - 1 };
        self.view = views[(at + by) % views.len()];
        if self.view == View::Volume {
            self.plays = self.draft.summaries();
        }
    }

    fn key(&mut self, k: KeyEvent, cfg: &Config) -> Step {
        self.error = None;
        let again = std::mem::take(&mut self.again);
        if let Some(form) = &mut self.form {
            match popup::edit(form, k) {
                Outcome::Stay => {}
                Outcome::Cancel => self.form = None,
                Outcome::Submit => {
                    let form = self.form.take().unwrap_or_else(path_form);
                    if let Err(e) = self.submit(&form) {
                        let mut form = form;
                        form.error = Some(e);
                        self.form = Some(form);
                    }
                }
            }
            return Step::Stay;
        }
        if self.view == View::Document && self.doc.picking() {
            if self.doc.doc_key(k, &mut self.draft) {
                self.people_changed();
            }
            return Step::Stay;
        }
        let n = self.draft.people.len();
        match k.code {
            KeyCode::Esc if self.merging.is_some() => self.merging = None,
            KeyCode::Esc if self.view == View::Document && self.doc.ranging() => {
                self.doc.stop_ranging()
            }
            KeyCode::Tab if self.merging.is_none() => self.next_view(true),
            KeyCode::BackTab if self.merging.is_none() => self.next_view(false),
            KeyCode::Esc => {
                return Step::Done(Some((
                    lang::l("İçe aktarma bırakıldı.", "Import dropped.").into(),
                    false,
                )));
            }
            KeyCode::Char('o') if self.ocr && again => return Step::Again,
            KeyCode::Char('o') if self.ocr => {
                self.again = true;
                self.error = Some(
                    lang::l(
                        "Sayfa görüntülerinden baştan okunur, düzelttiklerin sıfırlanır: yine o",
                        "Reads the page images afresh, what you put right starts over: o again",
                    )
                    .into(),
                );
            }
            KeyCode::Char('v') if self.draft.source.plays.len() > 1 => self.next_play(),
            KeyCode::Up if self.view == View::People => self.sel = self.sel.saturating_sub(1),
            KeyCode::Down if self.view == View::People => {
                self.sel = (self.sel + 1).min(n.saturating_sub(1))
            }
            KeyCode::Up if self.view == View::Volume => {
                self.play_sel = self.play_sel.saturating_sub(1)
            }
            KeyCode::Down if self.view == View::Volume => {
                self.play_sel = (self.play_sel + 1).min(self.plays.len().saturating_sub(1))
            }
            KeyCode::Char(' ') if self.view == View::Volume => {
                if let Some(on) = self.chosen.get_mut(self.play_sel) {
                    *on = !*on;
                }
            }
            KeyCode::Enter if self.view == View::Volume => {
                self.open_play(self.play_sel);
                self.view = View::People;
            }
            KeyCode::Char('w') if self.view == View::Volume => return self.write_chosen(cfg),
            KeyCode::Char('b') if self.view == View::Volume => self.as_book = !self.as_book,
            KeyCode::Char('f') if self.view == View::Volume => self.front = !self.front,
            KeyCode::Char('x') if self.view == View::Volume => {
                self.draft.drop_play(self.play_sel);
                self.volume_changed();
            }
            KeyCode::Up if self.view == View::Cast => {
                self.cast_sel = self.cast_sel.saturating_sub(1)
            }
            KeyCode::Down if self.view == View::Cast => {
                let n = self.draft.cast_text().lines().count();
                self.cast_sel = (self.cast_sel + 1).min(n.saturating_sub(1))
            }
            KeyCode::Enter | KeyCode::Char('e') if self.view == View::Cast => {
                let text = self.draft.cast_text();
                let line = text.lines().nth(self.cast_sel).unwrap_or("").to_string();
                self.cast_form(Some(self.cast_sel), &line);
            }
            KeyCode::Char('n') if self.view == View::Cast => self.cast_form(None, ""),
            KeyCode::Char('x') if self.view == View::Cast => {
                let text = self.draft.cast_text();
                let kept: Vec<&str> = text
                    .lines()
                    .enumerate()
                    .filter(|(i, _)| *i != self.cast_sel)
                    .map(|(_, l)| l)
                    .collect();
                self.set_cast(&kept.join("\n"));
            }
            KeyCode::Char('k') if self.view == View::Document => {
                self.draft.mark_cast(self.doc.at(), self.doc.end());
                self.doc.stop_ranging();
                self.people_changed();
                self.view = View::Cast;
                self.cast_sel = 0;
            }
            KeyCode::Char('y') if self.view == View::Document => {
                let at = self.doc.at();
                let first = self.draft.source.paras[at].text();
                let first = first.lines().next().unwrap_or("").to_string();
                self.form = Some(Form {
                    kind: FormKind::ImportPlay(at),
                    title: lang::l("burada yeni oyun başlıyor", "a new play starts here").into(),
                    fields: vec![Field::text("Ad", &first, Vec::new())],
                    focus: 0,
                    at: 0..0,
                    error: None,
                    confirmed: false,
                });
            }
            KeyCode::Enter if self.merging.is_some() => {
                let from = self.merging.take().unwrap_or(self.sel);
                if from != self.sel {
                    self.draft.merge(from, self.sel);
                    self.doc.refresh(&self.draft);
                }
            }
            KeyCode::Enter => return self.write(cfg),
            KeyCode::Char('t') => {
                self.form = Some(Form {
                    kind: FormKind::ImportInfo,
                    title: lang::l("başlık ve yazar", "title and author").into(),
                    fields: vec![
                        Field::text("Başlık", &self.draft.title, Vec::new()),
                        Field::text("Yazar", &self.draft.author, Vec::new()),
                    ],
                    focus: 0,
                    at: 0..0,
                    error: None,
                    confirmed: false,
                });
            }
            _ if self.view == View::Document => {
                if self.doc.doc_key(k, &mut self.draft) {
                    self.people_changed();
                }
            }
            _ if self.view == View::Play => self.doc.play_key(k),
            _ if matches!(self.view, View::Volume | View::Cast) || n == 0 => {}
            KeyCode::Char('m') => {
                if self.draft.people[self.sel].merged_into.is_some() {
                    self.draft.unmerge(self.sel);
                    self.doc.refresh(&self.draft);
                } else {
                    self.merging = Some(self.sel);
                }
            }
            KeyCode::Char('x') => {
                let p = &mut self.draft.people[self.sel];
                p.not_character = !p.not_character;
                self.doc.refresh(&self.draft);
            }
            KeyCode::Char('e') | KeyCode::Char('a') => {
                let p = &self.draft.people[self.sel];
                self.form = Some(Form {
                    kind: FormKind::ImportPerson(self.sel),
                    title: lang::l("ad ve kısaltma", "name and alias").into(),
                    fields: vec![
                        Field::text("Ad", &p.name, Vec::new()),
                        Field::text("Kısaltma", &p.alias, Vec::new()),
                    ],
                    focus: usize::from(k.code == KeyCode::Char('a')),
                    at: 0..0,
                    error: None,
                    confirmed: false,
                });
            }
            _ => {}
        }
        Step::Stay
    }

    fn submit(&mut self, form: &Form) -> Result<(), String> {
        match form.kind {
            FormKind::ImportPerson(i) => {
                let name = form.get("Ad");
                let alias = form.get("Kısaltma");
                if name.is_empty() {
                    return Err(lang::l("Ad boş olamaz.", "The name can't be empty.").into());
                }
                if alias != self.draft.people[i].alias
                    && let Some(why) = self.draft.alias_problem(i, alias)
                {
                    return Err(why.into());
                }
                let p = &mut self.draft.people[i];
                p.name = dactile::cast::upper_tr(name);
                p.alias = alias.to_string();
                self.doc.refresh(&self.draft);
            }
            FormKind::ImportInfo => {
                self.draft.title = form.get("Başlık").to_string();
                self.draft.author = form.get("Yazar").to_string();
                if let Some(at) = self.draft.play {
                    self.draft.source.plays[at].title = self.draft.title.clone();
                }
            }
            FormKind::ImportCastLine(at) => {
                let text = self.draft.cast_text();
                let mut lines: Vec<&str> = text.lines().collect();
                let line = form.get("Satır");
                match at {
                    Some(i) if i < lines.len() => lines[i] = line,
                    _ => lines.insert((self.cast_sel + 1).min(lines.len()), line),
                }
                self.set_cast(&lines.join("\n"));
            }
            FormKind::ImportPlay(i) => {
                self.draft.add_play(i, form.get("Ad"))?;
                self.volume_changed();
            }
            _ => {}
        }
        Ok(())
    }

    /// Which play of a volume this is, if it is one.
    fn volume_line(&self) -> String {
        let plays = &self.draft.source.plays;
        match self.draft.play {
            Some(at) if plays.len() > 1 => lang::f(
                "Bu ciltte {}/{}: {} · v sonraki oyun",
                "In this volume {}/{}: {} · v next play",
                &[&(at + 1), &plays.len(), &plays[at].title],
            ),
            _ => String::new(),
        }
    }

    /// People, document, play: the one on view lit.
    fn tabs(&self) -> Line<'static> {
        let tab = |view, name: &'static str| {
            let style = if self.view == view {
                Style::default().fg(BEIGE).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(FAINT)
            };
            Span::styled(format!("{name}  "), style)
        };
        let mut tabs = Vec::new();
        if self.draft.source.plays.len() > 1 {
            tabs.push(tab(View::Volume, lang::l("Cilt", "Volume")));
        }
        tabs.extend([
            tab(View::People, lang::l("Kişiler", "People")),
            tab(View::Document, lang::l("Belge", "Document")),
            tab(View::Play, lang::l("Oyun", "Play")),
            tab(View::Cast, lang::l("Kişi listesi", "Cast list")),
            Span::styled("(Tab)", Style::default().fg(FAINT)),
        ]);
        Line::from(tabs)
    }

    /// The play, in the plays folder under its title's file name.
    fn write(&mut self, cfg: &Config) -> Step {
        match self.write_play(&self.draft, cfg) {
            Ok(path) => Step::Open(path),
            Err(e) => {
                self.error = Some(e);
                Step::Stay
            }
        }
    }

    fn write_play(&self, d: &Draft, cfg: &Config) -> Result<PathBuf, String> {
        let stem = self
            .file
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        let title = if d.title.trim().is_empty() {
            stem
        } else {
            d.title.clone()
        };
        let dir = cfg.plays_dir();
        std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        let path = dactile::library::free_path(&dir, &dactile::library::file_name(&title));
        dactile::file::write_atomic(&path, &d.tex()).map_err(|e| format!("{e:#}"))?;
        Ok(path)
    }

    /// The chosen plays of the volume, each under its own name. One that
    /// can't be written doesn't stop the others.
    fn write_chosen(&mut self, cfg: &Config) -> Step {
        let picked: Vec<usize> = (0..self.chosen.len()).filter(|&i| self.chosen[i]).collect();
        if picked.is_empty() {
            self.error = Some(lang::l("Hiç oyun seçilmedi.", "No play is chosen.").into());
            return Step::Stay;
        }
        let plays = self.draft.plays(&picked);
        let root = if self.as_book {
            let title = self
                .file
                .file_stem()
                .map(|s| s.to_string_lossy().replace('_', " "))
                .unwrap_or_default();
            let text = if self.front {
                self.draft.source.front_matter()
            } else {
                String::new()
            };
            let (dir, d) = (cfg.plays_dir(), &self.draft);
            match dactile::book::make(&dir, &title, &d.author, &text, d.lang) {
                Ok(root) => Some(root),
                Err(e) => {
                    self.error = Some(format!("{e:#}"));
                    return Step::Stay;
                }
            }
        } else {
            None
        };
        let mut made = 0;
        let mut failed = Vec::new();
        // The list shows the newest on top: the first play is written last.
        for (at, play) in plays.iter().enumerate().rev() {
            let done = self.write_play(play, cfg).and_then(|path| match &root {
                Some(root) => dactile::book::join(root, &path, at).map_err(|e| format!("{e:#}")),
                None => Ok(()),
            });
            match done {
                Ok(()) => made += 1,
                Err(e) => failed.push(format!("{}: {e}", play.title)),
            }
        }
        if failed.is_empty() {
            return Step::Done(Some((
                lang::f("{} oyun oluşturuldu.", "{} plays made.", &[&made]),
                false,
            )));
        }
        self.error = Some(lang::f(
            "{} oyun oluşturuldu, olmayanlar: {}",
            "{} plays made, not these: {}",
            &[&made, &failed.join("; ")],
        ));
        Step::Stay
    }

    /// The volume's list, under the head.
    fn volume_lines(&self, room: usize) -> Vec<Line<'static>> {
        let first = self.play_sel.saturating_sub(room.saturating_sub(1));
        self.plays
            .iter()
            .enumerate()
            .skip(first)
            .take(room)
            .map(|(i, p)| {
                let on = self.chosen.get(i).copied().unwrap_or(false);
                let pages = match &p.pages {
                    Some((a, b)) if a == b => a.clone(),
                    Some((a, b)) => format!("{a}–{b}"),
                    None => String::new(),
                };
                let guess = if p.sure {
                    ""
                } else {
                    lang::l("  tahmin", "  guess")
                };
                let row = format!(
                    "{} [{}] {:<34} {:>9}  {:>5} {}  {:>3} {}{guess}",
                    if i == self.play_sel { ">" } else { " " },
                    if on { "x" } else { " " },
                    p.title.chars().take(34).collect::<String>(),
                    pages,
                    p.lines,
                    lang::l("replik", "lines"),
                    p.people,
                    lang::l("kişi", "people"),
                );
                let style = if i == self.play_sel {
                    Style::default().fg(BEIGE).add_modifier(Modifier::BOLD)
                } else if on {
                    Style::default().fg(FG)
                } else {
                    Style::default().fg(FAINT)
                };
                Line::from(Span::styled(row, style))
            })
            .collect()
    }

    fn draw(&self, f: &mut Frame, area: Rect) {
        let w = area.width.saturating_sub(6).clamp(40, 110).min(area.width);
        let h = area.height.saturating_sub(4).clamp(12, 34).min(area.height);
        let r = Rect {
            x: area.x + (area.width - w) / 2,
            y: area.y + (area.height - h) / 2,
            width: w,
            height: h,
        };
        popup::float(f, r);
        let name = self.file.file_name().unwrap_or_default().to_string_lossy();
        let title = lang::f("içe aktar — {}", "import — {}", &[&name]);
        f.render_widget(popup::panel(&title), r);
        let inner = popup::inner(r);
        let d = &self.draft;
        let c = d.counts();
        let muted = Style::default().fg(MUTED);
        let mut lines = vec![
            Line::from(vec![
                Span::styled(
                    if d.title.is_empty() {
                        lang::l("(başlıksız)", "(no title)").to_string()
                    } else {
                        d.title.clone()
                    },
                    Style::default().fg(BEIGE).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    if d.author.is_empty() {
                        String::new()
                    } else {
                        format!("  ·  {}", d.author)
                    },
                    Style::default().fg(FG),
                ),
            ]),
            Line::from(Span::styled(
                lang::f(
                    "{} replik · {} yönerge · {} perde · {} sahne · {} düz paragraf",
                    "{} lines · {} directions · {} acts · {} scenes · {} plain paragraphs",
                    &[&c.lines, &c.directions, &c.acts, &c.scenes, &c.plain],
                ),
                muted,
            )),
            Line::from(Span::styled(self.volume_line(), muted)),
            self.tabs(),
        ];
        let head = lines.len() as u16;
        let body = Rect {
            y: inner.y + head,
            height: inner.height.saturating_sub(head + 3),
            ..inner
        };
        let name_w = d
            .people
            .iter()
            .map(|p| p.name.chars().count())
            .max()
            .unwrap_or(4)
            .clamp(4, 24);
        if self.view == View::Volume {
            let room = (inner.height as usize)
                .saturating_sub(lines.len() + 3)
                .max(1);
            lines.extend(self.volume_lines(room));
        }
        if self.view == View::Cast {
            let room = (inner.height as usize)
                .saturating_sub(lines.len() + 3)
                .max(1);
            lines.extend(self.cast_lines(room));
        }
        if self.view == View::People {
            lines.push(Line::from(Span::styled(
                format!(
                    "  {:<name_w$}  {:<9} {:>6}  {}",
                    lang::l("AD", "NAME"),
                    lang::l("KISALTMA", "ALIAS"),
                    lang::l("REPLİK", "LINES"),
                    lang::l("İLK REPLİĞİ", "FIRST LINE"),
                ),
                Style::default().fg(FAINT),
            )));
        }
        let top = lines.len();
        let room = (inner.height as usize).saturating_sub(top + 3).max(1);
        let first = self.sel.saturating_sub(room - 1);
        let listed = if self.view == View::People { room } else { 0 };
        for (i, p) in d.people.iter().enumerate().skip(first).take(listed) {
            let sel = i == self.sel;
            let off = p.not_character || p.merged_into.is_some();
            let style = if off {
                Style::default().fg(FAINT)
            } else {
                Style::default().fg(FG)
            };
            let tail = if let Some(j) = p.merged_into {
                format!("→ {}", d.people[d.owner(j)].name)
            } else if p.not_character {
                lang::l(
                    "karakter değil: metin olarak kalır",
                    "not a character: stays as text",
                )
                .to_string()
            } else {
                p.sample.clone()
            };
            let lines_n = if p.merged_into.is_some() {
                p.lines
            } else {
                d.lines_of(i)
            };
            let fixed = 2 + name_w + 2 + 10 + 6 + 2;
            let mut spans = vec![
                popup::mark(sel || self.merging == Some(i)),
                Span::styled(
                    format!("{:<name_w$}  ", popup::fit(&p.name, name_w)),
                    if off {
                        style
                    } else {
                        Style::default().fg(BEIGE)
                    },
                ),
                Span::styled(format!("\\{:<8} ", p.alias), Style::default().fg(CLARET)),
                Span::styled(format!("{lines_n:>6}  "), style),
                Span::styled(
                    popup::fit(&tail, (inner.width as usize).saturating_sub(fixed)),
                    if off { style } else { muted },
                ),
            ];
            if self.merging == Some(i) {
                for s in &mut spans {
                    s.style = s.style.add_modifier(Modifier::UNDERLINED);
                }
            }
            lines.push(popup::row(spans, sel, inner.width));
        }
        if d.people.is_empty() && self.view == View::People {
            lines.push(Line::from(Span::styled(
                lang::l(
                    "Konuşan kimse bulunamadı: metin düz paragraf olarak gelir.",
                    "No one found speaking: the text comes in as plain paragraphs.",
                ),
                muted,
            )));
        }
        f.render_widget(Paragraph::new(lines), inner);
        match self.view {
            View::Volume | View::People | View::Cast => {}
            View::Document => self.doc.draw_doc(f, body),
            View::Play => self.doc.draw_play(f, body),
        }
        let foot = Rect {
            y: inner.y + inner.height.saturating_sub(3),
            height: 3.min(inner.height),
            ..inner
        };
        let help = if let Some(m) = self.merging {
            lang::f(
                "{} kiminle birleşsin? ↑↓ seç · ⏎ birleştir · Esc vazgeç",
                "Merge {} into whom? ↑↓ choose · ⏎ merge · Esc cancel",
                &[&d.people[m].name],
            )
        } else {
            let mut s = match self.view {
                View::Volume => lang::l(
                    "↑↓ · Space seç · ⏎ oyuna bak · w seçilenleri oluştur · x oyun değil · b kitap · f çevre sayfaları · Esc vazgeç",
                    "↑↓ · Space choose · ⏎ look at the play · w make the chosen · x not a play · b book · f pages around · Esc cancel",
                )
                .to_string()
                    + &lang::f(
                        " ({} · {})",
                        " ({} · {})",
                        &[
                            &if self.as_book { lang::l("kitap: evet", "book: yes") } else { lang::l("kitap: hayır", "book: no") },
                            &if self.front { lang::l("çevre: evet", "around: yes") } else { lang::l("çevre: hayır", "around: no") },
                        ],
                    ),
                View::People => lang::l(
                    "⏎ içe aktar · e ad · a kısaltma · m birleştir/ayır · x karakter değil · t başlık · Esc vazgeç",
                    "⏎ import · e name · a alias · m merge/split · x not a character · t title · Esc cancel",
                )
                .to_string(),
                View::Document => {
                    self.doc.help()
                        + lang::l(
                            " · k kişi sayfası bu · y burada yeni oyun",
                            " · k this is the cast page · y a new play here",
                        )
                }
                View::Cast => lang::l(
                    "↑↓ · ⏎ satırı düzelt · n yeni satır · x sil · Esc vazgeç",
                    "↑↓ · ⏎ edit the line · n new line · x delete · Esc cancel",
                )
                .to_string(),
                View::Play => lang::l(
                    "↑↓ kaydır · ⏎ içe aktar · t başlık · Esc vazgeç",
                    "↑↓ scroll · ⏎ import · t title · Esc cancel",
                )
                .to_string(),
            };
            if self.ocr {
                s += lang::l(" · o OCR ile yeniden oku", " · o read again with OCR");
            }
            s
        };
        let mut foot_lines = vec![];
        if let Some(e) = &self.error {
            foot_lines.push(Line::from(Span::styled(
                e.clone(),
                Style::default().fg(WARN),
            )));
        } else {
            foot_lines.push(Line::raw(""));
        }
        foot_lines.push(Line::from(Span::styled(help, Style::default().fg(MUTED))));
        f.render_widget(
            Paragraph::new(foot_lines).wrap(ratatui::widgets::Wrap { trim: true }),
            foot,
        );
        if let Some(form) = &self.form {
            popup::draw_form(f, area, form);
        }
    }
}
