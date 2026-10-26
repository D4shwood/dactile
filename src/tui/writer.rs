//! The writing screen: the play as it will read, edited in place.

use super::theme::{self, BEIGE, CLARET, FAINT, MUTED, SELECTION, WARN};
use anyhow::Result;
use dactile::buffer::{self, Buffer};
use dactile::file::{self, LockState};
use dactile::lex::Kind;
use dactile::outline;
use dactile::render::{self, Doc, Layout, Options, Stop};
use dactile::source;
use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Position, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

/// Saved this long after the last keystroke.
const AUTOSAVE_AFTER: Duration = Duration::from_secs(2);
/// How often the file on disk is checked for changes from elsewhere.
const CHECK_EVERY: Duration = Duration::from_secs(2);
/// Columns left of the text.
const MARGIN: u16 = 2;

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
    pub quit: bool,
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
            quit: false,
        };
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
        self.buf.cursor = byte;
        if self.raw_here && self.block() != before {
            self.raw_here = false;
            self.relayout();
            self.snap_back();
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
        if c == '\\' && !self.raw_now() {
            // A macro is being written: its block shows as source until done.
            self.raw_here = true;
            self.buf.insert("\\");
        } else if self.raw_now() {
            self.buf.insert(&c.to_string());
            // `\se ` or `\se{` finishes the macro: show the block live again.
            if self.raw_here && (c == ' ' || c == '{') && self.ends_macro() {
                self.raw_here = false;
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

    // ------------------------------------------------------------ saving

    pub fn tick(&mut self) {
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
            for c in &row.cells {
                let x = area.x + MARGIN + c.col;
                if x >= area.x + area.width {
                    continue;
                }
                let mut style = theme::of(c.style);
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
                format!(" · {} kelime", s.words),
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
            ("F2", "ham"),
            ("Alt+P", "kişiler"),
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
