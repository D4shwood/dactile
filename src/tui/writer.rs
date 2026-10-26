//! The writing screen: the play as it will read, edited in place.

use super::keys;
use super::panels::{self, CastPanel, Entry, ScenesPanel};
use super::popup::{self, Field, Form, FormKind, Menu, Verbs};
use super::theme::{self, BEIGE, CLARET, FAINT, MUTED, WARN};
use anyhow::Result;
use dactile::buffer::{self, Buffer};
use dactile::cast::{self as dcast, Spec};
use dactile::complete::{self, Action};
use dactile::file::{self, LockState};
use dactile::lex::Kind;
use dactile::outline;
use dactile::render::{self, Doc, Layout, Options, Stop};
use dactile::sidecar::Sidecar;
use dactile::source;
use dactile::spell::{self, Speller};
use dactile::{info, lang, shelf, timing};
use ratatui::Frame;
use ratatui::crossterm::event::{
    KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::layout::{Position, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant, SystemTime};

/// Saved this long after the last keystroke.
const AUTOSAVE_AFTER: Duration = Duration::from_secs(2);
/// How often the file on disk is checked for changes from elsewhere.
const CHECK_EVERY: Duration = Duration::from_secs(2);
/// Columns left of the text on the page: the draft line numbers.
const MARGIN: u16 = 7;
/// Air right of the text on the page.
const PAGE_RIGHT: u16 = 3;
/// The widest the text gets: about a typed page.
const PAGE_TEXT: u16 = 66;

/// Where the page sits on the screen.
struct Sheet {
    x: u16,
    width: u16,
    /// The text's first column.
    text_x: u16,
    text_w: u16,
}

pub struct Writer {
    path: PathBuf,
    buf: Buffer,
    doc: Doc,
    lay: Layout,
    width: u16,
    height: u16,
    /// The block at the cursor is shown as raw source.
    raw_here: bool,
    raw_all: bool,
    preamble_open: bool,
    scroll: usize,
    /// The column Up/Down try to keep.
    goal_col: Option<u16>,
    _lock: Option<file::Lock>,
    read_only: bool,
    history: PathBuf,
    /// The file's modification time after our last read or write.
    disk_time: Option<SystemTime>,
    checked: Instant,
    /// The file changed on disk while we had unsaved changes: autosave waits.
    conflict: bool,
    message: Option<(String, bool)>,
    clipboard: Option<arboard::Clipboard>,
    /// Fallback when there's no system clipboard.
    copied: String,
    popup: Popup,
    /// The cursor's line stays in the middle of the screen.
    typewriter: bool,
    /// Only the cursor's scene; the others fold to one line.
    focus: bool,
    /// Everything but the cursor's block in the background.
    dim: bool,
    /// The play on a paper page; dark otherwise.
    paper: bool,
    /// Page view: A4-shaped sheets broken where the PDF breaks.
    pages: bool,
    /// `NAME:` just became a line with its space: the next space is dropped.
    eat_space: bool,
    /// The keyboard the help screen draws.
    keyboard: String,
    /// The margin map's rows on screen and the block each one opens.
    map_hits: Vec<(u16, usize)>,
    /// Acts and scenes marked to hide or throw away, by where they start.
    /// Any edit clears them.
    marks: std::collections::BTreeSet<usize>,
    /// The play's trash, as last read.
    trash: Vec<shelf::Trashed>,
    /// Suggestions only with ^K, not on every backslash.
    menu_manual: bool,
    default_wpm: u32,
    /// The top of the text area on screen, for the mouse.
    text_top: u16,
    sidecar: Sidecar,
    speller: Option<Speller>,
    speller_rx: Option<Receiver<Result<Speller>>>,
    /// Words the play knows: names and words added by hand, lower case.
    known: HashSet<String>,
    spell_cache: HashMap<String, bool>,
    /// Misspelt words on screen, by byte range.
    misspelt: Vec<std::ops::Range<usize>>,
    pub quit: bool,
}

/// The scenes a character speaks in, to give some to a new character.
pub struct SplitPanel {
    pub from: String,
    pub spec: Spec,
    /// Indexes into the outline's scenes.
    pub scenes: Vec<usize>,
    pub picked: std::collections::BTreeSet<usize>,
    pub sel: usize,
}

/// The view toggles, kept in the settings between sessions.
pub struct View {
    pub typewriter: bool,
    pub focus: bool,
    pub dim: bool,
    pub paper: bool,
    pub pages: bool,
}

enum Popup {
    None,
    /// After a backslash: what can follow it.
    Menu(Menu),
    /// After `\xdo `: how directions usually start.
    Verbs(Verbs),
    /// ^K: the same list, searched by name.
    Search(Menu, String),
    Form(Form),
    /// ^O: the cast.
    Cast(CastPanel),
    /// ^G: the acts and scenes, and the trash.
    Scenes(ScenesPanel),
    /// Splitting a character: which of its scenes the new one takes.
    Split(SplitPanel),
    /// F1: the keyboard, with the key last pressed or pointed at.
    Help(keys::Help),
    /// Alt+S: spelling suggestions for the word at `range`.
    Suggest(std::ops::Range<usize>, Vec<String>, usize),
}

impl Writer {
    pub fn open(path: &Path, cfg: &dactile::config::Config) -> Result<Writer> {
        let text = file::read(path)?;
        lang::set(lang::resolve(&cfg.lang, Some(&text)));
        let (lock, read_only, message) = match file::lock(path)? {
            LockState::Ours(l) => (Some(l), false, None),
            LockState::Taken(who) => (
                None,
                true,
                Some((
                    lang::f(
                        "Başka bir Dactile bu oyunu açmış ({}); salt okunur açıldı.",
                        "Another Dactile has this play open ({}); opened read-only.",
                        &[&who],
                    ),
                    true,
                )),
            ),
        };
        let history = file::history_dir(path);
        // The play as it was before this session, whatever happens next.
        let _ = file::snapshot(&history, &text, true);
        let doc = Doc::new(&text);
        let mut buf = Buffer::new(text);
        // Start at the first line of the body that has something on it.
        buf.cursor = doc.blocks[doc.body.clone()]
            .iter()
            .find(|b| !matches!(b.kind, Kind::Blank))
            .map_or(doc.parts.body.start, |b| b.span.start);
        let mut w = Writer {
            path: path.to_path_buf(),
            buf,
            doc,
            lay: Layout::default(),
            width: 80,
            height: 24,
            raw_here: false,
            raw_all: false,
            preamble_open: false,
            scroll: 0,
            goal_col: None,
            _lock: lock,
            read_only,
            history,
            disk_time: file::modified(path),
            checked: Instant::now(),
            conflict: false,
            message,
            clipboard: arboard::Clipboard::new().ok(),
            copied: String::new(),
            popup: Popup::None,
            typewriter: cfg.typewriter,
            focus: cfg.focus,
            dim: cfg.dim,
            paper: cfg.paper,
            pages: cfg.pages,
            eat_space: false,
            keyboard: cfg.keyboard.clone(),
            map_hits: Vec::new(),
            marks: Default::default(),
            trash: Vec::new(),
            menu_manual: cfg.menu == "manual",
            default_wpm: cfg.wpm,
            text_top: 1,
            sidecar: Sidecar::load(path).unwrap_or_default(),
            speller: None,
            speller_rx: None,
            known: HashSet::new(),
            spell_cache: HashMap::new(),
            misspelt: Vec::new(),
            quit: false,
        };
        // The dictionary takes a moment to load: in the background.
        if let Some((aff, dic)) = spell::find() {
            let (tx, rx) = channel();
            std::thread::spawn(move || {
                let _ = tx.send(Speller::load(&aff, &dic));
            });
            w.speller_rx = Some(rx);
        }
        w.refresh_known();
        w.relayout();
        w.snap_forward();
        Ok(w)
    }

    // ------------------------------------------------------------ model

    fn block(&self) -> Option<usize> {
        self.doc.block_at(self.buf.cursor)
    }

    fn opts(&self) -> Options {
        Options {
            width: self.text_width(),
            raw_block: if self.raw_here { self.block() } else { None },
            raw_all: self.raw_all,
            preamble_open: self.preamble_open,
            page_rows: self.pages.then(|| self.page_rows()),
            focus_scene: if self.focus {
                self.block().and_then(|b| self.doc.scene_of(b))
            } else {
                None
            },
        }
    }

    fn text_width(&self) -> u16 {
        self.sheet().text_w
    }

    /// A page of `PAGE_TEXT` columns in the middle, or the whole screen when
    /// that doesn't fit.
    fn sheet(&self) -> Sheet {
        let full = MARGIN + PAGE_TEXT + PAGE_RIGHT;
        if self.width >= full + 4 {
            let x = (self.width - full) / 2;
            Sheet {
                x,
                width: full,
                text_x: x + MARGIN,
                text_w: PAGE_TEXT,
            }
        } else {
            Sheet {
                x: 0,
                width: self.width,
                text_x: MARGIN,
                text_w: self.width.saturating_sub(MARGIN + 1).max(20),
            }
        }
    }

    /// Rows of text on a sheet in page view: an A4 sheet as wide as the
    /// page on screen (terminal cells are about twice as tall as wide), less
    /// its margins.
    fn page_rows(&self) -> u16 {
        let tall = (self.sheet().width as f32 * 1.414 / 2.0).round() as u16;
        tall.saturating_sub((render::PAGE_TOP + render::PAGE_BOTTOM) as u16)
            .max(12)
    }

    fn palette(&self) -> &'static theme::Palette {
        if self.paper {
            &theme::PAPER
        } else {
            &theme::DARK
        }
    }

    /// Rebuilds the model and the layout from the text.
    fn relayout(&mut self) {
        self.doc = Doc::new(&self.buf.text);
        self.lay = render::layout(&self.buf.text, &self.doc, self.opts());
    }

    /// Lays out again after an edit; the cursor lands on a stop.
    fn edited(&mut self) {
        self.marks.clear();
        self.relayout();
        self.snap_back();
    }

    fn cursor_stop(&self) -> Option<Stop> {
        self.lay.stop_at(self.buf.cursor)
    }

    /// Moves the cursor onto the stop at or before it.
    fn snap_back(&mut self) {
        if !self.lay.is_stop(self.buf.cursor)
            && let Some(s) = self.lay.stop_at(self.buf.cursor)
        {
            self.buf.cursor = s.byte;
        }
    }

    fn snap_forward(&mut self) {
        if !self.lay.is_stop(self.buf.cursor)
            && let Some(s) = self
                .lay
                .next_stop(self.buf.cursor)
                .or_else(|| self.lay.stop_at(self.buf.cursor))
        {
            self.buf.cursor = s.byte;
        }
    }

    /// Moves to `byte`; leaving the block closes its raw view.
    fn go(&mut self, byte: usize, select: bool) {
        if select {
            self.buf.anchor.get_or_insert(self.buf.cursor);
        } else {
            self.buf.anchor = None;
        }
        let before = self.block();
        let scene = before.and_then(|b| self.doc.scene_of(b));
        self.buf.cursor = byte;
        let moved = self.block() != before;
        if self.raw_here && moved {
            self.raw_here = false;
            self.relayout();
            self.snap_back();
        } else if self.focus && moved && self.block().and_then(|b| self.doc.scene_of(b)) != scene {
            // Into a folded scene: it opens, the one left folds.
            self.relayout();
            self.snap_forward();
        }
    }

    fn in_folded_preamble(&self) -> bool {
        !self.preamble_open && !self.raw_all && self.buf.cursor < self.doc.parts.body.start
    }

    fn editable(&mut self) -> bool {
        if self.read_only {
            self.say(
                lang::l(
                    "Salt okunur: oyun başka bir Dactile'da açık.",
                    "Read-only: the play is open in another Dactile.",
                ),
                true,
            );
            return false;
        }
        if self.in_folded_preamble() {
            self.say(
                lang::l(
                    "Kişiler ve oyun bilgisi kapalı: Alt+P ile aç.",
                    "The cast and cover lines are folded: Alt+P opens them.",
                ),
                false,
            );
            return false;
        }
        true
    }

    fn say(&mut self, s: &str, warn: bool) {
        self.message = Some((s.to_string(), warn));
    }

    fn raw_now(&self) -> bool {
        self.raw_here || self.raw_all
    }

    // ------------------------------------------------------------ keys

    pub fn key(&mut self, k: KeyEvent) {
        self.message = None;
        if !matches!(k.code, KeyCode::Char(_)) {
            self.eat_space = false;
        }
        if self.popup_key(k) {
            self.keep_visible();
            return;
        }
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        let alt = k.modifiers.contains(KeyModifiers::ALT);
        let shift = k.modifiers.contains(KeyModifiers::SHIFT);
        let vertical = matches!(
            k.code,
            KeyCode::Up | KeyCode::Down | KeyCode::PageUp | KeyCode::PageDown
        );
        if !vertical {
            self.goal_col = None;
        }
        match k.code {
            KeyCode::Char('q') if ctrl => self.quit = true,
            KeyCode::Char('s') if ctrl => self.save(true),
            KeyCode::Char('z') if ctrl => {
                if self.buf.undo() {
                    self.raw_here = false;
                    self.edited();
                }
            }
            KeyCode::Char('y') if ctrl => {
                if self.buf.redo() {
                    self.raw_here = false;
                    self.edited();
                }
            }
            KeyCode::Char('c') if ctrl => self.copy(false),
            KeyCode::Char('x') if ctrl => self.copy(true),
            KeyCode::Char('v') if ctrl => {
                let text = self
                    .clipboard
                    .as_mut()
                    .and_then(|c| c.get_text().ok())
                    .unwrap_or_else(|| self.copied.clone());
                self.paste(&text);
            }
            KeyCode::Char('a') if ctrl => {
                self.buf.anchor = Some(self.doc.parts.body.start);
                self.buf.cursor = self.doc.parts.body.end;
                self.snap_back();
            }
            KeyCode::Char('p') if alt => {
                self.preamble_open = !self.preamble_open;
                if !self.preamble_open && self.buf.cursor < self.doc.parts.body.start {
                    self.buf.cursor = self.doc.parts.body.start;
                }
                self.relayout();
                self.snap_forward();
            }
            KeyCode::F(2) => {
                self.raw_all = !self.raw_all;
                self.relayout();
                self.snap_back();
            }
            KeyCode::F(3) => {
                self.typewriter = !self.typewriter;
                self.say(
                    if self.typewriter {
                        lang::l("Daktilo modu açık.", "Typewriter mode on.")
                    } else {
                        lang::l("Daktilo modu kapalı.", "Typewriter mode off.")
                    },
                    false,
                );
            }
            KeyCode::F(4) => {
                self.focus = !self.focus;
                self.relayout();
                self.snap_back();
                self.say(
                    if self.focus {
                        lang::l(
                            "Sahne odağı: yalnız bu sahne.",
                            "Scene focus: this scene only.",
                        )
                    } else {
                        lang::l("Sahne odağı kapalı.", "Scene focus off.")
                    },
                    false,
                );
            }
            KeyCode::F(5) => {
                self.dim = !self.dim;
            }
            KeyCode::F(6) => {
                self.paper = !self.paper;
            }
            KeyCode::F(7) => {
                self.pages = !self.pages;
                self.relayout();
                self.snap_back();
                self.keep_visible();
                self.say(
                    if self.pages {
                        lang::l(
                            "Sayfa görünümü: A4 yapraklar, PDF'in böldüğü yerlerden.",
                            "Page view: A4 sheets, broken where the PDF breaks.",
                        )
                    } else {
                        lang::l(
                            "Sayfa görünümü kapalı: tek uzun kâğıt.",
                            "Page view off: one long sheet.",
                        )
                    },
                    false,
                );
            }
            KeyCode::Char('o') if ctrl => {
                self.popup = Popup::Cast(CastPanel {
                    sel: 0,
                    by_lines: false,
                });
            }
            KeyCode::Char('g') if ctrl => {
                // On the act or scene the cursor is in.
                let units = shelf::units(&self.buf.text, &self.doc);
                let at = self.buf.cursor;
                let sel = units.iter().rposition(|u| u.span.start <= at).unwrap_or(0);
                self.trash = shelf::load(&shelf::trash_path(&self.path));
                self.popup = Popup::Scenes(ScenesPanel {
                    sel,
                    ..Default::default()
                });
            }
            KeyCode::Char('f') if ctrl => {
                let q = self
                    .buf
                    .selected_text()
                    .filter(|t| !t.contains('\n'))
                    .unwrap_or("")
                    .to_string();
                self.popup = Popup::Form(Form {
                    kind: FormKind::Find,
                    title: lang::l("bul", "find").into(),
                    fields: vec![
                        Field::text("Ara", &q, Vec::new()),
                        Field::text("Değiştir", "", Vec::new()),
                    ],
                    focus: 0,
                    at: 0..0,
                    error: None,
                    confirmed: false,
                });
            }
            KeyCode::Char('i') if alt => self.popup = Popup::Form(self.info_form()),
            KeyCode::Char('s') if alt => self.suggest(),
            KeyCode::F(1) => self.popup = Popup::Help(keys::Help::default()),
            KeyCode::Char('k') if ctrl => {
                let items = complete::items(&self.doc, self.block());
                self.popup = Popup::Search(Menu::new(items, self.buf.cursor), String::new());
            }
            KeyCode::Esc => {
                self.buf.anchor = None;
                if self.raw_here {
                    self.raw_here = false;
                    self.relayout();
                    self.snap_back();
                }
            }
            KeyCode::Left => {
                let to = if ctrl {
                    self.word_left()
                } else {
                    self.lay.prev_stop(self.buf.cursor).map(|s| s.byte)
                };
                if let Some(b) = to {
                    self.go(b, shift);
                }
            }
            KeyCode::Right => {
                let to = if ctrl {
                    self.word_right()
                } else {
                    self.lay.next_stop(self.buf.cursor).map(|s| s.byte)
                };
                if let Some(b) = to {
                    self.go(b, shift);
                }
            }
            KeyCode::Up => self.vertical(-1, shift),
            KeyCode::Down => self.vertical(1, shift),
            KeyCode::PageUp => self.vertical(-(self.page() as isize), shift),
            KeyCode::PageDown => self.vertical(self.page() as isize, shift),
            KeyCode::Home if ctrl => self.go(self.lay.stops.first().map_or(0, |s| s.byte), shift),
            KeyCode::End if ctrl => self.go(self.lay.stops.last().map_or(0, |s| s.byte), shift),
            KeyCode::Home => {
                if let Some(s) = self.row_stops().first() {
                    self.go(s.byte, shift);
                }
            }
            KeyCode::End => {
                if let Some(s) = self.row_stops().last() {
                    self.go(s.byte, shift);
                }
            }
            KeyCode::Enter => {
                if self.editable() {
                    let nl = source::line_ending(&self.buf.text);
                    self.raw_here = false;
                    self.buf.insert(nl);
                    self.edited();
                }
            }
            KeyCode::Backspace => self.backspace(),
            KeyCode::Delete => self.delete(),
            KeyCode::Tab => {
                if let Some(a) = self.ghost()
                    && self.editable()
                {
                    self.buf.insert(&format!("\\{a} "));
                    self.edited();
                }
            }
            KeyCode::Char(c) if !ctrl && !alt => self.type_char(c),
            _ => {}
        }
        self.follow_popups();
        self.keep_visible();
    }

    fn page(&self) -> usize {
        self.height.saturating_sub(5).max(1) as usize
    }

    fn row_stops(&self) -> Vec<Stop> {
        let Some(cur) = self.cursor_stop() else {
            return Vec::new();
        };
        self.lay
            .stops
            .iter()
            .filter(|s| s.row == cur.row)
            .copied()
            .collect()
    }

    fn vertical(&mut self, rows: isize, select: bool) {
        let Some(cur) = self.cursor_stop() else {
            return;
        };
        let col = *self.goal_col.get_or_insert(cur.col);
        let last = self.lay.rows.len().saturating_sub(1) as isize;
        let mut row = cur.row as isize + rows;
        let step = rows.signum();
        // Skip rows without a stop (a scene's cast line).
        while (0..=last).contains(&row) {
            if let Some(s) = self.lay.stop_near(row as usize, col) {
                self.go(s.byte, select);
                return;
            }
            row += step;
        }
        let edge = if rows < 0 {
            self.lay.stops.first()
        } else {
            self.lay.stops.last()
        };
        if let Some(s) = edge.copied() {
            self.go(s.byte, select);
        }
    }

    fn word_left(&self) -> Option<usize> {
        let t = &self.buf.text[..self.buf.cursor];
        let trimmed = t.trim_end_matches(|c: char| !c.is_alphanumeric());
        let start = trimmed
            .rfind(|c: char| !c.is_alphanumeric())
            .map_or(0, |i| i + trimmed[i..].chars().next().unwrap().len_utf8());
        self.lay.stop_at(start).map(|s| s.byte)
    }

    fn word_right(&self) -> Option<usize> {
        let t = &self.buf.text[self.buf.cursor..];
        let skip = t.find(|c: char| c.is_alphanumeric()).unwrap_or(t.len());
        let rest = &t[skip..];
        let end = skip
            + rest
                .find(|c: char| !c.is_alphanumeric())
                .unwrap_or(rest.len());
        let at = self.buf.cursor + end;
        self.lay
            .stop_at(at)
            .map(|s| s.byte)
            .filter(|&b| b > self.buf.cursor)
            .or_else(|| self.lay.next_stop(self.buf.cursor).map(|s| s.byte))
    }

    /// Whether the character starting at `at` is shown as itself (so it can
    /// be deleted alone without breaking a macro).
    fn plain_at(&self, at: usize) -> bool {
        let Some(b) = self.doc.block_at(at) else {
            return false;
        };
        let Some(rows) = self.lay.block_rows.get(b) else {
            return false;
        };
        self.lay.rows[rows.clone()]
            .iter()
            .any(|r| r.cells.iter().any(|c| c.byte == Some(at)))
    }

    fn is_line_break(&self, at: usize) -> bool {
        matches!(self.buf.text.as_bytes().get(at), Some(b'\n' | b'\r'))
    }

    fn backspace(&mut self) {
        if !self.editable() {
            return;
        }
        if self.buf.selection().is_some() {
            self.buf.delete(0..0);
            self.edited();
            return;
        }
        let cur = self.buf.cursor;
        if cur == 0 {
            return;
        }
        let p = self.buf.prev_char(cur);
        if self.is_line_break(p) {
            // Join with the line above ("\r\n" goes as one).
            let from = if p > 0
                && self.buf.text.as_bytes()[p] == b'\n'
                && self.buf.text.as_bytes()[p - 1] == b'\r'
            {
                p - 1
            } else {
                p
            };
            self.raw_here = false;
            self.buf.delete(from..cur);
            self.edited();
        } else if self.raw_now() || self.plain_at(p) {
            self.buf.delete(p..cur);
            self.edited();
        } else {
            // Into a macro: show the block's source to edit it.
            self.raw_here = true;
            self.relayout();
            self.snap_back();
        }
    }

    fn delete(&mut self) {
        if !self.editable() {
            return;
        }
        if self.buf.selection().is_some() {
            self.buf.delete(0..0);
            self.edited();
            return;
        }
        let cur = self.buf.cursor;
        if cur >= self.buf.text.len() {
            return;
        }
        let n = self.buf.next_char(cur);
        if self.is_line_break(cur) {
            let to = if self.buf.text[cur..].starts_with("\r\n") {
                cur + 2
            } else {
                n
            };
            self.buf.delete(cur..to);
            self.edited();
        } else if self.raw_now() || self.plain_at(cur) {
            self.buf.delete(cur..n);
            self.edited();
        } else {
            self.raw_here = true;
            self.relayout();
            self.snap_back();
        }
    }

    fn type_char(&mut self, c: char) {
        if !self.editable() {
            return;
        }
        if std::mem::take(&mut self.eat_space) && c == ' ' {
            return;
        }
        let block = self.block().unwrap_or(0);
        let dialogue = self.doc.is_dialogue(block) || self.at_blank_line();
        let hash_ok = !info::has_option(&self.buf.text, "nohash");
        if c == '#' && hash_ok && !self.raw_all && self.at_line_start() {
            // A heading: #sahne, #perde, #önsöz, #notlar.
            self.raw_here = true;
            self.buf.insert("#");
            let nl = source::line_ending(&self.buf.text).to_string();
            let mut m = Menu::new(complete::headings(&nl), self.buf.cursor);
            m.hash = true;
            self.popup = Popup::Menu(m);
            self.edited();
            return;
        }
        if c == ':' && !self.raw_now() && self.name_colon() {
            return;
        }
        if c == '\\' {
            // A macro is being written: its block shows as source until done,
            // with what can follow the backslash.
            self.raw_here = true;
            self.buf.insert("\\");
            let items = complete::items(&self.doc, self.block());
            if !self.menu_manual {
                self.popup = Popup::Menu(Menu::new(items, self.buf.cursor));
            }
        } else if self.raw_now() {
            let name = if c == ' ' { self.macro_before() } else { None };
            self.buf.insert(&c.to_string());
            // `\se ` or `\se{` finishes the macro: show the block live again.
            if self.raw_here && (c == ' ' || c == '{') && self.ends_macro() {
                self.raw_here = false;
            }
            if let Some(name) = name {
                self.edited();
                self.after_macro(&name);
                return;
            }
        } else if c == '(' && dialogue && self.buf.text[..self.buf.cursor].ends_with('(') {
            // "((" in dialogue: an inline direction, \ic{…}.
            let start = self.buf.cursor - 1;
            self.buf.apply(
                &[dactile::tex::Edit::new(start..self.buf.cursor, "\\ic{}")],
                start + 4,
            );
        } else {
            self.buf.insert(&buffer::escape_typed(c, dialogue));
        }
        self.edited();
        // The edit may leave the cursor between stops; keep it after what was typed.
        self.snap_forward_if_inside();
    }

    fn snap_forward_if_inside(&mut self) {
        if !self.lay.is_stop(self.buf.cursor)
            && let Some(s) = self.lay.next_stop(self.buf.cursor)
        {
            self.buf.cursor = s.byte;
        }
    }

    /// The text before the cursor ends with `\letters` and the char just typed.
    fn ends_macro(&self) -> bool {
        let t = &self.buf.text[..self.buf.cursor];
        let mut chars = t.chars().rev();
        chars.next();
        let letters = chars
            .clone()
            .take_while(|c| dactile::tex::is_letter(*c))
            .count();
        letters > 0 && chars.nth(letters) == Some('\\')
    }

    /// Nothing but spaces between the line's start and the cursor.
    fn at_line_start(&self) -> bool {
        self.buf.text[self.line_start()..self.buf.cursor]
            .trim()
            .is_empty()
    }

    fn line_start(&self) -> usize {
        let t = &self.buf.text[..self.buf.cursor];
        t.rfind('\n').map_or(0, |i| i + 1)
    }

    /// `SELİN:` typed at a line's start: the line becomes SELİN's. A name in
    /// capitals nobody has (`AHMET:`) asks for a new character. True when the
    /// colon was taken.
    fn name_colon(&mut self) -> bool {
        let from = self.line_start();
        let typed = &self.buf.text[from..self.buf.cursor];
        let lead = from + (typed.len() - typed.trim_start().len());
        let name = typed.trim();
        if name.is_empty() || name.contains('\\') {
            return false;
        }
        if let Some(c) = self.doc.cast.by_name(name) {
            let line = format!("\\{} ", c.alias);
            self.buf.apply(
                &[dactile::tex::Edit::new(lead..self.buf.cursor, line.clone())],
                lead + line.len(),
            );
            self.raw_here = false;
            self.edited();
            // The space usually typed after the colon is already there.
            self.eat_space = true;
            return true;
        }
        if dactile::writing::is_caps_name(name) {
            let name = name.to_string();
            // `AHMET:` stays until the form is done and is replaced then; a
            // cancelled form leaves it as typed.
            self.buf.insert(":");
            self.edited();
            let mut form = self.character_form("", Some(lead));
            form.at = lead..self.buf.cursor;
            form.fields[0].value = name.clone();
            form.fields[0].touched = true;
            form.fields[1].value = self.doc.cast.suggest_alias(&name);
            self.popup = Popup::Form(form);
            return true;
        }
        false
    }

    /// Who probably speaks on the empty line at the cursor: shown faint,
    /// taken with Tab.
    fn ghost(&self) -> Option<String> {
        if !matches!(self.popup, Popup::None)
            || self.raw_now()
            || !self.at_blank_line()
            || !self.at_line_start()
        {
            return None;
        }
        dactile::writing::next_speaker(&self.doc, self.block()?)
    }

    fn at_blank_line(&self) -> bool {
        matches!(
            self.block().map(|b| &self.doc.blocks[b].kind),
            Some(Kind::Blank)
        )
    }

    pub fn paste(&mut self, s: &str) {
        if !self.editable() || s.is_empty() {
            return;
        }
        let block = self.block().unwrap_or(0);
        let text = if self.raw_now() {
            s.replace("\r\n", "\n")
        } else if self.at_line_start() {
            // Whole lines from a draft: names, directions and headings take
            // their shape.
            let (text, unknown) = dactile::writing::paste_lines(s, &self.doc.cast);
            if unknown > 0 {
                self.say(
                    &lang::f(
                        "{} satırda tanımsız ad var (AD: …); düz yazı olarak kaldı.",
                        "{} lines start with an unknown NAME:; left as text.",
                        &[&unknown],
                    ),
                    true,
                );
            }
            text
        } else {
            buffer::escape_pasted(s, self.doc.is_dialogue(block))
        };
        let nl = source::line_ending(&self.buf.text);
        let text = if nl == "\r\n" {
            text.replace('\n', "\r\n")
        } else {
            text
        };
        self.buf.insert(&text);
        self.edited();
        self.keep_visible();
    }

    fn copy(&mut self, cut: bool) {
        let Some(sel) = self.buf.selected_text().map(str::to_string) else {
            return;
        };
        self.copied = sel.clone();
        if let Some(c) = self.clipboard.as_mut() {
            let _ = c.set_text(sel);
        }
        if cut && self.editable() {
            self.buf.delete(0..0);
            self.edited();
        }
    }

    /// The `\\name` right before the cursor, if any.
    fn macro_before(&self) -> Option<String> {
        let t = &self.buf.text[..self.buf.cursor];
        let name: String = t
            .chars()
            .rev()
            .take_while(|c| dactile::tex::is_letter(*c))
            .collect();
        let name: String = name.chars().rev().collect();
        (!name.is_empty() && t[..t.len() - name.len()].ends_with('\\')).then_some(name)
    }

    /// A macro was just finished with a space: an unknown one at the start
    /// of a line asks for a new character; `\\xdo ` offers verbs.
    fn after_macro(&mut self, name: &str) {
        let Some(b) = self.block() else { return };
        match &self.doc.blocks[b].kind {
            Kind::Unknown { name: n } if n == name => {
                self.popup = Popup::Form(self.character_form(name, None));
            }
            Kind::Do { text, .. } if text.start == self.buf.cursor && text.is_empty() => {
                let verbs = complete::verbs(&self.doc, &self.buf.text);
                self.popup = Popup::Verbs(Verbs::new(verbs, self.buf.cursor));
            }
            _ => {}
        }
    }

    // ------------------------------------------------------------ popups

    /// Keys a popup takes; true when it took this one.
    fn popup_key(&mut self, k: KeyEvent) -> bool {
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        match &mut self.popup {
            Popup::None => false,
            Popup::Menu(m) => match k.code {
                KeyCode::Up => {
                    m.up();
                    true
                }
                KeyCode::Down => {
                    m.down();
                    true
                }
                KeyCode::Tab | KeyCode::Enter if !m.shown.is_empty() => {
                    let item = m.chosen().cloned();
                    let range = m.from - 1..self.buf.cursor;
                    self.popup = Popup::None;
                    if let Some(item) = item {
                        self.choose(item.action, range);
                    }
                    true
                }
                KeyCode::Esc => {
                    // A lone # is a plain one.
                    let lone =
                        m.hash && self.buf.text.get(m.from - 1..self.buf.cursor) == Some("#");
                    let at = m.from - 1;
                    self.popup = Popup::None;
                    if lone {
                        self.buf
                            .apply(&[dactile::tex::Edit::new(at..at + 1, "\\#")], at + 2);
                        self.raw_here = false;
                        self.edited();
                    }
                    true
                }
                _ => false,
            },
            Popup::Verbs(v) => match k.code {
                KeyCode::Up => {
                    v.sel = v.sel.saturating_sub(1);
                    true
                }
                KeyCode::Down => {
                    if v.sel + 1 < v.shown.len() {
                        v.sel += 1;
                    }
                    true
                }
                KeyCode::Tab | KeyCode::Enter if !v.shown.is_empty() => {
                    let verb = v.shown[v.sel].clone();
                    let range = v.from..self.buf.cursor;
                    self.popup = Popup::None;
                    self.buf.apply(
                        &[dactile::tex::Edit::new(range.clone(), verb.clone())],
                        range.start + verb.len(),
                    );
                    self.edited();
                    true
                }
                KeyCode::Esc => {
                    self.popup = Popup::None;
                    true
                }
                _ => false,
            },
            Popup::Search(m, q) => {
                match k.code {
                    KeyCode::Esc => self.popup = Popup::None,
                    KeyCode::Up => m.up(),
                    KeyCode::Down => m.down(),
                    KeyCode::Backspace => {
                        q.pop();
                        let q = q.clone();
                        m.filter(&q);
                    }
                    KeyCode::Enter | KeyCode::Tab => {
                        let item = m.chosen().cloned();
                        self.popup = Popup::None;
                        if let Some(item) = item
                            && self.editable()
                        {
                            let at = self.buf.cursor;
                            self.choose(item.action, at..at);
                        }
                    }
                    KeyCode::Char(c) if !ctrl => {
                        q.push(c);
                        let q = q.clone();
                        m.filter(&q);
                    }
                    _ => {}
                }
                true
            }
            Popup::Form(_) => {
                self.form_key(k);
                true
            }
            Popup::Cast(_) => {
                self.cast_key(k);
                true
            }
            Popup::Scenes(p) => {
                let units = shelf::units(&self.buf.text, &self.doc);
                if p.trash {
                    let n = self.trash.len();
                    if k.code != KeyCode::Char('x') {
                        p.confirm = false;
                    }
                    match k.code {
                        KeyCode::Esc => self.popup = Popup::None,
                        KeyCode::Char('g') if ctrl => self.popup = Popup::None,
                        KeyCode::Tab => p.trash = false,
                        KeyCode::Up => p.trash_sel = p.trash_sel.saturating_sub(1),
                        KeyCode::Down => p.trash_sel = (p.trash_sel + 1).min(n.saturating_sub(1)),
                        KeyCode::Char('r') if n > 0 => {
                            let i = p.trash_sel;
                            self.put_back(i);
                        }
                        KeyCode::Char('x') if n > 0 => {
                            if p.confirm {
                                let i = p.trash_sel;
                                p.confirm = false;
                                self.destroy(i);
                            } else {
                                p.confirm = true;
                            }
                        }
                        _ => {}
                    }
                    if let Popup::Scenes(p) = &mut self.popup {
                        p.trash_sel = p.trash_sel.min(self.trash.len().saturating_sub(1));
                    }
                    return true;
                }
                let n = units.len();
                match k.code {
                    KeyCode::Esc => self.popup = Popup::None,
                    KeyCode::Char('g') if ctrl => self.popup = Popup::None,
                    KeyCode::Tab => p.trash = true,
                    KeyCode::Up => p.sel = p.sel.saturating_sub(1),
                    KeyCode::Down => p.sel = (p.sel + 1).min(n.saturating_sub(1)),
                    KeyCode::Char(' ') => {
                        if let Some(u) = units.get(p.sel) {
                            if !self.marks.remove(&u.span.start) {
                                self.marks.insert(u.span.start);
                            }
                            p.sel = (p.sel + 1).min(n.saturating_sub(1));
                        }
                    }
                    KeyCode::Char('h') => {
                        let sel = p.sel;
                        self.hide_or_show(&units, sel);
                    }
                    KeyCode::Char('t') => {
                        let sel = p.sel;
                        self.throw_away(&units, sel);
                    }
                    KeyCode::Enter => {
                        let target = units.get(p.sel).map(|u| (u.span.start, u.hidden));
                        self.popup = Popup::None;
                        if let Some((at, hidden)) = target
                            && let Some(b) = self.doc.block_at(at)
                        {
                            self.go(at, false);
                            self.relayout();
                            // The first line after the heading.
                            if !hidden
                                && let Some(s) = self.lay.next_stop(self.doc.blocks[b].span.end)
                            {
                                self.go(s.byte, false);
                            }
                        }
                    }
                    _ => {}
                }
                if let Popup::Scenes(p) = &mut self.popup {
                    let n = shelf::units(&self.buf.text, &self.doc).len();
                    p.sel = p.sel.min(n.saturating_sub(1));
                }
                true
            }
            Popup::Split(p) => {
                let n = p.scenes.len();
                match k.code {
                    KeyCode::Esc => self.popup = Popup::None,
                    KeyCode::Up => p.sel = p.sel.saturating_sub(1),
                    KeyCode::Down => p.sel = (p.sel + 1).min(n.saturating_sub(1)),
                    KeyCode::Char(' ') => {
                        if let Some(&sc) = p.scenes.get(p.sel)
                            && !p.picked.remove(&sc)
                        {
                            p.picked.insert(sc);
                        }
                        p.sel = (p.sel + 1).min(n.saturating_sub(1));
                    }
                    KeyCode::Enter => {
                        let Popup::Split(p) = std::mem::replace(&mut self.popup, Popup::None)
                        else {
                            return true;
                        };
                        self.split_into(p);
                    }
                    _ => {}
                }
                true
            }
            Popup::Help(h) => {
                // Keys light up here instead of acting; Esc and F1 close.
                match k.code {
                    KeyCode::Esc | KeyCode::F(1) => self.popup = Popup::None,
                    _ => h.key(k, &self.keyboard),
                }
                true
            }
            Popup::Suggest(range, items, sel) => {
                match k.code {
                    KeyCode::Esc => self.popup = Popup::None,
                    KeyCode::Up => *sel = sel.saturating_sub(1),
                    KeyCode::Down => *sel = (*sel + 1).min(items.len().saturating_sub(1)),
                    KeyCode::Enter | KeyCode::Tab => {
                        let (range, choice) = (range.clone(), items[*sel].clone());
                        self.popup = Popup::None;
                        let word = self.buf.text[range.clone()].to_string();
                        if choice.starts_with('+') {
                            self.sidecar.add_word(&word);
                            let _ = self.sidecar.save(&self.path);
                            self.refresh_known();
                            self.say(
                                &lang::f(
                                    "«{}» bu oyunun sözlüğüne eklendi.",
                                    "«{}» added to this play's words.",
                                    &[&word],
                                ),
                                false,
                            );
                        } else if self.editable() {
                            let at = range.start;
                            self.buf.apply(
                                &[dactile::tex::Edit::new(range, choice.clone())],
                                at + choice.len(),
                            );
                            self.edited();
                        }
                    }
                    _ => {}
                }
                true
            }
        }
    }

    /// Keeps the menu and the verb list in step with what is typed.
    fn follow_popups(&mut self) {
        let cur = self.buf.cursor;
        match &mut self.popup {
            Popup::Menu(m) => {
                let q = self.buf.text.get(m.from..cur);
                match q {
                    Some(q) if cur >= m.from && q.chars().all(dactile::tex::is_letter) => {
                        let q = q.to_string();
                        m.filter(&q);
                    }
                    _ => self.popup = Popup::None,
                }
            }
            Popup::Verbs(v) => match self.buf.text.get(v.from..cur) {
                Some(q) if cur >= v.from && !q.contains(' ') => {
                    let q = q.to_string();
                    v.filter(&q);
                }
                _ => self.popup = Popup::None,
            },
            _ => {}
        }
    }

    /// Carries out a suggestion over `range` (the typed `\\query`, or the
    /// cursor when it came from the search window).
    fn choose(&mut self, action: Action, range: std::ops::Range<usize>) {
        match action {
            Action::Insert { text, cursor } => {
                let at = range.start;
                self.buf
                    .apply(&[dactile::tex::Edit::new(range, text.clone())], at + cursor);
                self.raw_here = false;
                self.edited();
                self.snap_forward_if_inside();
                if text.ends_with("do ") {
                    let verbs = complete::verbs(&self.doc, &self.buf.text);
                    self.popup = Popup::Verbs(Verbs::new(verbs, self.buf.cursor));
                }
            }
            Action::Scene | Action::Act | Action::NewCharacter => {
                // The typed `\\query` goes; the form writes the real thing.
                let at = range.start;
                if !range.is_empty() {
                    self.buf.apply(&[dactile::tex::Edit::new(range, "")], at);
                }
                self.raw_here = false;
                self.edited();
                let at = self.buf.cursor;
                self.popup = Popup::Form(match action {
                    Action::Scene => self.scene_form(at),
                    Action::Act => self.act_form(at),
                    _ => self.character_form("", Some(at)),
                });
            }
        }
    }

    fn scene_form(&self, at: usize) -> Form {
        Form {
            kind: FormKind::Scene,
            title: lang::l("yeni sahne", "new scene").into(),
            fields: vec![
                Field::text("Yer", "", complete::places(&self.doc)),
                Field::text("Zaman", "", complete::times(&self.doc)),
            ],
            focus: 0,
            at: at..at,
            error: None,
            confirmed: false,
        }
    }

    fn act_form(&self, at: usize) -> Form {
        Form {
            kind: FormKind::Act,
            title: lang::l("yeni perde", "new act").into(),
            fields: vec![Field::text("Başlık", "", Vec::new())],
            focus: 0,
            at: at..at,
            error: None,
            confirmed: false,
        }
    }

    /// `alias` is the one typed (`\\yv `), or empty from the menu, where
    /// `insert_at` is where `\\alias ` goes once the character exists.
    fn character_form(&self, alias: &str, insert_at: Option<usize>) -> Form {
        let mut alias_field = Field::text("Kısaltma", alias, Vec::new());
        alias_field.touched = !alias.is_empty();
        let at = insert_at.unwrap_or(usize::MAX);
        Form {
            kind: FormKind::Character,
            title: lang::l("yeni karakter", "new character").into(),
            fields: vec![
                Field::text("Ad", "", Vec::new()),
                alias_field,
                Field::text("Tam ad", "", Vec::new()),
                Field::text("Açıklama", "", Vec::new()),
                Field::flag("Kişiler tablosunda", true),
                Field::flag("Yıldızlı", false),
                Field::flag("Gizli (HERKES gibi)", false),
            ],
            focus: 0,
            at: at..at,
            error: None,
            confirmed: false,
        }
    }

    fn form_key(&mut self, k: KeyEvent) {
        let Popup::Form(form) = &mut self.popup else {
            return;
        };
        form.error = None;
        let n = form.fields.len();
        if form.kind == FormKind::Find {
            let (needle, with) = (
                form.get("Ara").to_string(),
                form.get("Değiştir").to_string(),
            );
            let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
            let alt = k.modifiers.contains(KeyModifiers::ALT);
            match k.code {
                KeyCode::Enter => {
                    self.find_next(&needle);
                    return;
                }
                KeyCode::Char('r') if ctrl => {
                    self.replace_one(&needle, &with);
                    return;
                }
                KeyCode::Char('r') if alt => {
                    self.replace_all(&needle, &with);
                    return;
                }
                _ => {}
            }
        }
        let Popup::Form(form) = &mut self.popup else {
            return;
        };
        match k.code {
            KeyCode::Esc => {
                self.popup = Popup::None;
                return;
            }
            KeyCode::Up | KeyCode::BackTab => form.focus = (form.focus + n - 1) % n,
            KeyCode::Down => form.focus = (form.focus + 1) % n,
            KeyCode::Tab => {
                if !form.complete() {
                    form.focus = (form.focus + 1) % n;
                }
            }
            KeyCode::Enter => {
                let form = match std::mem::replace(&mut self.popup, Popup::None) {
                    Popup::Form(f) => f,
                    _ => return,
                };
                if let Err(err) = self.submit(form) {
                    let (mut form, e) = *err;
                    form.error = Some(e);
                    self.popup = Popup::Form(form);
                }
                return;
            }
            KeyCode::Backspace => {
                let f = &mut form.fields[form.focus];
                if !f.toggle {
                    f.value.pop();
                    f.touched = true;
                }
            }
            KeyCode::Char(' ') if form.fields[form.focus].toggle => {
                let f = &mut form.fields[form.focus];
                f.value = if f.on() { "hayır" } else { "evet" }.into();
            }
            KeyCode::Char(c) => {
                let f = &mut form.fields[form.focus];
                if !f.toggle {
                    f.value.push(c);
                    f.touched = true;
                }
            }
            _ => {}
        }
        // The alias follows the name until it is edited by hand.
        if matches!(form.kind, FormKind::Character | FormKind::Split(_)) {
            let name = form.get("Ad").to_string();
            let cast = &self.doc.cast;
            if let Some(a) = form.field_mut("Kısaltma")
                && !a.touched
            {
                a.value = if name.is_empty() {
                    String::new()
                } else {
                    cast.suggest_alias(&name)
                };
            }
        }
    }

    /// Writes what a form says into the text; Err gives the form back with
    /// what is wrong.
    fn submit(&mut self, form: Form) -> Result<(), Box<(Form, String)>> {
        let nl = source::line_ending(&self.buf.text).to_string();
        match form.kind.clone() {
            FormKind::Scene => {
                let parts: Vec<String> = [form.get("Yer"), form.get("Zaman")]
                    .into_iter()
                    .filter(|s| !s.is_empty())
                    .map(dactile::tex::escape_text)
                    .collect();
                let hash = !info::has_option(&self.buf.text, "nohash");
                let text = match (hash, parts.is_empty()) {
                    (true, true) => "#sahne".to_string(),
                    (true, false) => format!("#sahne {}", parts.join(" — ")),
                    (false, true) => "\\sahne".to_string(),
                    (false, false) => format!("\\sahne[{}]", parts.join(" — ")),
                };
                self.write_line(form.at.start, &text, &nl);
            }
            FormKind::Act => {
                let title = dactile::tex::escape_text(form.get("Başlık"));
                let hash = !info::has_option(&self.buf.text, "nohash");
                let text = match (hash, title.is_empty()) {
                    (true, true) => "#perde".to_string(),
                    (true, false) => format!("#perde {title}"),
                    (false, true) => "\\perde".to_string(),
                    (false, false) => format!("\\perde[{title}]"),
                };
                self.write_line(form.at.start, &text, &nl);
            }
            FormKind::Find
            | FormKind::NewPlay
            | FormKind::OpenPath
            | FormKind::Settings
            | FormKind::Copy(_)
            | FormKind::Fork(_) => {}
            FormKind::Card(alias) => {
                let card = dactile::sidecar::Card {
                    age: form.get("Yaş").to_string(),
                    relations: form.get("İlişkiler").to_string(),
                    notes: form.get("Notlar").to_string(),
                };
                if card.is_empty() {
                    self.sidecar.cards.remove(&alias);
                } else {
                    self.sidecar.cards.insert(alias.clone(), card);
                }
                if let Err(e) = self.sidecar.save(&self.path) {
                    self.say(
                        &lang::f(
                            "Kart kaydedilemedi: {}",
                            "Couldn't save the card: {}",
                            &[&format!("{e:#}")],
                        ),
                        true,
                    );
                }
                self.popup = Popup::Cast(CastPanel {
                    sel: self.cast_index(&alias),
                    by_lines: false,
                });
            }
            FormKind::Info => {
                let mut t = self.buf.text.clone();
                for (cmd, label) in info::FIELDS {
                    let value = dactile::tex::escape_text(form.get(label));
                    if let Some(e) = info::set(&t, cmd, &value) {
                        t = dactile::tex::apply(&t, &[e]);
                    }
                }
                let fin = form
                    .fields
                    .iter()
                    .find(|f| f.label.starts_with("Final"))
                    .is_some_and(Field::on);
                if let Some(e) = info::set_final(&t, fin) {
                    t = dactile::tex::apply(&t, &[e]);
                }
                if t != self.buf.text && self.editable() {
                    // Every change is in the preamble, before the cursor.
                    let cursor = (self.buf.cursor as isize + t.len() as isize
                        - self.buf.text.len() as isize) as usize;
                    let all = 0..self.buf.text.len();
                    self.buf.apply(&[dactile::tex::Edit::new(all, t)], cursor);
                    self.edited();
                    self.say(
                        lang::l("Oyun bilgisi güncellendi.", "Cover details updated."),
                        false,
                    );
                }
            }
            FormKind::Merge(from) => {
                let into = form
                    .get("Şununla birleştir")
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .trim_start_matches('\\')
                    .to_string();
                if into == from || self.doc.cast.get(&into).is_none() {
                    return Err(Box::new((
                        form,
                        lang::l(
                            "Oyundaki bir karakterin kısaltmasını yaz (Tab listeyi gezer).",
                            "Type the alias of a character in the play (Tab goes through them).",
                        )
                        .into(),
                    )));
                }
                if !form.confirmed {
                    let (lines, dos) =
                        dcast::uses(&self.buf.text, self.doc.parts.body.clone(), &from);
                    let mut form = form;
                    form.confirmed = true;
                    return Err(Box::new((
                        form,
                        lang::f(
                            "\\{} → \\{}: {} yer geçer, \\{} silinir. ⏎ onaylar.",
                            "\\{} → \\{}: {} places move, \\{} goes. ⏎ confirms.",
                            &[&from, &into, &(lines + dos), &from],
                        ),
                    )));
                }
                if !self.editable() {
                    return Ok(());
                }
                let edits = dcast::merge(&self.buf.text, &self.doc.cast, &from, &into);
                let cursor = self.buf.cursor.min(self.doc.parts.body.start);
                self.buf.apply(&edits, cursor);
                // The card: what was known about both.
                if let Some(old) = self.sidecar.cards.remove(&from) {
                    let card = self.sidecar.cards.entry(into.clone()).or_default();
                    for (mine, theirs) in [
                        (&mut card.age, old.age),
                        (&mut card.relations, old.relations),
                        (&mut card.notes, old.notes),
                    ] {
                        if mine.is_empty() {
                            *mine = theirs;
                        } else if !theirs.is_empty() {
                            mine.push_str(&format!(" / {theirs}"));
                        }
                    }
                    let _ = self.sidecar.save(&self.path);
                }
                self.edited();
                self.say(
                    &lang::f(
                        "\\{} artık \\{} (^Z geri alır).",
                        "\\{} is now \\{} (^Z undoes).",
                        &[&from, &into],
                    ),
                    false,
                );
            }
            FormKind::Split(from) => {
                let spec = match self.spec_from(&form, None) {
                    Ok(s) => s,
                    Err(e) => return Err(Box::new((form, e))),
                };
                if self.doc.cast.alias_taken(&spec.alias) {
                    let alias = spec.alias.clone();
                    return Err(Box::new((
                        form,
                        lang::f(
                            "\\{} kullanılıyor; başka bir kısaltma seç.",
                            "\\{} is taken; choose another alias.",
                            &[&alias],
                        ),
                    )));
                }
                let scenes: Vec<usize> = self
                    .doc
                    .outline
                    .scenes
                    .iter()
                    .enumerate()
                    .filter(|(_, s)| s.cast.contains(&from))
                    .map(|(i, _)| i)
                    .collect();
                self.popup = Popup::Split(SplitPanel {
                    from,
                    spec,
                    scenes,
                    picked: Default::default(),
                    sel: 0,
                });
            }
            FormKind::EditCharacter(old) => {
                let spec = match self.spec_from(&form, Some(&old)) {
                    Ok(s) => s,
                    Err(e) => return Err(Box::new((form, e))),
                };
                let renaming = spec.alias != old;
                if renaming && !form.confirmed {
                    let n = dcast::rename(&self.buf.text, &self.doc.cast, &old, &spec.alias).len();
                    let mut form = form;
                    form.confirmed = true;
                    return Err(Box::new((
                        form,
                        format!(
                            "\\{old} → \\{}: {n} yerde değişecek. Onay için tekrar ⏎.",
                            spec.alias
                        ),
                    )));
                }
                // The declaration first, then the alias everywhere: one undo step.
                let mut keep = spec.clone();
                keep.alias = old.clone();
                let mut t = self.buf.text.clone();
                if let Some(e) = dcast::update(&self.doc.cast, &old, &keep) {
                    t = dactile::tex::apply(&t, &[e]);
                }
                if renaming {
                    let c = dcast::Cast::of_play(&t);
                    t = dactile::tex::apply(&t, &dcast::rename(&t, &c, &old, &spec.alias));
                    self.sidecar.rename(&old, &spec.alias);
                    let _ = self.sidecar.save(&self.path);
                }
                if t != self.buf.text && self.editable() {
                    let at = self.buf.cursor;
                    let all = 0..self.buf.text.len();
                    // The cursor stays at the same line: count from the end of the text.
                    let from_end = self.buf.text.len() - at;
                    self.buf.apply(
                        &[dactile::tex::Edit::new(all, t.clone())],
                        t.len().saturating_sub(from_end),
                    );
                    self.edited();
                    self.refresh_known();
                }
                self.popup = Popup::Cast(CastPanel {
                    sel: self.cast_index(&spec.alias),
                    by_lines: false,
                });
            }
            FormKind::Character => {
                let script = form.get("Ad").to_string();
                let alias = form.get("Kısaltma").to_string();
                if script.is_empty() {
                    return Err(Box::new((
                        form,
                        lang::l("Ad boş olamaz.", "The name can't be empty.").into(),
                    )));
                }
                if alias.is_empty() || !alias.chars().all(|c| c.is_ascii_alphabetic()) {
                    return Err(Box::new((
                        form,
                        lang::l(
                            "Kısaltma yalnız İngilizce harflerden oluşmalı (ş, ç… olmadan).",
                            "The alias takes plain letters a–z only.",
                        )
                        .into(),
                    )));
                }
                let typed_here = form.at.start == usize::MAX;
                if self.doc.cast.alias_taken(&alias) {
                    return Err(Box::new((
                        form,
                        format!(
                            "\\{alias} kullanılıyor ya da bir LaTeX komutu; başka bir kısaltma seç."
                        ),
                    )));
                }
                let hidden = form
                    .fields
                    .iter()
                    .find(|f| f.label.starts_with("Gizli"))
                    .is_some_and(Field::on);
                let in_table = form
                    .fields
                    .iter()
                    .find(|f| f.label.starts_with("Kişiler"))
                    .is_some_and(Field::on);
                let star = form
                    .fields
                    .iter()
                    .find(|f| f.label == "Yıldızlı")
                    .is_some_and(Field::on);
                let spec = Spec {
                    alias: alias.clone(),
                    script: dactile::tex::escape_text(&script),
                    full: dactile::tex::escape_text(form.get("Tam ad")),
                    desc: dactile::tex::escape_text(form.get("Açıklama")),
                    star,
                    kind: if hidden {
                        dcast::Kind::Hidden
                    } else if in_table {
                        dcast::Kind::Cast
                    } else {
                        dcast::Kind::Plain
                    },
                };
                let decl = dcast::add(&self.buf.text, &self.doc.cast, &spec, None);
                let shift = decl.text.len();
                let mut edits = vec![decl.clone()];
                let mut cursor = self.buf.cursor;
                if !typed_here {
                    let at = form.at.start;
                    let use_text = format!("\\{alias} ");
                    cursor = at + use_text.len();
                    // Over what was typed for it (`AHMET:`), if anything.
                    edits.push(dactile::tex::Edit::new(form.at.clone(), use_text));
                }
                if decl.range.start <= cursor {
                    cursor += shift;
                }
                self.buf.apply(&edits, cursor);
                self.raw_here = false;
                self.edited();
                self.say(&format!("{} eklendi: \\{alias}", spec.script), false);
            }
        }
        Ok(())
    }

    /// The character a form describes. `old` is the alias being edited.
    fn spec_from(&self, form: &Form, old: Option<&str>) -> Result<Spec, String> {
        let script = form.get("Ad").to_string();
        let alias = form.get("Kısaltma").to_string();
        if script.is_empty() {
            return Err(lang::l("Ad boş olamaz.", "The name can't be empty.").into());
        }
        if alias.is_empty() || !alias.chars().all(|c| c.is_ascii_alphabetic()) {
            return Err(lang::l(
                "Kısaltma yalnız İngilizce harflerden oluşmalı (ş, ç… olmadan).",
                "The alias takes plain letters a–z only.",
            )
            .into());
        }
        if Some(alias.as_str()) != old && self.doc.cast.alias_taken(&alias) {
            return Err(format!(
                "\\{alias} kullanılıyor ya da bir LaTeX komutu; başka bir kısaltma seç."
            ));
        }
        let flag = |label: &str| {
            form.fields
                .iter()
                .find(|f| f.label.starts_with(label))
                .is_some_and(Field::on)
        };
        let member = old
            .and_then(|o| self.doc.cast.get(o))
            .is_some_and(|c| c.kind == dcast::Kind::Member);
        Ok(Spec {
            alias,
            script: dactile::tex::escape_text(&script),
            full: dactile::tex::escape_text(form.get("Tam ad")),
            desc: dactile::tex::escape_text(form.get("Açıklama")),
            star: flag("Yıldızlı"),
            kind: if member {
                dcast::Kind::Member
            } else if flag("Gizli") {
                dcast::Kind::Hidden
            } else if flag("Kişiler") {
                dcast::Kind::Cast
            } else {
                dcast::Kind::Plain
            },
        })
    }

    /// Where a character is in the cast panel.
    fn cast_index(&self, alias: &str) -> usize {
        self.cast_entries(false)
            .iter()
            .position(
                |e| matches!(e, Entry::Character(i) if self.doc.cast.characters[*i].alias == alias),
            )
            .unwrap_or(0)
    }

    // ------------------------------------------------------------ find

    /// The next match of `needle` after the cursor (wrapping), ignoring case
    /// the Turkish way.
    fn find_from(&self, needle: &str, from: usize) -> Option<std::ops::Range<usize>> {
        if needle.is_empty() {
            return None;
        }
        let t = &self.buf.text;
        let n = dcast::lower_tr(needle);
        let n: Vec<char> = n.chars().collect();
        let starts: Vec<usize> = t.char_indices().map(|(i, _)| i).collect();
        let try_at = |i: usize| -> Option<usize> {
            let mut end = i;
            for &c in &n {
                let ch = t[end..].chars().next()?;
                let low: Vec<char> = dcast::lower_tr(&ch.to_string()).chars().collect();
                if low.first() != Some(&c) {
                    return None;
                }
                end += ch.len_utf8();
            }
            Some(end)
        };
        let body = self.doc.parts.body.clone();
        let after = starts.iter().filter(|&&i| i >= from && i < body.end);
        let before = starts.iter().filter(|&&i| i >= body.start && i < from);
        after.chain(before).find_map(|&i| try_at(i).map(|e| i..e))
    }

    fn find_next(&mut self, needle: &str) {
        let from = self.buf.selection().map_or(self.buf.cursor, |s| s.end);
        match self.find_from(needle, from) {
            Some(r) => {
                // Show the line raw when the match is inside a macro.
                self.buf.anchor = Some(r.start);
                self.buf.cursor = r.end;
                if !self.lay.is_stop(r.start) || !self.lay.is_stop(r.end) {
                    self.raw_here = true;
                    self.relayout();
                }
                self.keep_visible();
            }
            None => self.say(
                &lang::f("«{}» bulunamadı.", "«{}» not found.", &[&needle]),
                false,
            ),
        }
    }

    fn replace_one(&mut self, needle: &str, with: &str) {
        let matches = self
            .buf
            .selected_text()
            .is_some_and(|s| dcast::lower_tr(s) == dcast::lower_tr(needle));
        if matches && self.editable() {
            let r = self.buf.selection().unwrap();
            let at = r.start;
            self.buf.apply(
                &[dactile::tex::Edit::new(r, with.to_string())],
                at + with.len(),
            );
            self.edited();
        }
        self.find_next(needle);
    }

    fn replace_all(&mut self, needle: &str, with: &str) {
        if needle.is_empty() || !self.editable() {
            return;
        }
        let mut edits = Vec::new();
        let mut at = self.doc.parts.body.start;
        while let Some(r) = self.find_from(needle, at) {
            if r.start < at
                || edits
                    .iter()
                    .any(|e: &dactile::tex::Edit| e.range.start == r.start)
            {
                break;
            }
            at = r.end;
            edits.push(dactile::tex::Edit::new(r, with.to_string()));
        }
        let n = edits.len();
        if n > 0 {
            let cursor = self.buf.cursor.min(self.buf.text.len());
            self.buf.apply(&edits, cursor.min(edits[0].range.start));
            self.edited();
        }
        self.say(
            &lang::f(
                "{} yer değişti (^Z geri alır).",
                "{} replaced (^Z undoes).",
                &[&n],
            ),
            false,
        );
    }

    /// Puts `text` on a line of its own at `at` and the cursor on the line after.
    fn write_line(&mut self, at: usize, text: &str, nl: &str) {
        let line_start = self.buf.text[..at].rfind('\n').map_or(0, |i| i + 1);
        let line_end = self.buf.text[at..]
            .find(['\r', '\n'])
            .map_or(self.buf.text.len(), |i| at + i);
        let alone = self.buf.text[line_start..line_end].trim().is_empty();
        let (range, insert) = if alone {
            (line_start..line_end, format!("{text}{nl}"))
        } else {
            (at..at, format!("{nl}{text}{nl}"))
        };
        let cursor = range.start + insert.len();
        self.buf
            .apply(&[dactile::tex::Edit::new(range, insert)], cursor);
        self.raw_here = false;
        self.edited();
        self.snap_forward_if_inside();
    }

    // ------------------------------------------------------------ cast panel

    fn wpm(&self) -> u32 {
        self.sidecar.wpm.unwrap_or(self.default_wpm)
    }

    fn cast_entries(&self, by_lines: bool) -> Vec<Entry> {
        let st = panels::stats(&self.doc, &self.buf.text);
        panels::entries(&self.doc, &st, by_lines)
    }

    fn cast_key(&mut self, k: KeyEvent) {
        let Popup::Cast(p) = &mut self.popup else {
            return;
        };
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        let alt = k.modifiers.contains(KeyModifiers::ALT);
        let (sel, by_lines) = (p.sel, p.by_lines);
        let list = self.cast_entries(by_lines);
        let n = list.len();
        let chosen = list.get(sel).copied();
        let alias = match chosen {
            Some(Entry::Character(i)) => Some(self.doc.cast.characters[i].alias.clone()),
            _ => None,
        };
        match k.code {
            KeyCode::Esc => self.popup = Popup::None,
            KeyCode::Char('o') if ctrl => self.popup = Popup::None,
            KeyCode::Up | KeyCode::Down if alt && !by_lines => {
                // Move the row (a character of the table, or a whole group).
                let row = match chosen {
                    Some(Entry::Character(i)) => self
                        .doc
                        .cast
                        .rows()
                        .iter()
                        .position(|r| *r == dactile::cast::Row::Character(i)),
                    Some(Entry::Group(g)) => self
                        .doc
                        .cast
                        .rows()
                        .iter()
                        .position(|r| *r == dactile::cast::Row::Group(g)),
                    None => None,
                };
                let up = k.code == KeyCode::Up;
                match row.and_then(|r| dcast::move_row(&self.buf.text, &self.doc.cast, r, up)) {
                    Some(edits) if self.editable() => {
                        // Two declarations swap places: lengths before the cursor don't change.
                        let cursor = self.buf.cursor;
                        self.buf.apply(&edits, cursor);
                        self.edited();
                        // The selection follows the moved row.
                        let new_list = self.cast_entries(false);
                        let target = alias.as_ref().and_then(|a| {
                            new_list.iter().position(|e| matches!(e, Entry::Character(i) if self.doc.cast.characters[*i].alias == *a))
                        });
                        if let Popup::Cast(p) = &mut self.popup {
                            p.sel =
                                target.unwrap_or(if up { sel.saturating_sub(1) } else { sel + 1 });
                        }
                    }
                    Some(_) => {}
                    None => self.say(
                        lang::l(
                            "Yalnız kişiler tablosundaki satırlar taşınır.",
                            "Only rows of the cast table move.",
                        ),
                        false,
                    ),
                }
            }
            KeyCode::Up => {
                if let Popup::Cast(p) = &mut self.popup {
                    p.sel = sel.saturating_sub(1);
                }
            }
            KeyCode::Down => {
                if let Popup::Cast(p) = &mut self.popup {
                    p.sel = (sel + 1).min(n.saturating_sub(1));
                }
            }
            KeyCode::Char('s') => {
                if let Popup::Cast(p) = &mut self.popup {
                    p.by_lines = !by_lines;
                    p.sel = 0;
                }
            }
            KeyCode::Char('n') => self.popup = Popup::Form(self.character_form("", None)),
            KeyCode::Char('m') => {
                if let Some(a) = alias {
                    let others: Vec<String> = self
                        .doc
                        .cast
                        .characters
                        .iter()
                        .filter(|c| c.alias != a)
                        .map(|c| format!("{} — {}", c.alias, c.script))
                        .collect();
                    self.popup = Popup::Form(Form {
                        kind: FormKind::Merge(a.clone()),
                        title: lang::f("\\{} birleştir", "merge \\{}", &[&a]),
                        fields: vec![Field::text("Şununla birleştir", "", others)],
                        focus: 0,
                        at: 0..0,
                        error: None,
                        confirmed: false,
                    });
                }
            }
            KeyCode::Char('b') => {
                if let Some(a) = alias {
                    let mut form = self.character_form("", None);
                    form.kind = FormKind::Split(a.clone());
                    form.title = lang::f(
                        "\\{} böl: yeni karakter",
                        "split \\{}: the new character",
                        &[&a],
                    );
                    self.popup = Popup::Form(form);
                }
            }
            KeyCode::Enter => {
                if let Some(a) = alias {
                    self.popup = Popup::Form(self.card_form(&a));
                }
            }
            KeyCode::Char('e') => {
                if let Some(a) = alias {
                    self.popup = Popup::Form(self.edit_form(&a));
                }
            }
            KeyCode::Char('d') => {
                if let Some(a) = alias {
                    let (lines, dos) = dcast::uses(&self.buf.text, self.doc.parts.body.clone(), &a);
                    if lines + dos > 0 {
                        self.say(
                            &format!(
                                "\\{a} oyunda {} kez geçiyor; önce o satırları değiştir.",
                                lines + dos
                            ),
                            true,
                        );
                    } else if self.editable()
                        && let Some(e) = dcast::remove(&self.buf.text, &self.doc.cast, &a)
                    {
                        let cursor = if e.range.end <= self.buf.cursor {
                            self.buf.cursor - e.range.len()
                        } else {
                            self.buf.cursor
                        };
                        self.buf.apply(&[e], cursor);
                        self.edited();
                        self.sidecar.cards.remove(&a);
                        let _ = self.sidecar.save(&self.path);
                        self.say(
                            &lang::f(
                                "\\{} silindi (^Z geri alır).",
                                "\\{} deleted (^Z undoes).",
                                &[&a],
                            ),
                            false,
                        );
                        if let Popup::Cast(p) = &mut self.popup {
                            p.sel = sel.min(n.saturating_sub(2));
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// The new character takes the picked scenes' lines of the old one.
    fn split_into(&mut self, p: SplitPanel) {
        if p.picked.is_empty() {
            self.say(
                lang::l(
                    "Hiç sahne seçilmedi; bir şey değişmedi.",
                    "No scene chosen; nothing changed.",
                ),
                false,
            );
            return;
        }
        if !self.editable() {
            return;
        }
        let base = self.doc.body.start;
        let ranges: Vec<std::ops::Range<usize>> = p
            .picked
            .iter()
            .map(|&i| {
                let sc = &self.doc.outline.scenes[i];
                self.doc.blocks[sc.block + base].span.start
                    ..self.doc.blocks[sc.end - 1 + base].span.end
            })
            .collect();
        let mut edits = dcast::split(&self.buf.text, &p.from, &p.spec.alias, &ranges);
        let moved = edits.len();
        edits.push(dcast::add(&self.buf.text, &self.doc.cast, &p.spec, None));
        let cursor = self.buf.cursor.min(self.doc.parts.body.start);
        self.buf.apply(&edits, cursor);
        self.edited();
        self.say(
            &lang::f(
                "{} eklendi; {} sahnede {} yer → \\{} (^Z geri alır).",
                "{} added; {} places in {} scenes moved to \\{} (^Z undoes).",
                &[&p.spec.script, &p.picked.len(), &moved, &p.spec.alias],
            ),
            false,
        );
    }

    // ------------------------------------------------------------ putting away

    /// The marked units, or the chosen one; a scene inside a chosen act goes
    /// with the act.
    fn targets(&self, units: &[shelf::Unit], sel: usize) -> Vec<shelf::Unit> {
        let mut out: Vec<shelf::Unit> = if self.marks.is_empty() {
            units.get(sel).cloned().into_iter().collect()
        } else {
            units
                .iter()
                .filter(|u| self.marks.contains(&u.span.start))
                .cloned()
                .collect()
        };
        let all = out.clone();
        out.retain(|u| {
            !all.iter().any(|o| {
                o.span != u.span && o.span.start <= u.span.start && u.span.end <= o.span.end
            })
        });
        out
    }

    /// Hides the chosen acts and scenes; when they all are hidden already,
    /// brings them back.
    fn hide_or_show(&mut self, units: &[shelf::Unit], sel: usize) {
        if !self.editable() {
            return;
        }
        let targets = self.targets(units, sel);
        if targets.is_empty() {
            return;
        }
        let show = targets.iter().all(|u| u.hidden);
        let edits: Vec<dactile::tex::Edit> = targets
            .iter()
            .filter(|u| u.hidden == show)
            .map(|u| {
                if show {
                    shelf::unhide(&self.buf.text, u.span.clone())
                } else {
                    shelf::hide(&self.buf.text, u.span.clone())
                }
            })
            .collect();
        let n = edits.len();
        let cursor = self.buf.cursor;
        self.buf.apply(&edits, cursor);
        self.raw_here = false;
        self.edited();
        self.say(
            &if show {
                lang::f("{} parça geri geldi.", "{} brought back.", &[&n])
            } else {
                lang::f(
                    "{} parça gizlendi: PDF'te yok, h ile geri gelir.",
                    "{} hidden: out of the PDF, h brings them back.",
                    &[&n],
                )
            },
            false,
        );
    }

    /// Moves the chosen acts and scenes to the play's trash.
    fn throw_away(&mut self, units: &[shelf::Unit], sel: usize) {
        if !self.editable() {
            return;
        }
        let targets = self.targets(units, sel);
        if targets.is_empty() {
            return;
        }
        let when = chrono::Local::now().format("%Y-%m-%d %H:%M").to_string();
        let text = self.buf.text.clone();
        let mut items: Vec<shelf::Trashed> = Vec::new();
        let mut edits = Vec::new();
        for u in &targets {
            let (after, before) = shelf::neighbours(&text, u.span.clone());
            items.push(shelf::Trashed {
                when: when.clone(),
                what: if u.hidden {
                    format!("{} ({})", u.name(), lang::l("gizliydi", "was hidden"))
                } else {
                    u.name()
                },
                after,
                before,
                text: text[u.span.clone()].to_string(),
            });
            edits.push(shelf::cut(&text, u.span.clone()));
        }
        let path = shelf::trash_path(&self.path);
        let mut trash = shelf::load(&path);
        trash.extend(items);
        if let Err(e) = shelf::save(&path, &trash) {
            self.say(&format!("{e:#}"), true);
            return;
        }
        self.trash = trash;
        let first = targets.iter().map(|u| u.span.start).min().unwrap_or(0);
        let cursor = self.buf.cursor.min(first);
        self.buf.apply(&edits, cursor);
        self.raw_here = false;
        self.edited();
        self.say(
            &lang::f(
                "{} parça çöp kutusunda (^G, Tab).",
                "{} in the trash (^G, Tab).",
                &[&targets.len()],
            ),
            false,
        );
    }

    /// Puts a trashed piece back where it was.
    fn put_back(&mut self, i: usize) {
        if !self.editable() || i >= self.trash.len() {
            return;
        }
        let t = self.trash[i].clone();
        let e = shelf::restore(&self.buf.text, &self.doc, &t);
        let at = e.range.start + e.text.len();
        self.buf.apply(&[e], at);
        let mut trash = self.trash.clone();
        trash.remove(i);
        if let Err(e) = shelf::save(&shelf::trash_path(&self.path), &trash) {
            self.say(&format!("{e:#}"), true);
            return;
        }
        self.trash = trash;
        self.edited();
        self.say(
            &lang::f("{} yerine kondu.", "{} put back.", &[&t.what]),
            false,
        );
    }

    /// Destroys a trashed piece; a copy goes to the play's history first.
    fn destroy(&mut self, i: usize) {
        if i >= self.trash.len() {
            return;
        }
        let t = self.trash[i].clone();
        let kept = std::fs::create_dir_all(&self.history).and_then(|_| {
            use std::io::Write;
            let mut f = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(self.history.join("destroyed.txt"))?;
            writeln!(
                f,
                "%%% {} · {} (yok edildi {})",
                t.what,
                t.when,
                chrono::Local::now().format("%Y-%m-%d %H:%M")
            )?;
            writeln!(f, "{}\n", t.text)
        });
        if let Err(e) = kept {
            self.say(&format!("{e:#}"), true);
            return;
        }
        let mut trash = self.trash.clone();
        trash.remove(i);
        if let Err(e) = shelf::save(&shelf::trash_path(&self.path), &trash) {
            self.say(&format!("{e:#}"), true);
            return;
        }
        self.trash = trash;
        self.say(
            &lang::f("{} yok edildi.", "{} destroyed.", &[&t.what]),
            false,
        );
    }

    fn edit_form(&self, alias: &str) -> Form {
        let c = self.doc.cast.get(alias).cloned();
        let c = c.as_ref();
        let mut f = self.character_form(alias, None);
        f.kind = FormKind::EditCharacter(alias.to_string());
        f.title = lang::f("\\{} düzenle", "edit \\{}", &[&alias]);
        let set = |f: &mut Form, label: &str, v: String| {
            if let Some(x) = f.field_mut(label) {
                x.value = v;
                x.touched = true;
            }
        };
        if let Some(c) = c {
            let plain = |v: Option<&String>| dactile::tex::unescape_text(v.map_or("", |s| s));
            set(&mut f, "Ad", plain(Some(&c.script)));
            set(&mut f, "Tam ad", plain(c.full.as_ref()));
            set(&mut f, "Açıklama", plain(c.desc.as_ref()));
            let on = |b: bool| if b { "evet" } else { "hayır" }.to_string();
            set(
                &mut f,
                "Kişiler tablosunda",
                on(c.kind == dcast::Kind::Cast),
            );
            set(&mut f, "Yıldızlı", on(c.star));
            set(
                &mut f,
                "Gizli (HERKES gibi)",
                on(c.kind == dcast::Kind::Hidden),
            );
            if c.kind == dcast::Kind::Member {
                // A group member stays one; its table and hidden flags don't apply.
                f.fields
                    .retain(|x| !x.label.starts_with("Kişiler") && !x.label.starts_with("Gizli"));
                f.fields.retain(|x| x.label != "Açıklama");
            }
        }
        f
    }

    fn card_form(&self, alias: &str) -> Form {
        let card = self.sidecar.cards.get(alias).cloned().unwrap_or_default();
        let name = self
            .doc
            .cast
            .get(alias)
            .map_or(alias.to_string(), |c| c.script.clone());
        Form {
            kind: FormKind::Card(alias.to_string()),
            title: lang::f(
                "{} — kart (PDF'e girmez)",
                "{} — card (not printed)",
                &[&name],
            ),
            fields: vec![
                Field::text("Yaş", &card.age, Vec::new()),
                Field::text("İlişkiler", &card.relations, Vec::new()),
                Field::text("Notlar", &card.notes, Vec::new()),
            ],
            focus: 0,
            at: 0..0,
            error: None,
            confirmed: false,
        }
    }

    fn info_form(&self) -> Form {
        let mut fields: Vec<Field> = info::FIELDS
            .iter()
            .map(|(cmd, label)| {
                let v = info::get(&self.buf.text, cmd).unwrap_or_default();
                Field::text(label, &dactile::tex::unescape_text(&v), Vec::new())
            })
            .collect();
        fields.push(Field::flag("Final (baskı)", info::is_final(&self.buf.text)));
        Form {
            kind: FormKind::Info,
            title: lang::l("oyun bilgisi", "cover details").into(),
            fields,
            focus: 0,
            at: 0..0,
            error: None,
            confirmed: false,
        }
    }

    // ------------------------------------------------------------ spelling

    fn refresh_known(&mut self) {
        self.known = spell::cast_words(&self.doc.cast)
            .into_iter()
            .chain(self.sidecar.words.iter().cloned())
            .map(|w| dcast::lower_tr(&w))
            .collect();
        self.spell_cache.clear();
    }

    fn word_ok(&mut self, w: &str) -> bool {
        let Some(sp) = &self.speller else { return true };
        if let Some(&ok) = self.spell_cache.get(w) {
            return ok;
        }
        let lower = dcast::lower_tr(w);
        let stem = lower.split(['\'', '’']).next().unwrap_or("").to_string();
        let ok = self.known.contains(&lower) || self.known.contains(&stem) || sp.check(w);
        self.spell_cache.insert(w.to_string(), ok);
        ok
    }

    /// The misspelt words of the blocks on screen.
    fn mark_misspelt(&mut self) {
        self.misspelt.clear();
        if self.speller.is_none() {
            return;
        }
        let h = self.text_height();
        let mut blocks: Vec<usize> = self
            .lay
            .rows
            .iter()
            .skip(self.scroll)
            .take(h)
            .filter_map(|r| r.block)
            .collect();
        blocks.dedup();
        let text = self.buf.text.clone();
        for b in blocks {
            for r in spell::words(&text, &self.doc, b) {
                if !self.word_ok(&text[r.clone()]) {
                    self.misspelt.push(r);
                }
            }
        }
    }

    fn suggest(&mut self) {
        if self.speller.is_none() {
            let why = if self.speller_rx.is_some() {
                lang::l(
                    "Sözlük yükleniyor, birazdan.",
                    "The dictionary is loading, a moment.",
                )
            } else {
                lang::l(
                    "Türkçe sözlük bulunamadı (tr_TR.aff/.dic): paru -S hunspell-tr ile kurulabilir.",
                    "No Turkish dictionary (tr_TR.aff/.dic): install hunspell-tr.",
                )
            };
            self.say(why, true);
            return;
        }
        let cur = self.buf.cursor;
        let Some(b) = self.block() else { return };
        let words = spell::words(&self.buf.text, &self.doc, b);
        let Some(range) = words.into_iter().find(|r| r.start <= cur && cur <= r.end) else {
            self.say(
                lang::l(
                    "İmleç bir kelimenin üstünde değil.",
                    "The cursor isn't on a word.",
                ),
                false,
            );
            return;
        };
        let word = self.buf.text[range.clone()].to_string();
        let mut items = self
            .speller
            .as_ref()
            .map(|s| s.suggest(&word))
            .unwrap_or_default();
        items.push(lang::f(
            "+ «{}» oyunun sözlüğüne",
            "+ «{}» to the play's words",
            &[&word],
        ));
        self.popup = Popup::Suggest(range, items, 0);
    }

    // ------------------------------------------------------------ mouse

    pub fn mouse(&mut self, m: MouseEvent) {
        // On the help screen the mouse points at keys.
        if let Popup::Help(h) = &mut self.popup {
            if matches!(m.kind, MouseEventKind::Moved | MouseEventKind::Down(_)) {
                h.point(m.column, m.row, &self.keyboard);
            }
            return;
        }
        match m.kind {
            MouseEventKind::ScrollUp => self.scroll = self.scroll.saturating_sub(3),
            MouseEventKind::ScrollDown => {
                let max = self.lay.rows.len().saturating_sub(self.text_height() / 2);
                self.scroll = (self.scroll + 3).min(max);
            }
            // In the margin map, a right click marks an act or scene for ^G.
            MouseEventKind::Down(MouseButton::Right) => {
                if m.column < self.sheet().x
                    && let Some(&(_, at)) = self.map_hits.iter().find(|(y, _)| *y == m.row)
                    && shelf::units(&self.buf.text, &self.doc)
                        .iter()
                        .any(|u| u.span.start == at)
                {
                    if !self.marks.remove(&at) {
                        self.marks.insert(at);
                    }
                    let n = self.marks.len();
                    self.say(
                        &lang::f(
                            "{} işaretli: ^G'de h gizler, t çöpe atar.",
                            "{} marked: in ^G, h hides, t throws away.",
                            &[&n],
                        ),
                        false,
                    );
                }
            }
            MouseEventKind::Down(MouseButton::Left) | MouseEventKind::Drag(MouseButton::Left) => {
                if !matches!(self.popup, Popup::None) || m.row < self.text_top {
                    return;
                }
                // A heading in the margin map: go there.
                if m.column < self.sheet().x {
                    if let Some(&(_, at)) = self.map_hits.iter().find(|(y, _)| *y == m.row)
                        && matches!(m.kind, MouseEventKind::Down(_))
                    {
                        self.go(at, false);
                        self.snap_forward();
                        self.scroll = self
                            .lay
                            .stop_at(at)
                            .map_or(self.scroll, |s| s.row.saturating_sub(2));
                    }
                    return;
                }
                let row = self.scroll + (m.row - self.text_top) as usize;
                let col = m.column.saturating_sub(self.sheet().text_x);
                let hit = (row..self.lay.rows.len())
                    .chain((0..row).rev())
                    .find_map(|r| self.lay.stop_near(r, col));
                if let Some(s) = hit {
                    let drag = matches!(m.kind, MouseEventKind::Drag(_));
                    self.go(s.byte, drag);
                }
            }
            _ => {}
        }
    }

    // ------------------------------------------------------------ saving

    pub fn tick(&mut self) {
        if let Some(rx) = &self.speller_rx
            && let Ok(result) = rx.try_recv()
        {
            self.speller_rx = None;
            match result {
                Ok(sp) => {
                    self.speller = Some(sp);
                    self.spell_cache.clear();
                }
                Err(e) => self.say(
                    &lang::f(
                        "Sözlük yüklenemedi: {}",
                        "Couldn't load the dictionary: {}",
                        &[&format!("{e:#}")],
                    ),
                    true,
                ),
            }
        }
        if self.checked.elapsed() >= CHECK_EVERY {
            self.checked = Instant::now();
            self.check_disk();
        }
        let due = self.buf.dirty
            && self
                .buf
                .changed_at
                .is_some_and(|t| t.elapsed() >= AUTOSAVE_AFTER);
        if due && !self.conflict && !self.read_only {
            self.save(false);
        }
    }

    /// Picks up changes made to the file elsewhere (another editor, a sync).
    fn check_disk(&mut self) {
        let now = file::modified(&self.path);
        if now.is_none() || now == self.disk_time {
            return;
        }
        if self.buf.dirty {
            self.conflict = true;
            self.say(
                lang::l(
                    "Dosya başka yerde değişti. ^S senin halini yazar; ^Q'dan önce karar ver.",
                    "The file changed elsewhere. ^S writes yours over it; decide before ^Q.",
                ),
                true,
            );
            return;
        }
        if let Ok(text) = file::read(&self.path)
            && text != self.buf.text
        {
            self.buf.reload(text);
            self.raw_here = false;
            self.relayout();
            self.snap_back();
            self.say(
                lang::l(
                    "Dosya başka yerde değişti; yeniden yüklendi.",
                    "The file changed elsewhere; reloaded.",
                ),
                false,
            );
        }
        self.disk_time = now;
    }

    fn save(&mut self, by_hand: bool) {
        if self.read_only {
            return;
        }
        if !self.buf.dirty && !by_hand {
            return;
        }
        match file::write_atomic(&self.path, &self.buf.text) {
            Ok(()) => {
                self.buf.dirty = false;
                self.conflict = false;
                self.disk_time = file::modified(&self.path);
                let _ = file::snapshot(&self.history, &self.buf.text, false);
                if by_hand {
                    self.say(lang::l("Kaydedildi.", "Saved."), false);
                }
            }
            Err(e) => self.say(
                &lang::f(
                    "Kaydedilemedi: {}",
                    "Couldn't save: {}",
                    &[&format!("{e:#}")],
                ),
                true,
            ),
        }
    }

    /// The view toggles, for the settings.
    pub fn view(&self) -> View {
        View {
            typewriter: self.typewriter,
            focus: self.focus,
            dim: self.dim,
            paper: self.paper,
            pages: self.pages,
        }
    }

    /// On quit: save what's left and keep a version.
    pub fn close(&mut self) -> Result<()> {
        if self.buf.dirty && !self.read_only {
            file::write_atomic(&self.path, &self.buf.text)?;
        }
        let _ = file::snapshot(&self.history, &self.buf.text, true);
        Ok(())
    }

    // ------------------------------------------------------------ drawing

    fn keep_visible(&mut self) {
        let Some(cur) = self.cursor_stop() else {
            return;
        };
        let h = self.text_height();
        if self.typewriter {
            self.scroll = cur.row.saturating_sub(h / 2);
            return;
        }
        let pad = 3.min(h / 3);
        if cur.row < self.scroll + pad {
            self.scroll = cur.row.saturating_sub(pad);
        } else if cur.row + pad >= self.scroll + h {
            self.scroll = (cur.row + pad + 1).saturating_sub(h);
        }
    }

    fn text_height(&self) -> usize {
        self.height.saturating_sub(3).max(1) as usize
    }

    pub fn draw(&mut self, f: &mut Frame) {
        let area = f.area();
        if area.width != self.width || area.height != self.height {
            self.width = area.width;
            self.height = area.height;
            self.relayout();
            self.snap_back();
            self.keep_visible();
        }
        f.render_widget(
            ratatui::widgets::Block::default().style(theme::base()),
            area,
        );
        self.draw_header(f, Rect { height: 1, ..area });
        let body = Rect {
            y: area.y + 1,
            height: self.text_height() as u16,
            ..area
        };
        self.text_top = body.y;
        self.draw_sheet(f, body);
        self.draw_map(f, body);
        self.mark_misspelt();
        self.draw_text(f, body);
        let status = Rect {
            y: area.y + area.height.saturating_sub(2),
            height: 1,
            ..area
        };
        self.draw_status(f, status);
        let keys = Rect {
            y: area.y + area.height.saturating_sub(1),
            height: 1,
            ..area
        };
        self.draw_keys(f, keys);
        let at = self
            .cursor_stop()
            .filter(|c| c.row >= self.scroll)
            .map(|c| {
                Position::new(
                    body.x + self.sheet().text_x + c.col,
                    body.y + (c.row - self.scroll) as u16,
                )
            })
            .unwrap_or(Position::new(body.x, body.y));
        match &self.popup {
            Popup::None => {}
            Popup::Menu(m) => {
                let q = self.buf.text.get(m.from..self.buf.cursor).unwrap_or("");
                popup::draw_menu(f, body, at, m, q);
            }
            Popup::Verbs(v) => popup::draw_verbs(f, body, at, v),
            Popup::Search(m, q) => popup::draw_search(f, area, m, q),
            Popup::Form(form) => popup::draw_form(f, area, form),
            Popup::Cast(p) => {
                panels::draw_cast(f, area, &self.doc, &self.buf.text, &self.sidecar, p)
            }
            Popup::Scenes(p) => {
                let units = shelf::units(&self.buf.text, &self.doc);
                panels::draw_scenes(
                    f,
                    area,
                    &self.doc,
                    &units,
                    &self.marks,
                    &self.trash,
                    self.wpm(),
                    p,
                );
            }
            // Drawn below: it keeps where its caps are, for the mouse.
            Popup::Help(_) => {}
            Popup::Split(p) => panels::draw_split(
                f,
                area,
                &self.doc,
                &p.from,
                &p.spec.script,
                &p.scenes,
                &p.picked,
                p.sel,
            ),
            Popup::Suggest(range, items, sel) => {
                let word = self.buf.text.get(range.clone()).unwrap_or("");
                panels::draw_suggest(f, body, at, word, items, *sel);
            }
        }
        if let Popup::Help(h) = &mut self.popup {
            keys::draw_help(f, area, &self.keyboard, h);
        }
    }

    fn draw_header(&self, f: &mut Frame, area: Rect) {
        let title = self.doc.title(&self.buf.text).unwrap_or_else(|| {
            self.path
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default()
        });
        let state = if self.read_only {
            Span::styled(
                lang::l("salt okunur ", "read-only "),
                Style::default().fg(WARN),
            )
        } else if self.conflict {
            Span::styled(
                lang::l("dışarıda değişti ", "changed elsewhere "),
                Style::default().fg(WARN),
            )
        } else if self.buf.dirty {
            Span::styled("• ", Style::default().fg(MUTED))
        } else {
            Span::styled(lang::l("kaydedildi ", "saved "), Style::default().fg(FAINT))
        };
        let left = Line::from(vec![
            Span::styled(" dactile", Style::default().fg(MUTED)),
            Span::styled(" — ", Style::default().fg(FAINT)),
            Span::styled(
                title,
                Style::default().fg(BEIGE).add_modifier(Modifier::BOLD),
            ),
        ]);
        f.render_widget(Paragraph::new(left).style(theme::base()), area);
        let w = state.content.chars().count() as u16;
        let right = Rect {
            x: area.x + area.width.saturating_sub(w),
            width: w.min(area.width),
            ..area
        };
        f.render_widget(
            Paragraph::new(Line::from(state)).style(theme::base()),
            right,
        );
    }

    /// The play's headings in the margin left of the page, where there is
    /// room: prefaces, acts and their scenes, the cursor's one marked.
    fn draw_map(&mut self, f: &mut Frame, area: Rect) {
        self.map_hits.clear();
        let sheet = self.sheet();
        if sheet.x < 22 {
            return;
        }
        let o = &self.doc.outline;
        let base = self.doc.body.start;
        // (where it starts, depth, label, hidden)
        let mut entries: Vec<(usize, u8, String, bool)> = Vec::new();
        for sct in &o.sections {
            let name = if sct.title.trim().is_empty() {
                match sct.which {
                    dactile::lex::Section::Preface => lang::l("Önsöz", "Preface"),
                    dactile::lex::Section::Notes => lang::l("Rejisöre notlar", "Director's notes"),
                }
                .to_string()
            } else {
                sct.title.trim().to_string()
            };
            entries.push((self.doc.blocks[sct.block + base].span.start, 0, name, false));
        }
        for u in shelf::units(&self.buf.text, &self.doc) {
            let depth = u8::from(u.kind == shelf::UnitKind::Scene);
            let name = if u.hidden {
                format!("{} ({})", u.name(), lang::l("gizli", "hidden"))
            } else {
                u.name()
            };
            entries.push((u.span.start, depth, name, u.hidden));
        }
        entries.sort_by_key(|e| e.0);
        if entries.is_empty() {
            return;
        }
        let here = self.buf.cursor;
        let current = entries.iter().rposition(|e| e.0 <= here);
        let w = (sheet.x - 4) as usize;
        let rows = area.height.saturating_sub(2) as usize;
        let start = current
            .map_or(0, |c| c.saturating_sub(rows / 2))
            .min(entries.len().saturating_sub(rows));
        let x = area.x + 2;
        for (n, (at, depth, label, hidden)) in entries.iter().enumerate().skip(start).take(rows) {
            let y = area.y + 1 + (n - start) as u16;
            let is_here = Some(n) == current;
            let indent = if *depth == 0 { "" } else { "  " };
            let marked = self.marks.contains(at);
            let mark = match (marked, is_here) {
                (true, _) => "■ ",
                (_, true) => "▸ ",
                _ => "  ",
            };
            let mut style = if is_here {
                Style::default().fg(CLARET)
            } else if *depth == 0 && !hidden {
                Style::default().fg(MUTED)
            } else {
                Style::default().fg(FAINT)
            };
            if *hidden {
                style = style.add_modifier(Modifier::ITALIC);
            }
            let text = popup::fit(&format!("{indent}{label}"), w.saturating_sub(2));
            let line = Line::from(vec![
                Span::styled(
                    mark,
                    Style::default().fg(if marked { WARN } else { CLARET }),
                ),
                Span::styled(text, style),
            ]);
            f.render_widget(
                Paragraph::new(line).style(theme::base()),
                Rect {
                    x,
                    y,
                    width: w as u16,
                    height: 1,
                },
            );
            self.map_hits.push((y, *at));
        }
    }

    /// The desk and the page on it.
    fn draw_sheet(&self, f: &mut Frame, area: Rect) {
        let sheet = self.sheet();
        let p = self.palette();
        let buf = f.buffer_mut();
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                let on_page = x >= sheet.x && x < sheet.x + sheet.width;
                let shadow = self.paper && x == sheet.x + sheet.width && y > area.y;
                let bg = if on_page {
                    p.bg
                } else if shadow {
                    theme::SHADOW
                } else {
                    theme::BG
                };
                buf[(x, y)].set_char(' ').set_style(Style::default().bg(bg));
            }
        }
    }

    fn draw_text(&self, f: &mut Frame, area: Rect) {
        let sel = self.buf.selection();
        let sheet = self.sheet();
        let p = self.palette();
        let left = area.x + sheet.text_x;
        let right = area.x + (sheet.x + sheet.width).min(area.width);
        // Panels cover the text; forms and search type somewhere else.
        let covered = matches!(
            self.popup,
            Popup::Cast(_)
                | Popup::Scenes(_)
                | Popup::Split(_)
                | Popup::Help(_)
                | Popup::Form(_)
                | Popup::Search(..)
        );
        let cursor = self.cursor_stop().filter(|c| {
            !covered && c.row >= self.scroll && c.row < self.scroll + area.height as usize
        });
        let buf = f.buffer_mut();
        for (i, row) in self
            .lay
            .rows
            .iter()
            .skip(self.scroll)
            .take(area.height as usize)
            .enumerate()
        {
            let y = area.y + i as u16;
            if row.gap {
                // Between two pages: the desk shows through.
                let bg = if self.paper { theme::BG } else { theme::SHADOW };
                for x in
                    area.x + sheet.x..(area.x + sheet.x + sheet.width + 1).min(area.x + area.width)
                {
                    buf[(x, y)].set_char(' ').set_style(Style::default().bg(bg));
                }
                continue;
            }
            let here = row.block.is_some() && row.block == self.block();
            if let Some(n) = row.number {
                let label = format!("{n:>4}");
                let style = Style::default()
                    .bg(p.bg)
                    .fg(if here { p.muted } else { p.faint });
                let x0 = left.saturating_sub(MARGIN) + 1;
                for (k, ch) in label.chars().enumerate() {
                    let x = x0 + k as u16;
                    if x < right {
                        buf[(x, y)].set_char(ch).set_style(style);
                    }
                }
            }
            for c in &row.cells {
                let x = left + c.col;
                if x >= right {
                    continue;
                }
                let mut style = theme::on(c.style, p);
                if self.dim && !here {
                    style = style.fg(p.faint);
                }
                if let Some(b) = c.byte
                    && self.misspelt.iter().any(|r| r.contains(&b))
                {
                    style = style
                        .add_modifier(Modifier::UNDERLINED)
                        .underline_color(p.warn);
                }
                if let (Some(s), Some(b)) = (&sel, c.byte)
                    && s.contains(&b)
                {
                    style = style.bg(p.selection);
                }
                buf[(x, y)].set_char(c.ch).set_style(style);
            }
        }
        // Who speaks next, faint, on the empty line.
        if let (Some(a), Some(cur)) = (self.ghost(), cursor)
            && let Some(c) = self.doc.cast.get(&a)
        {
            let y = area.y + (cur.row - self.scroll) as u16;
            let hint = format!("{}   Tab", c.script);
            let style = Style::default()
                .bg(p.bg)
                .fg(p.faint)
                .add_modifier(Modifier::ITALIC);
            for (k, ch) in hint.chars().enumerate() {
                let x = left + cur.col + k as u16;
                if x < right {
                    buf[(x, y)].set_char(ch).set_style(style);
                }
            }
        }
        // The cursor, unless a full panel covers the text. On paper the
        // terminal's own cursor can vanish, so the page draws its own.
        if let Some(cur) = cursor {
            let x = (left + cur.col).min(right.saturating_sub(1));
            let y = area.y + (cur.row - self.scroll) as u16;
            if self.paper {
                let cell = &mut buf[(x, y)];
                cell.set_style(Style::default().fg(p.bg).bg(p.fg));
            } else {
                f.set_cursor_position(Position::new(x, y));
            }
        }
    }

    fn draw_status(&self, f: &mut Frame, area: Rect) {
        if let Some((msg, warn)) = &self.message {
            let style = Style::default().fg(if *warn { WARN } else { MUTED });
            f.render_widget(
                Paragraph::new(format!(" {msg}")).style(theme::base().patch(style)),
                area,
            );
            return;
        }
        let mut parts: Vec<Span> = Vec::new();
        let block = self.block();
        if let Some(a) = block.and_then(|b| self.doc.act_of(b)) {
            parts.push(Span::styled(
                format!(
                    " {}. {}",
                    outline::roman(self.doc.outline.acts[a].number),
                    lang::l("Perde", "Act")
                ),
                Style::default().fg(CLARET),
            ));
        }
        if let Some(s) = block.and_then(|b| self.doc.scene_of(b)) {
            let s = &self.doc.outline.scenes[s];
            parts.push(Span::styled(
                format!(" · {} {}", lang::l("Sahne", "Scene"), s.label),
                Style::default().fg(CLARET),
            ));
            parts.push(Span::styled(
                format!(
                    " · {} {} · ≈ {}",
                    s.words,
                    lang::l("kelime", "words"),
                    timing::format(timing::scene_secs(s, self.wpm()))
                ),
                Style::default().fg(MUTED),
            ));
        }
        if let Some(who) = block
            .and_then(|b| self.doc.speaker(b))
            .and_then(|a| self.doc.cast.get(a))
        {
            parts.push(Span::styled(
                format!(" · {}", who.script),
                Style::default().fg(BEIGE),
            ));
        }
        if self.raw_now() {
            parts.push(Span::styled(
                lang::l("  HAM", "  RAW"),
                Style::default().fg(WARN).add_modifier(Modifier::BOLD),
            ));
        }
        if parts.is_empty() {
            parts.push(Span::styled(" ", Style::default()));
        }
        f.render_widget(Paragraph::new(Line::from(parts)).style(theme::base()), area);
        // The view's modes on the right, while they are on.
        let mut tags: Vec<&str> = Vec::new();
        if self.typewriter {
            tags.push(lang::l("daktilo", "typewriter"));
        }
        if self.focus {
            tags.push(lang::l("odak", "focus"));
        }
        if self.dim {
            tags.push(lang::l("soluk", "dim"));
        }
        let page;
        if self.pages && !self.lay.pages.is_empty() {
            let row = self.cursor_stop().map_or(0, |c| c.row);
            let n = self.lay.pages.partition_point(|&p| p <= row);
            page = lang::f("sayfa {}/{}", "page {}/{}", &[&n, &self.lay.pages.len()]);
            tags.push(&page);
        }
        let misspelt = self.misspelt.len();
        let spell = (misspelt > 0).then(|| lang::f("{} yazım", "{} spelling", &[&misspelt]));
        let mut right: Vec<Span> = Vec::new();
        if let Some(sp) = spell {
            right.push(Span::styled(sp, Style::default().fg(WARN)));
            right.push(Span::raw("  "));
        }
        for t in tags {
            right.push(Span::styled(
                format!(" {t} "),
                Style::default().fg(BEIGE).bg(theme::SELECTION),
            ));
            right.push(Span::raw(" "));
        }
        let w: usize = right.iter().map(|s| s.content.chars().count()).sum();
        if w > 0 && (w as u16) < area.width / 2 {
            let r = Rect {
                x: area.x + area.width - w as u16,
                width: w as u16,
                ..area
            };
            f.render_widget(Paragraph::new(Line::from(right)).style(theme::base()), r);
        }
    }

    fn draw_keys(&self, f: &mut Frame, area: Rect) {
        // (key, what, how much it matters): the least needed go first when
        // the screen is narrow.
        let keys = [
            ("F1", lang::l("yardım", "help"), 9),
            ("\\", lang::l("öneri", "suggest"), 7),
            ("^O", lang::l("kişiler", "cast"), 8),
            ("^G", lang::l("sahneler", "scenes"), 6),
            ("^F", lang::l("bul", "find"), 4),
            ("^K", lang::l("ara", "search"), 3),
            ("^Z", lang::l("geri al", "undo"), 2),
            ("F6", lang::l("kâğıt", "paper"), 1),
            ("F7", lang::l("sayfalar", "pages"), 1),
            ("^S", lang::l("kaydet", "save"), 5),
            ("^Q", lang::l("çık", "quit"), 10),
        ];
        let len = |k: &str, w: &str| k.chars().count() + w.chars().count() + 4;
        let mut shown: Vec<_> = keys.to_vec();
        while shown.iter().map(|(k, w, _)| len(k, w)).sum::<usize>() + 1 > area.width as usize {
            let Some(least) = shown
                .iter()
                .enumerate()
                .min_by_key(|(_, k)| k.2)
                .map(|(i, _)| i)
            else {
                break;
            };
            shown.remove(least);
        }
        let mut spans = vec![Span::raw(" ")];
        for (k, what, _) in shown {
            spans.push(Span::styled(k, Style::default().fg(MUTED)));
            spans.push(Span::styled(
                format!(" {what}   "),
                Style::default().fg(FAINT),
            ));
        }
        f.render_widget(Paragraph::new(Line::from(spans)).style(theme::base()), area);
    }
}
