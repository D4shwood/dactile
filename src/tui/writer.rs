//! The writing screen: the play as it will read, edited in place.

use super::panels::{self, CastPanel, Entry};
use super::popup::{self, Field, Form, FormKind, Menu, Verbs};
use super::theme::{self, BEIGE, CLARET, FAINT, MUTED, SELECTION, WARN};
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
use dactile::{info, timing};
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
/// Columns left of the text: the draft line numbers.
const MARGIN: u16 = 6;

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
    /// ^G: the scenes.
    Scenes(usize),
    Help,
    /// Alt+S: spelling suggestions for the word at `range`.
    Suggest(std::ops::Range<usize>, Vec<String>, usize),
}

impl Writer {
    pub fn open(path: &Path) -> Result<Writer> {
        let text = file::read(path)?;
        let (lock, read_only, message) = match file::lock(path)? {
            LockState::Ours(l) => (Some(l), false, None),
            LockState::Taken(who) => (
                None,
                true,
                Some((
                    format!("Başka bir Dactile bu oyunu açmış ({who}); salt okunur açıldı."),
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
            typewriter: false,
            focus: false,
            dim: false,
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
            focus_scene: if self.focus {
                self.block().and_then(|b| self.doc.scene_of(b))
            } else {
                None
            },
        }
    }

    fn text_width(&self) -> u16 {
        self.width.saturating_sub(MARGIN + 2).max(20)
    }

    /// Rebuilds the model and the layout from the text.
    fn relayout(&mut self) {
        self.doc = Doc::new(&self.buf.text);
        self.lay = render::layout(&self.buf.text, &self.doc, self.opts());
    }

    /// Lays out again after an edit; the cursor lands on a stop.
    fn edited(&mut self) {
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
            self.say("Salt okunur: oyun başka bir Dactile'da açık.", true);
            return false;
        }
        if self.in_folded_preamble() {
            self.say("Kişiler ve oyun bilgisi kapalı: Alt+P ile aç.", false);
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
                        "Daktilo modu açık."
                    } else {
                        "Daktilo modu kapalı."
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
                        "Sahne odağı: yalnız bu sahne."
                    } else {
                        "Sahne odağı kapalı."
                    },
                    false,
                );
            }
            KeyCode::F(5) => {
                self.dim = !self.dim;
            }
            KeyCode::Char('o') if ctrl => {
                self.popup = Popup::Cast(CastPanel {
                    sel: 0,
                    by_lines: false,
                });
            }
            KeyCode::Char('g') if ctrl => {
                let sel = self.block().and_then(|b| self.doc.scene_of(b)).unwrap_or(0);
                self.popup = Popup::Scenes(sel);
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
                    title: "bul".into(),
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
            KeyCode::F(1) => self.popup = Popup::Help,
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
            KeyCode::Tab => {}
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
        let block = self.block().unwrap_or(0);
        let dialogue = self.doc.is_dialogue(block) || self.at_blank_line();
        if c == '\\' {
            // A macro is being written: its block shows as source until done,
            // with what can follow the backslash.
            self.raw_here = true;
            self.buf.insert("\\");
            let items = complete::items(&self.doc, self.block());
            self.popup = Popup::Menu(Menu::new(items, self.buf.cursor));
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
                    self.popup = Popup::None;
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
            Popup::Scenes(sel) => {
                let n = self.doc.outline.scenes.len();
                match k.code {
                    KeyCode::Esc => self.popup = Popup::None,
                    KeyCode::Char('g') if ctrl => self.popup = Popup::None,
                    KeyCode::Up => *sel = sel.saturating_sub(1),
                    KeyCode::Down => *sel = (*sel + 1).min(n.saturating_sub(1)),
                    KeyCode::Enter => {
                        let target = self
                            .doc
                            .outline
                            .scenes
                            .get(*sel)
                            .map(|s| s.block + self.doc.body.start);
                        self.popup = Popup::None;
                        if let Some(b) = target {
                            let at = self.doc.blocks[b].span.start;
                            self.go(at, false);
                            self.relayout();
                            // The first line after the heading.
                            if let Some(s) = self.lay.next_stop(self.doc.blocks[b].span.end) {
                                self.go(s.byte, false);
                            }
                        }
                    }
                    _ => {}
                }
                true
            }
            Popup::Help => {
                if matches!(k.code, KeyCode::Esc | KeyCode::F(1) | KeyCode::Enter) {
                    self.popup = Popup::None;
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
                            self.say(&format!("«{word}» bu oyunun sözlüğüne eklendi."), false);
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
            title: "yeni sahne".into(),
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
            title: "yeni perde".into(),
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
            title: "yeni karakter".into(),
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
        if form.kind == FormKind::Character {
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
                let parts: Vec<&str> = [form.get("Yer"), form.get("Zaman")]
                    .into_iter()
                    .filter(|s| !s.is_empty())
                    .collect();
                let text = if parts.is_empty() {
                    "\\sahne".to_string()
                } else {
                    format!("\\sahne[{}]", parts.join(" — "))
                };
                self.write_line(form.at.start, &text, &nl);
            }
            FormKind::Act => {
                let title = form.get("Başlık");
                let text = if title.is_empty() {
                    "\\perde".to_string()
                } else {
                    format!("\\perde[{title}]")
                };
                self.write_line(form.at.start, &text, &nl);
            }
            FormKind::Find => {}
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
                    self.say(&format!("Kart kaydedilemedi: {e:#}"), true);
                }
                self.popup = Popup::Cast(CastPanel {
                    sel: self.cast_index(&alias),
                    by_lines: false,
                });
            }
            FormKind::Info => {
                let mut t = self.buf.text.clone();
                for (cmd, label) in info::FIELDS {
                    if let Some(e) = info::set(&t, cmd, form.get(label)) {
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
                    self.say("Oyun bilgisi güncellendi.", false);
                }
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
                    return Err(Box::new((form, "Ad boş olamaz.".into())));
                }
                if alias.is_empty() || !alias.chars().all(|c| c.is_ascii_alphabetic()) {
                    return Err(Box::new((
                        form,
                        "Kısaltma yalnız İngilizce harflerden oluşmalı (ş, ç… olmadan).".into(),
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
                    script,
                    full: form.get("Tam ad").to_string(),
                    desc: form.get("Açıklama").to_string(),
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
                    edits.push(dactile::tex::Edit::new(at..at, use_text));
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
            return Err("Ad boş olamaz.".into());
        }
        if alias.is_empty() || !alias.chars().all(|c| c.is_ascii_alphabetic()) {
            return Err("Kısaltma yalnız İngilizce harflerden oluşmalı (ş, ç… olmadan).".into());
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
            script,
            full: form.get("Tam ad").to_string(),
            desc: form.get("Açıklama").to_string(),
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
            None => self.say(&format!("«{needle}» bulunamadı."), false),
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
        self.say(&format!("{n} yer değişti (^Z geri alır)."), false);
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
        self.sidecar.wpm.unwrap_or(timing::WPM)
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
                    None => self.say("Yalnız kişiler tablosundaki satırlar taşınır.", false),
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
                        self.say(&format!("\\{a} silindi (^Z geri alır)."), false);
                        if let Popup::Cast(p) = &mut self.popup {
                            p.sel = sel.min(n.saturating_sub(2));
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn edit_form(&self, alias: &str) -> Form {
        let c = self.doc.cast.get(alias).cloned();
        let c = c.as_ref();
        let mut f = self.character_form(alias, None);
        f.kind = FormKind::EditCharacter(alias.to_string());
        f.title = format!("\\{alias} düzenle");
        let set = |f: &mut Form, label: &str, v: String| {
            if let Some(x) = f.field_mut(label) {
                x.value = v;
                x.touched = true;
            }
        };
        if let Some(c) = c {
            set(&mut f, "Ad", c.script.clone());
            set(&mut f, "Tam ad", c.full.clone().unwrap_or_default());
            set(&mut f, "Açıklama", c.desc.clone().unwrap_or_default());
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
            title: format!("{name} — kart (PDF'e girmez)"),
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
                Field::text(label, &v, Vec::new())
            })
            .collect();
        fields.push(Field::flag("Final (baskı)", info::is_final(&self.buf.text)));
        Form {
            kind: FormKind::Info,
            title: "oyun bilgisi".into(),
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
                "Sözlük yükleniyor, birazdan."
            } else {
                "Türkçe sözlük bulunamadı (tr_TR.aff/.dic): paru -S hunspell-tr ile kurulabilir."
            };
            self.say(why, true);
            return;
        }
        let cur = self.buf.cursor;
        let Some(b) = self.block() else { return };
        let words = spell::words(&self.buf.text, &self.doc, b);
        let Some(range) = words.into_iter().find(|r| r.start <= cur && cur <= r.end) else {
            self.say("İmleç bir kelimenin üstünde değil.", false);
            return;
        };
        let word = self.buf.text[range.clone()].to_string();
        let mut items = self
            .speller
            .as_ref()
            .map(|s| s.suggest(&word))
            .unwrap_or_default();
        items.push(format!("+ «{word}» oyunun sözlüğüne"));
        self.popup = Popup::Suggest(range, items, 0);
    }

    // ------------------------------------------------------------ mouse

    pub fn mouse(&mut self, m: MouseEvent) {
        match m.kind {
            MouseEventKind::ScrollUp => self.scroll = self.scroll.saturating_sub(3),
            MouseEventKind::ScrollDown => {
                let max = self.lay.rows.len().saturating_sub(self.text_height() / 2);
                self.scroll = (self.scroll + 3).min(max);
            }
            MouseEventKind::Down(MouseButton::Left) | MouseEventKind::Drag(MouseButton::Left) => {
                if !matches!(self.popup, Popup::None) || m.row < self.text_top {
                    return;
                }
                let row = self.scroll + (m.row - self.text_top) as usize;
                let col = m.column.saturating_sub(MARGIN);
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
                Err(e) => self.say(&format!("Sözlük yüklenemedi: {e:#}"), true),
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
                "Dosya başka yerde değişti. ^S senin halini yazar; ^Q'dan önce karar ver.",
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
            self.say("Dosya başka yerde değişti; yeniden yüklendi.", false);
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
                    self.say("Kaydedildi.", false);
                }
            }
            Err(e) => self.say(&format!("Kaydedilemedi: {e:#}"), true),
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
                    body.x + MARGIN + c.col,
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
            Popup::Scenes(sel) => panels::draw_scenes(f, area, &self.doc, self.wpm(), *sel),
            Popup::Help => panels::draw_help(f, area),
            Popup::Suggest(range, items, sel) => {
                let word = self.buf.text.get(range.clone()).unwrap_or("");
                panels::draw_suggest(f, body, at, word, items, *sel);
            }
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
            Span::styled("salt okunur ", Style::default().fg(WARN))
        } else if self.conflict {
            Span::styled("dışarıda değişti ", Style::default().fg(WARN))
        } else if self.buf.dirty {
            Span::styled("• ", Style::default().fg(MUTED))
        } else {
            Span::styled("kaydedildi ", Style::default().fg(FAINT))
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

    fn draw_text(&self, f: &mut Frame, area: Rect) {
        let sel = self.buf.selection();
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
            let here = row.block.is_some() && row.block == self.block();
            if let Some(n) = row.number {
                let label = format!("{n:>4}");
                let style = theme::base().fg(if here { MUTED } else { FAINT });
                for (k, ch) in label.chars().enumerate() {
                    if (k as u16) < area.width {
                        buf[(area.x + k as u16, y)].set_char(ch).set_style(style);
                    }
                }
            }
            for c in &row.cells {
                let x = area.x + MARGIN + c.col;
                if x >= area.x + area.width {
                    continue;
                }
                let mut style = theme::of(c.style);
                if self.dim && !here {
                    style = style.fg(FAINT);
                }
                if let Some(b) = c.byte
                    && self.misspelt.iter().any(|r| r.contains(&b))
                {
                    style = style
                        .add_modifier(Modifier::UNDERLINED)
                        .underline_color(WARN);
                }
                if let (Some(s), Some(b)) = (&sel, c.byte)
                    && s.contains(&b)
                {
                    style = style.bg(SELECTION);
                }
                buf[(x, y)].set_char(c.ch).set_style(style);
            }
        }
        // The cursor.
        if let Some(cur) = self.cursor_stop()
            && cur.row >= self.scroll
            && cur.row < self.scroll + area.height as usize
        {
            let x = (area.x + MARGIN + cur.col).min(area.x + area.width.saturating_sub(1));
            f.set_cursor_position(Position::new(x, area.y + (cur.row - self.scroll) as u16));
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
                    " {}. Perde",
                    outline::roman(self.doc.outline.acts[a].number)
                ),
                Style::default().fg(CLARET),
            ));
        }
        if let Some(s) = block.and_then(|b| self.doc.scene_of(b)) {
            let s = &self.doc.outline.scenes[s];
            parts.push(Span::styled(
                format!(" · Sahne {}", s.label),
                Style::default().fg(CLARET),
            ));
            parts.push(Span::styled(
                format!(
                    " · {} kelime · ≈ {}",
                    s.words,
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
                "  HAM",
                Style::default().fg(WARN).add_modifier(Modifier::BOLD),
            ));
        }
        if parts.is_empty() {
            parts.push(Span::styled(" ", Style::default()));
        }
        f.render_widget(Paragraph::new(Line::from(parts)).style(theme::base()), area);
    }

    fn draw_keys(&self, f: &mut Frame, area: Rect) {
        let keys = [
            ("^S", "kaydet"),
            ("^Z", "geri al"),
            ("^C/^X/^V", "kopyala"),
            ("F1", "yardım"),
            ("\\", "öneri"),
            ("^O", "kişiler"),
            ("^G", "sahneler"),
            ("^F", "bul"),
            ("^Q", "çık"),
        ];
        let mut spans = vec![Span::raw(" ")];
        for (k, what) in keys {
            spans.push(Span::styled(k, Style::default().fg(MUTED)));
            spans.push(Span::styled(
                format!(" {what}   "),
                Style::default().fg(FAINT),
            ));
        }
        f.render_widget(Paragraph::new(Line::from(spans)).style(theme::base()), area);
    }
}
