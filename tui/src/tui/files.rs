//! What belongs to a play, each a key from opening: its folder, its PDF,
//! the role table, the actors' copies, its drafts. On the home screen and in
//! the writer. Reading them and asking the file manager wait on the disk and
//! the desktop, so both go on a thread and the cursor keeps blinking.

use super::importing::can_choose;
use super::popup;
use super::theme::{BEIGE, FAINT, FG, MUTED, WARN};
use dactile::outputs::{self, Outputs};
use dactile::{build, config, lang};
use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, channel};

/// A line of the list; no path, nothing made yet.
struct Row {
    name: String,
    /// What it says after the name.
    note: String,
    path: Option<PathBuf>,
    /// A draft: it opens in the writer, not in the desktop's program.
    play: bool,
}

/// What a key leaves to the screen around the list.
pub enum Step {
    Stay,
    Close,
    /// A draft to open in the writer.
    Open(PathBuf),
}

pub struct Files {
    rx: Option<Receiver<(Outputs, bool)>>,
    rows: Vec<Row>,
    sel: usize,
    /// The play is in a folder the system empties.
    temporary: bool,
    /// The file manager's answer, while it is asked.
    reveal: Option<Receiver<Result<(), String>>>,
    message: Option<(String, bool)>,
}

/// Shows `path` in the file manager, on a thread: the answer can take
/// seconds. Where there is no desktop, nothing is launched.
pub fn reveal(path: &Path) -> Receiver<Result<(), String>> {
    let (tx, rx) = channel();
    let path = path.to_path_buf();
    if !can_choose() {
        let _ = tx.send(Err(config::tilde(&path)));
        return rx;
    }
    std::thread::spawn(move || {
        let _ = tx.send(build::reveal(&path).map_err(|e| format!("{e:#}")));
    });
    rx
}

/// The folder a play is in, if it is one the system empties: for a warning.
pub fn temporary(play_or_dir: &Path) -> bool {
    outputs::is_temporary(play_or_dir)
}

/// The warning, as the desktop has it.
pub fn temporary_warning() -> &'static str {
    lang::l(
        "Bu klasör geçici: bilgisayar yeniden başlayınca silinebilir.",
        "This folder is temporary: it may be erased when the computer restarts.",
    )
}

fn date(t: Option<std::time::SystemTime>) -> String {
    t.map_or(String::new(), |t| {
        let d: chrono::DateTime<chrono::Local> = t.into();
        d.format("%d.%m.%Y").to_string()
    })
}

fn rows(o: Outputs) -> Vec<Row> {
    let row = |name: String, note: String, path: Option<PathBuf>| Row {
        name,
        note,
        path,
        play: false,
    };
    let mut out = vec![row(
        lang::l("Klasör", "Folder").into(),
        config::tilde(&o.folder),
        Some(o.folder),
    )];
    out.push(match o.pdf {
        Some(m) => row("PDF".into(), date(m.modified), Some(m.path)),
        None => row(
            "PDF".into(),
            lang::l(
                "Henüz yok: oyun açıkken F5 ile derle.",
                "None yet: build it with F5 while the play is open.",
            )
            .into(),
            None,
        ),
    });
    if let Some(m) = o.table {
        out.push(row(
            lang::l("Dağıtım tablosu", "Role table").into(),
            date(m.modified),
            Some(m.path),
        ));
    }
    out.push(match o.copies {
        Some(c) => row(
            lang::l("Oyuncu kopyaları", "Actors' copies").into(),
            format!(
                "{} · {}",
                lang::f("{} PDF", "{} PDFs", &[&c.pdfs]),
                date(c.modified)
            ),
            Some(c.path),
        ),
        None => row(
            lang::l("Oyuncu kopyaları", "Actors' copies").into(),
            lang::l(
                "Henüz yok: Dağıtım (F9) içinden yapılır.",
                "None yet: made from Casting (F9).",
            )
            .into(),
            None,
        ),
    });
    for d in o.drafts {
        let name = match &d.draft {
            Some(n) => lang::f("Taslak {}", "Draft {}", &[n]),
            None => lang::l("Taslak", "Draft").to_string(),
        };
        out.push(Row {
            name,
            note: format!("{} · {}", d.title, date(d.modified)),
            path: Some(d.path),
            play: true,
        });
    }
    out
}

