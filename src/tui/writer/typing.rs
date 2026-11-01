//! Keys in the writing: moving, typing, deleting, pasting.

use super::*;

impl Writer {
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
            KeyCode::F(5) => self.build(),
            KeyCode::F(8) if shift => self.toggle_role_view(),
            KeyCode::F(8) => {
                self.dim = !self.dim;
            }
            KeyCode::F(9) => self.popup = Popup::Roles,
            KeyCode::Char('l') if alt => {
                if let Err(e) = build::open_log(&self.path) {
                    self.say(&format!("{e:#}"), true);
                }
            }
            KeyCode::Char('o') if alt => {
                if let Err(e) = build::open(&build::pdf_path(&self.path)) {
                    self.say(&format!("{e:#}"), true);
                }
            }
            KeyCode::F(6) => {
                self.theme = theme::next_theme(&self.theme).to_string();
                self.say(
                    &lang::f("Tema: {}", "Theme: {}", &[&theme::theme_name(&self.theme)]),
                    false,
                );
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
            // The strip; twice quickly, or when the strip has no room, the board.
            KeyCode::Char('g') if ctrl => {
                let twice = self.strip_key.take().is_some_and(|t| t.elapsed() < TWICE);
                let room = {
                    let was = self.strip;
                    self.strip = true;
                    let r = self.strip_shown();
                    self.strip = was;
                    r
                };
                if twice || !room {
                    if twice {
                        self.strip = !self.strip;
                    }
                    self.open_board();
                } else {
                    self.strip = !self.strip;
                    self.strip_key = Some(Instant::now());
                }
            }
            KeyCode::Char('e') if alt => self.summary_here(),
            KeyCode::Char('b') if alt => self.split_here(),
            KeyCode::Char('j') if alt => {
                let at = self.buf.cursor;
                if let Some(k) = self.doc.block_at(at).and_then(|b| self.doc.scene_of(b)) {
                    self.join(k);
                }
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
            KeyCode::Char('p') if ctrl => self.popup = Popup::Palette(String::new(), 0),
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

    pub(super) fn page(&self) -> usize {
        self.height.saturating_sub(5).max(1) as usize
    }

    pub(super) fn row_stops(&self) -> Vec<Stop> {
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

    pub(super) fn vertical(&mut self, rows: isize, select: bool) {
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

    pub(super) fn word_left(&self) -> Option<usize> {
        let t = &self.buf.text[..self.buf.cursor];
        let trimmed = t.trim_end_matches(|c: char| !c.is_alphanumeric());
        let start = trimmed
            .rfind(|c: char| !c.is_alphanumeric())
            .map_or(0, |i| i + trimmed[i..].chars().next().unwrap().len_utf8());
        self.lay.stop_at(start).map(|s| s.byte)
    }

    pub(super) fn word_right(&self) -> Option<usize> {
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
    pub(super) fn plain_at(&self, at: usize) -> bool {
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

    pub(super) fn is_line_break(&self, at: usize) -> bool {
        matches!(self.buf.text.as_bytes().get(at), Some(b'\n' | b'\r'))
    }

    pub(super) fn backspace(&mut self) {
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

    pub(super) fn delete(&mut self) {
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

    pub(super) fn type_char(&mut self, c: char) {
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

    pub(super) fn snap_forward_if_inside(&mut self) {
        if !self.lay.is_stop(self.buf.cursor)
            && let Some(s) = self.lay.next_stop(self.buf.cursor)
        {
            self.buf.cursor = s.byte;
        }
    }

    /// The text before the cursor ends with `\letters` and the char just typed.
    pub(super) fn ends_macro(&self) -> bool {
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
    pub(super) fn at_line_start(&self) -> bool {
        self.buf.text[self.line_start()..self.buf.cursor]
            .trim()
            .is_empty()
    }

    pub(super) fn line_start(&self) -> usize {
        let t = &self.buf.text[..self.buf.cursor];
        t.rfind('\n').map_or(0, |i| i + 1)
    }

    /// `SELİN:` typed at a line's start: the line becomes SELİN's. A name in
    /// capitals nobody has (`AHMET:`) asks for a new character. True when the
    /// colon was taken.
    pub(super) fn name_colon(&mut self) -> bool {
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
    pub(super) fn ghost(&self) -> Option<String> {
        if !matches!(self.popup, Popup::None)
            || self.raw_now()
            || !self.at_blank_line()
            || !self.at_line_start()
        {
            return None;
        }
        dactile::writing::next_speaker(&self.doc, self.block()?)
    }

    pub(super) fn at_blank_line(&self) -> bool {
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

    pub(super) fn copy(&mut self, cut: bool) {
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
    pub(super) fn macro_before(&self) -> Option<String> {
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
    pub(super) fn after_macro(&mut self, name: &str) {
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
}
