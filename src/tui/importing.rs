//! Importing on the home screen: the desktop's file chooser (or a path
//! typed where there is none), the reading in the background, and a
//! preview of the names found to put right before the play is written.

use super::popup::{self, Field, Form, FormKind, Outcome};
use super::theme::{BEIGE, CLARET, FAINT, FG, MUTED, WARN};
use dactile::config::Config;
use dactile::import::shape::{Draft, shape};
use dactile::lang;
use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};

pub enum Importing {
    Picking {
        rx: Receiver<Option<PathBuf>>,
        since: Instant,
    },
    Reading {
        rx: Receiver<Result<Draft, String>>,
        file: PathBuf,
    },
    Preview(Box<Preview>),
}

pub struct Preview {
    file: PathBuf,
    draft: Draft,
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
    AskPath,
    /// Written here: open it.
    Open(PathBuf),
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

/// The desktop's chooser, in the background.
pub fn pick() -> Importing {
    let (tx, rx) = channel();
    let dir = dirs::download_dir()
        .filter(|d| d.is_dir())
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("/"));
    std::thread::spawn(move || {
        let chosen = rfd::FileDialog::new()
            .set_title(lang::l("İçe aktarılacak oyun", "A play to import"))
            .add_filter(
                lang::l("Oyun belgeleri", "Play documents"),
                &dactile::import::EXTENSIONS,
            )
            .set_directory(dir)
            .pick_file();
        let _ = tx.send(chosen);
    });
    Importing::Picking {
        rx,
        since: Instant::now(),
    }
}

/// Reading and shaping `file`, in the background (a PDF takes a while).
pub fn read(file: PathBuf, lang: lang::Lang) -> Importing {
    let (tx, rx) = channel();
    let f = file.clone();
    std::thread::spawn(move || {
        let r = dactile::import::read(&f)
            .map(|src| shape(&src, lang))
            .map_err(|e| format!("{e:#}"));
        let _ = tx.send(r);
    });
    Importing::Reading { rx, file }
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
        !matches!(self, Importing::Preview(_))
    }

    /// The chooser's or the reader's answer, when it comes.
    pub fn tick(&mut self, lang: lang::Lang) -> Step {
        match self {
            Importing::Picking { rx, since } => match rx.try_recv() {
                Ok(Some(path)) => {
                    *self = read(path, lang);
                    Step::Stay
                }
                // Back at once with nothing: no chooser came up.
                Ok(None) if since.elapsed() < Duration::from_millis(400) => Step::AskPath,
                Ok(None) => Step::Done(None),
                Err(std::sync::mpsc::TryRecvError::Empty) => Step::Stay,
                Err(_) => Step::AskPath,
            },
            Importing::Reading { rx, file } => match rx.try_recv() {
                Ok(Ok(draft)) => {
                    *self = Importing::Preview(Box::new(Preview {
                        file: file.clone(),
                        draft,
                        sel: 0,
                        merging: None,
                        form: None,
                        error: None,
                    }));
                    Step::Stay
                }
                Ok(Err(e)) => Step::Done(Some((e, true))),
                Err(std::sync::mpsc::TryRecvError::Empty) => Step::Stay,
                Err(_) => Step::Done(None),
            },
            Importing::Preview(_) => Step::Stay,
        }
    }

    pub fn key(&mut self, k: KeyEvent, cfg: &Config) -> Step {
        match self {
            Importing::Preview(p) => p.key(k, cfg),
            // Waiting: Esc gives up (a chooser left open closes on its own).
            _ if k.code == KeyCode::Esc => Step::Done(None),
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
            Importing::Reading { file, .. } => note(
                f,
                area,
                &lang::f(
                    "{} okunuyor…",
                    "Reading {}…",
                    &[&file.file_name().unwrap_or_default().to_string_lossy()],
                ),
            ),
            Importing::Preview(p) => p.draw(f, area),
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
    fn key(&mut self, k: KeyEvent, cfg: &Config) -> Step {
        self.error = None;
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
        let n = self.draft.people.len();
        match k.code {
            KeyCode::Esc if self.merging.is_some() => self.merging = None,
            KeyCode::Esc => {
                return Step::Done(Some((
                    lang::l("İçe aktarma bırakıldı.", "Import dropped.").into(),
                    false,
                )));
            }
            KeyCode::Up => self.sel = self.sel.saturating_sub(1),
            KeyCode::Down => self.sel = (self.sel + 1).min(n.saturating_sub(1)),
            KeyCode::Enter if self.merging.is_some() => {
                let from = self.merging.take().unwrap_or(self.sel);
                if from != self.sel {
                    self.draft.merge(from, self.sel);
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
            _ if n == 0 => {}
            KeyCode::Char('m') => {
                if self.draft.people[self.sel].merged_into.is_some() {
                    self.draft.unmerge(self.sel);
                } else {
                    self.merging = Some(self.sel);
                }
            }
            KeyCode::Char('x') => {
                let p = &mut self.draft.people[self.sel];
                p.not_character = !p.not_character;
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
            }
            FormKind::ImportInfo => {
                self.draft.title = form.get("Başlık").to_string();
                self.draft.author = form.get("Yazar").to_string();
            }
            _ => {}
        }
        Ok(())
    }

    /// The play, in the plays folder under its title's file name.
    fn write(&mut self, cfg: &Config) -> Step {
        let stem = self
            .file
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        let title = if self.draft.title.trim().is_empty() {
            stem
        } else {
            self.draft.title.clone()
        };
        let dir = cfg.plays_dir();
        if let Err(e) = std::fs::create_dir_all(&dir) {
            self.error = Some(format!("{}: {e}", dir.display()));
            return Step::Stay;
        }
        let path = dactile::library::free_path(&dir, &dactile::library::file_name(&title));
        match dactile::file::write_atomic(&path, &self.draft.tex()) {
            Ok(()) => Step::Open(path),
            Err(e) => {
                self.error = Some(format!("{e:#}"));
                Step::Stay
            }
        }
    }

    fn draw(&self, f: &mut Frame, area: Rect) {
        let w = area.width.saturating_sub(6).clamp(40, 110);
        let h = area.height.saturating_sub(4).clamp(12, 34);
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
            Line::raw(""),
        ];
        let name_w = d
            .people
            .iter()
            .map(|p| p.name.chars().count())
            .max()
            .unwrap_or(4)
            .clamp(4, 24);
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
        let top = lines.len();
        let room = (inner.height as usize).saturating_sub(top + 3).max(1);
        let first = self.sel.saturating_sub(room - 1);
        for (i, p) in d.people.iter().enumerate().skip(first).take(room) {
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
        if d.people.is_empty() {
            lines.push(Line::from(Span::styled(
                lang::l(
                    "Konuşan kimse bulunamadı: metin düz paragraf olarak gelir.",
                    "No one found speaking: the text comes in as plain paragraphs.",
                ),
                muted,
            )));
        }
        f.render_widget(Paragraph::new(lines), inner);
        let foot = Rect {
            y: inner.y + inner.height.saturating_sub(2),
            height: 2,
            ..inner
        };
        let help = if let Some(m) = self.merging {
            lang::f(
                "{} kiminle birleşsin? ↑↓ seç · ⏎ birleştir · Esc vazgeç",
                "Merge {} into whom? ↑↓ choose · ⏎ merge · Esc cancel",
                &[&d.people[m].name],
            )
        } else {
            lang::l(
                "⏎ içe aktar · e ad · a kısaltma · m birleştir/ayır · x karakter değil · t başlık · Esc vazgeç",
                "⏎ import · e name · a alias · m merge/split · x not a character · t title · Esc cancel",
            )
            .to_string()
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