impl Files {
    pub fn open(play: &Path) -> Files {
        let (tx, rx) = channel();
        let play = play.to_path_buf();
        std::thread::spawn(move || {
            let _ = tx.send((outputs::of(&play), temporary(&play)));
        });
        Files {
            rx: Some(rx),
            rows: Vec::new(),
            sel: 0,
            temporary: false,
            reveal: None,
            message: None,
        }
    }

    /// Whether it waits on something: the screen is drawn more often.
    pub fn busy(&self) -> bool {
        self.rx.is_some() || self.reveal.is_some()
    }

    /// The answers, when they come.
    pub fn tick(&mut self) {
        if let Some((o, temporary)) = self.rx.as_ref().and_then(|rx| rx.try_recv().ok()) {
            self.rows = rows(o);
            self.temporary = temporary;
            self.rx = None;
        }
        if let Some(r) = self.reveal.as_ref().and_then(|rx| rx.try_recv().ok()) {
            self.reveal = None;
            self.message = r.err().map(|e| (e, true));
        }
    }

    pub fn key(&mut self, k: KeyEvent) -> Step {
        self.message = None;
        let chosen = self.rows.get(self.sel);
        match k.code {
            KeyCode::Esc => return Step::Close,
            KeyCode::Up => self.sel = self.sel.saturating_sub(1),
            KeyCode::Down => self.sel = (self.sel + 1).min(self.rows.len().saturating_sub(1)),
            KeyCode::Enter => match chosen.and_then(|r| r.path.clone().map(|p| (p, r.play))) {
                Some((p, true)) => return Step::Open(p),
                Some((p, false)) if can_choose() => {
                    if let Err(e) = build::launch(&p) {
                        self.message = Some((format!("{e:#}"), true));
                    }
                }
                // Over SSH: the path is what there is to give.
                Some((p, false)) => self.message = Some((config::tilde(&p), false)),
                None => {}
            },
            KeyCode::Char('r') => {
                if let Some(p) = chosen.and_then(|r| r.path.clone()) {
                    self.reveal = Some(reveal(&p));
                }
            }
            _ => {}
        }
        Step::Stay
    }

    pub fn draw(&self, f: &mut Frame, area: Rect) {
        let w = 72.min(area.width);
        let h = (self.rows.len() as u16 + 6).clamp(9, 24).min(area.height);
        let r = Rect {
            x: area.x + (area.width - w) / 2,
            y: area.y + (area.height - h) / 2,
            width: w,
            height: h,
        };
        popup::float(f, r);
        f.render_widget(popup::panel(lang::l("dosyalarım", "my files")), r);
        let inner = popup::inner(r);
        let room = (inner.height as usize).saturating_sub(2).max(1);
        let first = (self.sel + 1).saturating_sub(room);
        let name_w = 20usize.min(inner.width as usize / 2);
        let note_w = (inner.width as usize).saturating_sub(name_w + 3);
        let mut lines: Vec<Line> = self
            .rows
            .iter()
            .enumerate()
            .skip(first)
            .take(room)
            .map(|(i, row)| {
                let color = if row.path.is_some() { FG } else { FAINT };
                popup::row(
                    vec![
                        popup::mark(i == self.sel),
                        Span::styled(
                            format!("{:<name_w$} ", popup::fit(&row.name, name_w)),
                            Style::default().fg(BEIGE),
                        ),
                        Span::styled(popup::fit(&row.note, note_w), Style::default().fg(color)),
                    ],
                    i == self.sel,
                    inner.width,
                )
            })
            .collect();
        if self.rx.is_some() {
            lines.push(Line::from(Span::styled(
                lang::l("Okunuyor…", "Reading…"),
                Style::default().fg(MUTED),
            )));
        }
        f.render_widget(Paragraph::new(lines), inner);
        let keys = lang::l(
            "⏎ aç  r dosya yöneticisinde göster  Esc",
            "⏎ open  r show in the file manager  Esc",
        );
        let foot = match &self.message {
            Some((m, warn)) => {
                let color = if *warn { WARN } else { MUTED };
                Span::styled(
                    popup::fit(m, inner.width as usize),
                    Style::default().fg(color),
                )
            }
            None if self.temporary => Span::styled(
                popup::fit(temporary_warning(), inner.width as usize),
                Style::default().fg(WARN),
            ),
            None => Span::styled(keys, Style::default().fg(MUTED)),
        };
        f.render_widget(
            Paragraph::new(foot),
            Rect {
                y: inner.y + inner.height.saturating_sub(1),
                height: 1.min(inner.height),
                ..inner
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dactile::outputs::{Copies, Draft, Made};

    fn outputs_of() -> Outputs {
        Outputs {
            folder: PathBuf::from("/yok"),
            pdf: Some(Made {
                path: PathBuf::from("/yok/oyun.pdf"),
                modified: None,
            }),
            copies: Some(Copies {
                path: PathBuf::from("/yok/kopyalar"),
                pdfs: 3,
                modified: None,
            }),
            table: None,
            drafts: vec![Draft {
                path: PathBuf::from("/yok/oyun-0.2.tex"),
                title: "Kar Masalı".into(),
                draft: Some("0.2".into()),
                modified: None,
            }],
        }
    }

    fn listed() -> Files {
        Files {
            rx: None,
            rows: rows(outputs_of()),
            sel: 0,
            temporary: false,
            reveal: None,
            message: None,
        }
    }

    fn press(f: &mut Files, code: KeyCode) -> Step {
        f.key(KeyEvent::new(
            code,
            ratatui::crossterm::event::KeyModifiers::NONE,
        ))
    }

    #[test]
    fn a_draft_opens_in_the_writer_and_esc_closes() {
        let mut f = listed();
        assert!(matches!(press(&mut f, KeyCode::Esc), Step::Close));
        for _ in 0..9 {
            press(&mut f, KeyCode::Down);
        }
        let Step::Open(p) = press(&mut f, KeyCode::Enter) else {
            panic!("no draft opened");
        };
        assert_eq!(p, PathBuf::from("/yok/oyun-0.2.tex"));
    }

    #[test]
    fn what_is_not_made_yet_has_no_path() {
        let mut o = outputs_of();
        o.pdf = None;
        o.copies = None;
        let r = rows(o);
        assert!(r[1].path.is_none() && r[2].path.is_none());
    }

    #[test]
    fn it_draws_on_any_screen() {
        let f = listed();
        for (w, h) in [(100, 30), (40, 10), (10, 4)] {
            let mut t = ratatui::Terminal::new(ratatui::backend::TestBackend::new(w, h)).unwrap();
            t.draw(|fr| f.draw(fr, fr.area())).unwrap();
        }
    }

    #[test]
    fn the_list_is_read_on_a_thread_and_arrives() {
        let dir = std::env::temp_dir().join(format!("dactile-files-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut f = Files::open(&dir.join("oyun.tex"));
        for _ in 0..200 {
            f.tick();
            if !f.busy() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(!f.busy());
        assert_eq!(f.rows[0].path.as_deref(), Some(dir.as_path()));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_temporary_folder_is_told_from_the_others() {
        assert!(temporary(Path::new("/tmp/oyun.tex")));
        assert!(!temporary(Path::new("/opt/oyunlar/oyun.tex")));
    }
}
