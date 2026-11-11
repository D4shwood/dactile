//! Notes beside the writing (^N): a panel on the right, each note's place
//! in the text underlined in its author's colour, a key to put it right.

use super::*;
use dactile::notes::{self as dnotes, Anchor, Index, Note, Notes, State};
use ratatui::style::Color;

/// The panel's width, its border included.
pub(super) const NOTES_W: u16 = 42;

/// Each author's colour, in the order they first wrote.
const AUTHOR_COLORS: [Color; 6] = [
    CLARET,
    Color::Rgb(0x5f, 0x9a, 0x96),
    WARN,
    Color::Rgb(0x76, 0x8f, 0xc0),
    Color::Rgb(0x8e, 0xa3, 0x5e),
    Color::Rgb(0xa8, 0x82, 0xb4),
];

/// The open notes, all but the archive, or the archive.
#[derive(Clone, Copy, Default, PartialEq)]
pub enum Show {
    #[default]
    Open,
    All,
    Archive,
}

impl Show {
    fn lists(self, n: &Note) -> bool {
        match self {
            Show::Open => !n.archived && n.state == State::Open,
            Show::All => !n.archived,
            Show::Archive => n.archived,
        }
    }

    fn next(self) -> Show {
        match self {
            Show::Open => Show::All,
            Show::All => Show::Archive,
            Show::Archive => Show::Open,
        }
    }
}

#[derive(Default)]
pub struct NotesView {
    pub open: bool,
    /// Keys go to the panel, not the text.
    pub focus: bool,
    /// An index into the notes.
    pub sel: usize,
    /// Only this author's.
    pub filter: Option<String>,
    /// Which of them the panel lists.
    pub show: Show,
    /// What was last read or written of the notes file.
    pub disk_time: Option<SystemTime>,
    /// Bumped on every change to the notes, for the places to be found again.
    pub generation: u64,
    /// The places, worked out once per version of the text and the notes.
    pub places: Option<Places>,
    /// Notes whose quote was found once: gone, they look put right.
    pub seen: HashSet<String>,
    /// The desktop's file chooser, for a file of notes.
    pub picking: Option<Receiver<Option<PathBuf>>>,
    /// Who the last pasted notes were from.
    pub last_from: Option<String>,
    /// A copy with the notes is being built.
    pub building: bool,
}

pub struct Places {
    rev: u64,
    generation: u64,
    pub anchors: Vec<Option<Anchor>>,
    pub gone: Vec<bool>,
    pub index: Index,
}

impl Writer {
    // ------------------------------------------------------------ model

    pub(super) fn load_notes(&mut self) {
        match Notes::load(&self.path) {
            Ok(n) => self.notes = n,
            Err(e) => self.say(
                &lang::f(
                    "Notlar okunamadı: {}",
                    "The notes didn't read: {}",
                    &[&format!("{e:#}")],
                ),
                true,
            ),
        }
        self.notes_view.disk_time = file::modified(&dnotes::path_for(&self.path));
        self.notes_changed();
    }

    fn notes_changed(&mut self) {
        self.notes_view.generation += 1;
        let n = self.notes.list.len();
        if self.notes_view.sel >= n {
            self.notes_view.sel = n.saturating_sub(1);
        }
    }

    /// Writes the notes now: every change to them is one.
    pub(super) fn save_notes(&mut self) {
        self.notes_changed();
        match self.notes.save(&self.path) {
            Ok(()) => self.notes_view.disk_time = file::modified(&dnotes::path_for(&self.path)),
            Err(e) => self.say(
                &lang::f(
                    "Notlar kaydedilemedi: {}",
                    "Couldn't save the notes: {}",
                    &[&format!("{e:#}")],
                ),
                true,
            ),
        }
    }

    /// The notes file changed elsewhere (another editor, an AI writing it):
    /// read again.
    pub(super) fn check_notes_disk(&mut self) {
        let now = file::modified(&dnotes::path_for(&self.path));
        if now == self.notes_view.disk_time {
            return;
        }
        let had = !self.notes.list.is_empty();
        self.load_notes();
        if had || !self.notes.list.is_empty() {
            self.say(
                lang::l(
                    "Notlar başka yerde değişti; yeniden okundu.",
                    "The notes changed elsewhere; read again.",
                ),
                false,
            );
        }
    }

    /// The notes' places, for this text and these notes.
    pub(super) fn places(&mut self) -> &Places {
        let (rev, generation) = (self.buf.rev, self.notes_view.generation);
        let fresh = self
            .notes_view
            .places
            .as_ref()
            .is_some_and(|p| p.rev == rev && p.generation == generation);
        if !fresh {
            let index = Index::new(&self.buf.text);
            let anchors: Vec<Option<Anchor>> =
                self.notes.list.iter().map(|n| index.anchor(n)).collect();
            for (n, a) in self.notes.list.iter().zip(&anchors) {
                if a.as_ref().is_some_and(|a| a.quoted) {
                    self.notes_view.seen.insert(n.text.clone());
                }
            }
            let gone = self
                .notes
                .list
                .iter()
                .map(|n| {
                    n.state == State::Open
                        && self.notes_view.seen.contains(&n.text)
                        && index.gone(n)
                })
                .collect();
            self.notes_view.places = Some(Places {
                rev,
                generation,
                anchors,
                gone,
                index,
            });
        }
        self.notes_view.places.as_ref().unwrap()
    }

    /// After a save: each note's number follows its quote to where it is
    /// now, so the file stays right for whoever reads it.
    pub(super) fn notes_follow(&mut self) {
        if self.notes.list.is_empty() {
            return;
        }
        let lines: Vec<Option<usize>> = self
            .places()
            .anchors
            .iter()
            .map(|a| a.as_ref().filter(|a| a.quoted).map(|a| a.line))
            .collect();
        let mut changed = false;
        for (n, l) in self.notes.list.iter_mut().zip(lines) {
            if let Some(l) = l
                && n.line != Some(l)
            {
                n.line = Some(l);
                changed = true;
            }
        }
        if changed {
            self.save_notes();
        }
    }

    /// The notes the panel lists, in order.
    pub(super) fn shown_notes(&self) -> Vec<usize> {
        let v = &self.notes_view;
        let mut out: Vec<usize> = (0..self.notes.list.len())
            .filter(|&i| {
                let n = &self.notes.list[i];
                v.show.lists(n) && v.filter.as_ref().is_none_or(|a| &n.author == a)
            })
            .collect();
        // By group as they first came, the archive last.
        let mut groups: Vec<(bool, &str)> = Vec::new();
        for &i in &out {
            let n = &self.notes.list[i];
            if !groups.contains(&(n.archived, n.group.as_str())) {
                groups.push((n.archived, &n.group));
            }
        }
        // As in the file: the ones under no heading first.
        groups.sort_by_key(|g| (g.0, !g.1.is_empty()));
        out.sort_by_key(|&i| {
            let n = &self.notes.list[i];
            groups
                .iter()
                .position(|g| *g == (n.archived, n.group.as_str()))
        });
        out
    }

    pub(super) fn author_color(&self, author: &str) -> Color {
        let i = self
            .notes
            .authors()
            .iter()
            .position(|a| a == author)
            .unwrap_or(0);
        AUTHOR_COLORS[i % AUTHOR_COLORS.len()]
    }

    /// Whether the panel and the page fit side by side.
    pub(super) fn notes_beside(&self) -> bool {
        self.notes_view.open && self.width >= MARGIN + 50 + PAGE_RIGHT + NOTES_W
    }

    /// The open notes' places in the text, in their author's colour; the
    /// chosen one marked.
    pub(super) fn mark_notes(&mut self) {
        self.note_marks.clear();
        if self.notes.list.is_empty() {
            return;
        }
        let sel = self.notes_view.open.then_some(self.notes_view.sel);
        let anchors = self.places().anchors.clone();
        for (i, a) in anchors.into_iter().enumerate() {
            let n = &self.notes.list[i];
            let Some(a) = a else { continue };
            let chosen = sel == Some(i);
            if !chosen && (n.state != State::Open || n.archived) {
                continue;
            }
            let color = self.author_color(&n.author);
            self.note_marks.push(NoteMark {
                range: a.range,
                quoted: a.quoted,
                color,
                chosen,
            });
        }
    }

    // ------------------------------------------------------------ keys

    /// ^N: the panel opens and takes the keys; again, back to the text with
    /// it open; once more, it closes.
    pub(super) fn toggle_notes(&mut self) {
        let v = &mut self.notes_view;
        match (v.open, v.focus) {
            (false, _) => {
                v.open = true;
                v.focus = true;
                if self.notes.list.is_empty() {
                    self.say(
                        lang::l(
                            "Not yok. p panodan yapıştırır, i dosyadan alır, n buraya not yazar.",
                            "No notes. p pastes them, i takes a file, n writes one here.",
                        ),
                        false,
                    );
                } else {
                    self.pick_shown_note();
                }
            }
            (true, false) => v.focus = true,
            (true, true) => {
                v.open = false;
                v.focus = false;
            }
        }
        self.relayout();
        self.snap_back();
    }

    /// The chosen note is one the panel shows: the nearest to the cursor.
    fn pick_shown_note(&mut self) {
        let shown = self.shown_notes();
        if shown.contains(&self.notes_view.sel) {
            return;
        }
        let at = self.buf.cursor;
        let anchors = self.places().anchors.clone();
        if let Some(&i) = shown.iter().min_by_key(|&&i| {
            anchors[i]
                .as_ref()
                .map_or(usize::MAX, |a| a.range.start.abs_diff(at))
        }) {
            self.notes_view.sel = i;
        }
    }

    /// Keys while the panel has them. Whether it took the key.
    pub(super) fn notes_key(&mut self, k: KeyEvent, armed: Option<Armed>) -> bool {
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        let alt = k.modifiers.contains(KeyModifiers::ALT);
        if ctrl || alt {
            return false;
        }
        let shown = self.shown_notes();
        let pos = shown.iter().position(|&i| i == self.notes_view.sel);
        let step = |d: isize| -> Option<usize> {
            if shown.is_empty() {
                return None;
            }
            let p = pos.map_or(0, |p| (p as isize + d).clamp(0, shown.len() as isize - 1));
            Some(shown[p as usize])
        };
        let sel = self.notes_view.sel;
        let has = pos.is_some();
        match k.code {
            KeyCode::Up => self.choose_note(step(-1)),
            KeyCode::Down => self.choose_note(step(1)),
            KeyCode::PageUp => self.choose_note(step(-8)),
            KeyCode::PageDown => self.choose_note(step(8)),
            KeyCode::Home => self.choose_note(shown.first().copied()),
            KeyCode::End => self.choose_note(shown.last().copied()),
            KeyCode::Esc | KeyCode::Tab | KeyCode::Enter => {
                if has {
                    self.go_to_note(sel);
                }
                self.notes_view.focus = false;
            }
            KeyCode::Char(' ') if has => {
                let n = &mut self.notes.list[sel];
                n.state = if n.state == State::Done {
                    State::Open
                } else {
                    State::Done
                };
                self.save_notes();
                self.pick_shown_note();
            }
            KeyCode::Char('u') if has => self.apply_note(sel),
            // Its part, and all that are shown.
            KeyCode::Char('g') if has => {
                let group = self.notes.list[sel].group.clone();
                let part: Vec<usize> = shown
                    .iter()
                    .copied()
                    .filter(|&i| self.notes.list[i].group == group)
                    .collect();
                self.apply_notes(&part);
            }
            KeyCode::Char('U') => self.apply_notes(&shown),
            KeyCode::Char('r') if has => self.popup = Popup::Form(self.reply_form(sel)),
            KeyCode::Char('e') if has => self.popup = Popup::Form(self.edit_note_form(sel)),
            KeyCode::Char('n') => self.new_note(),
            KeyCode::Char('h') if has => {
                let n = &mut self.notes.list[sel];
                n.state = if n.state == State::Hidden {
                    State::Open
                } else {
                    State::Hidden
                };
                let hidden = n.state == State::Hidden;
                self.save_notes();
                if hidden && self.notes_view.show == Show::Open {
                    self.say(
                        lang::l(
                            "Gizlendi: s ile Hepsi'nde görünür.",
                            "Hidden: s shows it under All.",
                        ),
                        false,
                    );
                    self.choose_note(step(1).filter(|&i| i != sel).or(step(-1)));
                }
            }
            KeyCode::Char('a') if has => {
                let n = &mut self.notes.list[sel];
                n.archived = !n.archived;
                let archived = n.archived;
                self.save_notes();
                if archived != (self.notes_view.show == Show::Archive) {
                    self.say(
                        if archived {
                            lang::l(
                                "Arşive gitti: s ile Arşiv'de görünür.",
                                "Archived: s shows it under Archive.",
                            )
                        } else {
                            lang::l("Arşivden çıktı.", "Out of the archive.")
                        },
                        false,
                    );
                    self.choose_note(step(1).filter(|&i| i != sel).or(step(-1)));
                }
            }
            KeyCode::Char('x') | KeyCode::Delete if has => {
                if armed == Some(Armed::DeleteNote) {
                    self.notes.list.remove(sel);
                    self.save_notes();
                    self.pick_shown_note();
                    self.say(lang::l("Not silindi.", "Note deleted."), false);
                } else {
                    self.armed = Some(Armed::DeleteNote);
                    self.say(
                        lang::l(
                            "x yine basılırsa not silinir (arşiv için a).",
                            "x again deletes the note (a archives it).",
                        ),
                        true,
                    );
                }
            }
            KeyCode::Char('f') => {
                let authors = self.notes.authors();
                let next = match &self.notes_view.filter {
                    None => authors.first().cloned(),
                    Some(a) => authors
                        .iter()
                        .position(|x| x == a)
                        .and_then(|i| authors.get(i + 1))
                        .cloned(),
                };
                self.say(
                    &match &next {
                        Some(a) => lang::f("Yalnız {} notları.", "Only {}'s notes.", &[a]),
                        None => lang::l("Bütün notlar.", "All notes.").to_string(),
                    },
                    false,
                );
                self.notes_view.filter = next;
                self.pick_shown_note();
            }
            KeyCode::Char('s') => {
                self.notes_view.show = self.notes_view.show.next();
                self.pick_shown_note();
            }
            KeyCode::Char('p') => {
                let text = self
                    .clipboard
                    .as_mut()
                    .and_then(|c| c.get_text().ok())
                    .unwrap_or_else(|| self.copied.clone());
                self.take_notes(&text);
            }
            KeyCode::Char('i') => self.pick_notes_file(),
            KeyCode::Char('c') => self.notes_pdf(),
            KeyCode::Char('w') => self.notes_docx(),
            KeyCode::Char('o') => {
                let pdf = dnotes::copy_path(&self.path);
                if !pdf.is_file() {
                    self.say(
                        lang::l(
                            "Önce c ile notlu kopyayı derle.",
                            "Build the copy with c first.",
                        ),
                        true,
                    );
                } else if let Err(e) = build::open(&pdf) {
                    self.say(&format!("{e:#}"), true);
                }
            }
            // Nothing else types into the play while the panel listens.
            KeyCode::Char(_) | KeyCode::Backspace | KeyCode::Left | KeyCode::Right => {}
            _ => return false,
        }
        true
    }

    fn choose_note(&mut self, i: Option<usize>) {
        if let Some(i) = i {
            self.notes_view.sel = i;
            self.go_to_note(i);
        }
    }

    /// The cursor to the note's place.
    fn go_to_note(&mut self, i: usize) {
        let Some(Some(a)) = self.places().anchors.get(i).cloned() else {
            return;
        };
        if a.range.start < self.doc.parts.body.start && !self.preamble_open {
            self.preamble_open = true;
            self.relayout();
        }
        self.go(a.range.start, false);
        self.snap_forward();
        self.keep_visible();
    }

    /// `u`: the note's corrections made, the note done.
    /// The corrections of these notes, in one undo.
    fn apply_notes(&mut self, which: &[usize]) {
        if !self.editable() {
            return;
        }
        let (new, done, left) = dnotes::apply_fixes(&self.buf.text, &mut self.notes, which);
        if done > 0 {
            self.replace_text(new);
            self.save_notes();
        }
        let msg = lang::f(
            "{} düzeltme uygulandı, {} tanesi açık kaldı (^Z geri alır).",
            "{} corrections applied, {} left open (^Z undoes them).",
            &[&done, &left],
        );
        self.say(&msg, false);
    }

    fn apply_note(&mut self, i: usize) {
        let note = self.notes.list[i].clone();
        let edits = match dnotes::checked_fixes(&self.buf.text, &note) {
            Ok(e) => e,
            Err(r) => {
                let msg = match r {
                    dnotes::Refused::Nothing => lang::l(
                        "Bu notta uygulanacak öneri yok (“yanlış” → doğru gibi).",
                        "This note suggests nothing to apply (“wrong” → right).",
                    )
                    .to_string(),
                    dnotes::Refused::NotFound => lang::l(
                        "Önerinin yanlışı metinde bulunamadı: belki düzeltilmiş.",
                        "The words to change aren't in the text: put right already?",
                    )
                    .to_string(),
                    dnotes::Refused::Breaks { what, line } => lang::f(
                        "Uygulanmadı: {}. satırda {} Elle düzelt (e ile notu da).",
                        "Not applied: at line {}, {} Fix it by hand (e edits the note).",
                        &[&line, &what],
                    ),
                };
                self.say(&msg, true);
                return;
            }
        };
        if !self.editable() {
            return;
        }
        let what: Vec<String> = edits
            .iter()
            .map(|e| format!("{} → {}", &self.buf.text[e.range.clone()], e.text))
            .collect();
        let first = edits.iter().map(|e| e.range.start).min().unwrap_or(0);
        self.buf.apply(&edits, first);
        self.raw_here = false;
        self.edited();
        self.keep_visible();
        self.notes.list[i].state = State::Done;
        self.save_notes();
        self.say(
            &lang::f(
                "Düzeltildi: {} (^Z geri alır).",
                "Changed: {} (^Z undoes it).",
                &[&what.join("; ")],
            ),
            false,
        );
    }

    /// Alt+N, or `n` in the panel: a note on the selection or the word at
    /// the cursor.
    pub(super) fn new_note(&mut self) {
        let text = &self.buf.text;
        let quote = match self.buf.selected_text() {
            Some(s) if !s.contains('\n') && !s.trim().is_empty() => {
                let s = s.trim();
                (s.chars().count() <= 120).then(|| s.to_string())
            }
            _ => {
                let at = self.buf.cursor.min(text.len());
                let is_word = |c: char| c.is_alphanumeric() || c == '\'' || c == '’';
                let start = text[..at]
                    .char_indices()
                    .rev()
                    .take_while(|(_, c)| is_word(*c))
                    .last()
                    .map_or(at, |(i, _)| i);
                let end = text[at..]
                    .char_indices()
                    .find(|(_, c)| !is_word(*c))
                    .map_or(text.len(), |(i, _)| at + i);
                let w = &text[start..end];
                (!w.is_empty() && !text[..start].ends_with('\\')).then(|| w.to_string())
            }
        };
        let at = self.buf.cursor;
        let line = self.places().index.line_of(at);
        let value = quote.map(|q| format!("“{q}” ")).unwrap_or_default();
        let mut note = Field::text("Not", &value, Vec::new());
        note.at = None;
        self.notes_view.open = true;
        self.popup = Popup::Form(Form {
            kind: FormKind::NoteNew(line),
            title: lang::f("not: {}. satır", "note: line {}", &[&line]),
            fields: vec![self.who_field(&self.me.clone()), note],
            focus: 1,
            at: 0..0,
            error: None,
            confirmed: false,
        });
    }

    fn who_field(&self, value: &str) -> Field {
        let mut options = self.notes.authors();
        if !self.me.is_empty() && !options.contains(&self.me) {
            options.insert(0, self.me.clone());
        }
        Field::text("Kimden", value, options)
    }

    fn reply_form(&self, i: usize) -> Form {
        Form {
            kind: FormKind::NoteReply(i),
            title: lang::f(
                "cevap: {}",
                "reply to {}",
                &[&popup::fit(&self.notes.list[i].text, 40)],
            ),
            fields: vec![
                self.who_field(&self.me),
                Field::text("Cevap", "", Vec::new()),
            ],
            focus: 1,
            at: 0..0,
            error: None,
            confirmed: false,
        }
    }

    fn edit_note_form(&self, i: usize) -> Form {
        let n = &self.notes.list[i];
        let mut fields = vec![
            self.who_field(&n.author),
            Field::text(
                "Satır",
                &n.line.map(|l| l.to_string()).unwrap_or_default(),
                Vec::new(),
            ),
            Field::text("Not", &n.text, Vec::new()),
        ];
        // Its replies, each a field; emptied, one goes.
        for r in &n.replies {
            let label = if r.author.is_empty() {
                "↳".to_string()
            } else {
                format!("↳ {}", r.author)
            };
            fields.push(Field::text(&label, &r.text, Vec::new()));
        }
        Form {
            kind: FormKind::NoteEdit(i),
            title: lang::l("notu düzenle", "edit the note").into(),
            fields,
            focus: 2,
            at: 0..0,
            error: None,
            confirmed: false,
        }
    }

    /// Notes out of a pasted reply or a file: asked who they are from.
    pub(super) fn take_notes(&mut self, text: &str) {
        let looks_ours = text
            .lines()
            .any(|l| l.starts_with("- [") || l.starts_with("* ["));
        let found = if looks_ours {
            Notes::parse(text).list
        } else {
            dnotes::from_reply(text, "")
        };
        if found.is_empty() {
            self.say(
                lang::l(
                    "Not bulunamadı: satır numaralı maddeler arıyorum (139: …, - 20: …).",
                    "No notes found: I look for numbered items (139: …, - 20: …).",
                ),
                true,
            );
            return;
        }
        let from = self
            .notes_view
            .last_from
            .clone()
            .unwrap_or_else(|| "AI".to_string());
        let n = found.len();
        let groups = {
            let mut g: Vec<&str> = found.iter().map(|n| n.group.as_str()).collect();
            g.dedup();
            g.retain(|g| !g.is_empty());
            g.len()
        };
        let title = if groups > 0 {
            lang::f(
                "{} not, {} başlık altında: kimden?",
                "{} notes under {} headings: from whom?",
                &[&n, &groups],
            )
        } else {
            lang::f("{} not: kimden?", "{} notes: from whom?", &[&n])
        };
        self.popup = Popup::Form(Form {
            kind: FormKind::NotesTake(found),
            title,
            fields: vec![self.who_field(&from)],
            focus: 0,
            at: 0..0,
            error: None,
            confirmed: false,
        });
    }

    /// `i`: a file of notes, from the desktop's chooser or by its path.
    pub(super) fn pick_notes_file(&mut self) {
        if !super::super::importing::can_choose() {
            self.popup = Popup::Form(self.notes_path_form());
            return;
        }
        let (tx, rx) = channel();
        let dir = self
            .path
            .parent()
            .map(Path::to_path_buf)
            .filter(|d| d.is_dir())
            .or_else(dirs::download_dir)
            .unwrap_or_else(|| PathBuf::from("/"));
        std::thread::spawn(move || {
            let chosen = rfd::FileDialog::new()
                .set_title(lang::l("Not dosyası", "A file of notes"))
                .add_filter(lang::l("Metin", "Text"), &["md", "markdown", "txt"])
                .set_directory(dir)
                .pick_file();
            let _ = tx.send(chosen);
        });
        self.notes_view.picking = Some(rx);
        self.say(lang::l("Dosya seçiliyor…", "Choosing a file…"), false);
    }

    fn notes_path_form(&self) -> Form {
        let dir = self
            .path
            .parent()
            .map(|d| format!("{}/", dactile::config::tilde(d)))
            .unwrap_or_default();
        Form {
            kind: FormKind::NotesPath,
            title: lang::l("not dosyası (.md .txt)", "a file of notes (.md .txt)").into(),
            fields: vec![Field::text("Dosya", &dir, Vec::new())],
            focus: 0,
            at: 0..0,
            error: None,
            confirmed: false,
        }
    }

    /// The chooser's answer, when it comes; none in time: the path form.
    pub(super) fn tick_notes(&mut self) {
        let Some(rx) = &self.notes_view.picking else {
            return;
        };
        match rx.try_recv() {
            Ok(Some(p)) => {
                self.notes_view.picking = None;
                self.read_notes_file(&p);
            }
            Ok(None) | Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.notes_view.picking = None;
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
        }
    }

    fn read_notes_file(&mut self, p: &Path) {
        match std::fs::read(p) {
            Ok(bytes) => {
                let text = String::from_utf8_lossy(&bytes).to_string();
                self.take_notes(&text);
            }
            Err(e) => self.say(&format!("{}: {e}", p.display()), true),
        }
    }

    // ------------------------------------------------------------ forms

    pub(super) fn submit_note(&mut self, form: Form) -> Result<(), Box<(Form, String)>> {
        let from = form.get("Kimden").to_string();
        match form.kind.clone() {
            FormKind::NoteNew(line) => {
                let text = form.get("Not").to_string();
                if text.is_empty()
                    || text.trim_matches(['“', '”', ' ']).is_empty() && !text.contains(' ')
                {
                    return Err(Box::new((
                        form,
                        lang::l("Not boş.", "The note is empty.").into(),
                    )));
                }
                if !from.is_empty() {
                    self.me_used(&from);
                }
                self.notes.list.push(Note {
                    line: Some(line),
                    author: from,
                    text,
                    ..Note::default()
                });
                self.notes_view.sel = self.notes.list.len() - 1;
                self.notes_view.open = true;
                self.save_notes();
            }
            FormKind::NoteReply(i) => {
                let text = form.get("Cevap").to_string();
                if text.is_empty() {
                    return Err(Box::new((
                        form,
                        lang::l("Cevap boş.", "The reply is empty.").into(),
                    )));
                }
                if let Some(n) = self.notes.list.get_mut(i) {
                    n.replies.push(dnotes::Reply { author: from, text });
                }
                self.save_notes();
            }
            FormKind::NoteEdit(i) => {
                let line = form.get("Satır");
                let line = if line.is_empty() {
                    None
                } else {
                    match line.parse::<usize>() {
                        Ok(l) if l > 0 => Some(l),
                        _ => {
                            return Err(Box::new((
                                form,
                                lang::l("Satır bir sayı olmalı.", "The line is a number.").into(),
                            )));
                        }
                    }
                };
                let text = form.get("Not").to_string();
                if text.is_empty() {
                    return Err(Box::new((
                        form,
                        lang::l(
                            "Not boş: silmek için panelde x.",
                            "The note is empty: x in the panel deletes it.",
                        )
                        .into(),
                    )));
                }
                if let Some(n) = self.notes.list.get_mut(i) {
                    n.author = from;
                    n.line = line;
                    n.text = text;
                    let replies: Vec<String> = form.fields[3..]
                        .iter()
                        .map(|f| f.value.trim().to_string())
                        .collect();
                    let old = std::mem::take(&mut n.replies);
                    n.replies = old
                        .into_iter()
                        .zip(replies)
                        .filter(|(_, t)| !t.is_empty())
                        .map(|(r, text)| dnotes::Reply { text, ..r })
                        .collect();
                }
                self.save_notes();
            }
            FormKind::NotesTake(found) => {
                if from.is_empty() {
                    return Err(Box::new((
                        form,
                        lang::l(
                            "Kimden geldiğini yaz (Claude, Atölye, bir ad…).",
                            "Say who they are from (an AI, a workshop, a name…).",
                        )
                        .into(),
                    )));
                }
                self.notes_view.last_from = Some(from.clone());
                let n = found.len();
                let first = self.notes.list.len();
                for mut note in found {
                    if note.author.is_empty() {
                        note.author = from.clone();
                    }
                    self.notes.list.push(note);
                }
                self.notes_view.sel = first;
                self.notes_view.open = true;
                self.notes_view.focus = true;
                self.save_notes();
                self.relayout();
                self.snap_back();
                self.go_to_note(first);
                self.say(
                    &lang::f("{} not eklendi ({}).", "{} notes added ({}).", &[&n, &from]),
                    false,
                );
            }
            FormKind::NotesPath => {
                let p = dactile::config::expand(form.get("Dosya"));
                if !p.is_file() {
                    return Err(Box::new((
                        form,
                        lang::l("Böyle bir dosya yok.", "No such file.").into(),
                    )));
                }
                self.read_notes_file(&p);
            }
            _ => {}
        }
        Ok(())
    }

    /// `c`: the play as a PDF with the open notes in its margin, beside it.
    pub(super) fn notes_pdf(&mut self) {
        let open = self
            .notes
            .list
            .iter()
            .any(|n| n.state == State::Open && !n.archived);
        if !open {
            self.say(lang::l("Açık not yok.", "No open notes."), true);
            return;
        }
        if self.aside.is_some() {
            self.say(
                lang::l(
                    "Bir derleme sürüyor; bitince yine dene.",
                    "A build is running; try again after it.",
                ),
                true,
            );
            return;
        }
        let tex = dnotes::copy_tex(&self.buf.text, &self.doc, &self.notes, false);
        let aux = build::casts(&self.doc.outline);
        let out = dnotes::copy_path(&self.path);
        let play = self.path.clone();
        self.notes_view.building = true;
        self.start_aside(move || build::build_aside(&play, &tex, &aux, &out).map(|b| (b.pdf, 1)));
    }

    /// The play with its open notes as Word's comments, beside it, opened.
    pub(super) fn notes_docx(&mut self) {
        let open = self
            .notes
            .list
            .iter()
            .any(|n| n.state == State::Open && !n.archived);
        if !open {
            self.say(lang::l("Açık not yok.", "No open notes."), true);
            return;
        }
        match dactile::word::export_with_notes(&self.path, &self.buf.text, &self.notes) {
            Ok(p) => {
                let said = lang::f(
                    "Notlu Word belgesi yazıldı: {}",
                    "Word document with notes written: {}",
                    &[&p.display()],
                );
                self.say(&said, false);
                if let Err(e) = build::launch(&p) {
                    self.say(&format!("{e:#}"), true);
                }
            }
            Err(e) => self.say(&format!("{e:#}"), true),
        }
    }

    /// The name last written as "from" for one's own notes.
    fn me_used(&mut self, name: &str) {
        if self.me.is_empty() {
            self.me = name.to_string();
        }
    }

    // ------------------------------------------------------------ drawing

    pub(super) fn draw_notes(&mut self, f: &mut Frame, area: Rect) {
        let w = NOTES_W.min(area.width);
        let r = Rect {
            x: area.x + area.width - w,
            width: w,
            ..area
        };
        let p = self.palette();
        let (bg, fg) = (theme::BG, theme::FG);
        let base = Style::default().bg(bg).fg(fg);
        f.render_widget(Paragraph::new("").style(base), r);
        // The border on the left, lit while the panel has the keys.
        let edge = if self.notes_view.focus { CLARET } else { FAINT };
        for y in r.y..r.y + r.height {
            f.buffer_mut()[(r.x, y)]
                .set_char('│')
                .set_style(Style::default().bg(bg).fg(edge));
        }
        let inner = Rect {
            x: r.x + 2,
            width: r.width.saturating_sub(3),
            ..r
        };
        let tw = inner.width as usize;
        let shown = self.shown_notes();
        let gone = self.places().gone.clone();
        let tab = |show: Show, name: &str| {
            let n = self.notes.list.iter().filter(|n| show.lists(n)).count();
            let color = if self.notes_view.show == show {
                BEIGE
            } else {
                FAINT
            };
            Span::styled(format!("  {name} {n}"), Style::default().fg(color))
        };
        let mut head = vec![
            Span::styled(
                lang::l("Notlar", "Notes"),
                Style::default().fg(BEIGE).add_modifier(Modifier::BOLD),
            ),
            tab(Show::Open, lang::l("Açık", "Open")),
            tab(Show::All, lang::l("Hepsi", "All")),
            tab(Show::Archive, lang::l("Arşiv", "Archive")),
        ];
        if let Some(a) = &self.notes_view.filter {
            head.push(Span::styled(
                format!("  {a}"),
                Style::default().fg(self.author_color(a)),
            ));
        }
        f.render_widget(
            Paragraph::new(Line::from(head)).style(base),
            Rect { height: 1, ..inner },
        );
        // Every row of the list, then the window of it that shows.
        let mut rows: Vec<(Option<usize>, Line<'static>)> = Vec::new();
        let mut last_group: Option<(bool, String)> = None;
        for &i in &shown {
            let n = &self.notes.list[i];
            let group = (n.archived, n.group.clone());
            if last_group.as_ref() != Some(&group) {
                let name = match (n.archived, n.group.is_empty()) {
                    (true, true) => lang::l("Arşiv", "Archive").to_string(),
                    (true, false) => format!("{} · {}", lang::l("Arşiv", "Archive"), n.group),
                    (false, true) => String::new(),
                    (false, false) => n.group.clone(),
                };
                if !rows.is_empty() {
                    rows.push((None, Line::from("")));
                }
                if !name.is_empty() {
                    rows.push((
                        None,
                        Line::from(Span::styled(
                            popup::fit(&name, tw),
                            Style::default().fg(MUTED).add_modifier(Modifier::ITALIC),
                        )),
                    ));
                }
                last_group = Some(group);
            }
            let chosen = i == self.notes_view.sel;
            let color = self.author_color(&n.author);
            let (mark, mark_style) = match n.state {
                State::Open => ("○", Style::default().fg(color)),
                State::Done => ("✓", Style::default().fg(MUTED)),
                State::Hidden => ("·", Style::default().fg(FAINT)),
            };
            let mut top = vec![
                Span::styled(format!("{mark} "), mark_style),
                Span::styled(
                    n.line
                        .map(|l| format!("{l:>4} "))
                        .unwrap_or_else(|| "     ".into()),
                    Style::default().fg(FAINT),
                ),
                Span::styled(
                    if n.author.is_empty() {
                        "?".to_string()
                    } else {
                        n.author.clone()
                    },
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ),
            ];
            if gone.get(i).copied().unwrap_or(false) {
                top.push(Span::styled(
                    lang::l("  düzeltilmiş gibi", "  looks fixed"),
                    Style::default().fg(WARN),
                ));
            }
            if !n.fixes().is_empty() && n.state == State::Open {
                top.push(Span::styled("  u", Style::default().fg(FAINT)));
            }
            rows.push((Some(i), Line::from(top)));
            let mut text_style = Style::default().fg(match n.state {
                State::Open => fg,
                State::Done => MUTED,
                State::Hidden => FAINT,
            });
            if n.state == State::Done {
                text_style = text_style.add_modifier(Modifier::CROSSED_OUT);
            }
            let body = wrap(&n.text, tw.saturating_sub(2));
            let keep = if chosen {
                body.len()
            } else {
                2.min(body.len())
            };
            for (k, l) in body.iter().take(keep).enumerate() {
                let l = if k + 1 == keep && keep < body.len() {
                    format!("{}…", popup::fit(l, tw.saturating_sub(3)))
                } else {
                    l.clone()
                };
                rows.push((
                    Some(i),
                    Line::from(Span::styled(format!("  {l}"), text_style)),
                ));
            }
            if chosen {
                for rp in &n.replies {
                    let who = if rp.author.is_empty() {
                        "?"
                    } else {
                        &rp.author
                    };
                    let lines = wrap(&format!("{who}: {}", rp.text), tw.saturating_sub(4));
                    for (k, l) in lines.into_iter().enumerate() {
                        let lead = if k == 0 { "  ↳ " } else { "    " };
                        rows.push((
                            Some(i),
                            Line::from(vec![
                                Span::styled(lead, Style::default().fg(FAINT)),
                                Span::styled(l, Style::default().fg(self.author_color(&rp.author))),
                            ]),
                        ));
                    }
                }
            } else if !n.replies.is_empty() {
                rows.push((
                    Some(i),
                    Line::from(Span::styled(
                        format!("  ↳ {}", n.replies.len()),
                        Style::default().fg(FAINT),
                    )),
                ));
            }
        }
        let foot: Vec<&str> = if self.notes_view.focus {
            vec![
                lang::l(
                    "␣ çözüldü  u uygula  r cevap  e düzenle",
                    "␣ done  u apply  r reply  e edit",
                ),
                lang::l(
                    "n yeni  h gizle  a arşiv  x sil  f süz  s görünüm",
                    "n new  h hide  a archive  x delete  f filter  s view",
                ),
                lang::l(
                    "p yapıştır  i dosya  c notlu PDF  o aç  w notlu Word",
                    "p paste  i file  c PDF with notes  o open  w Word with notes",
                ),
                lang::l("Esc metne  ^N kapat", "Esc to text  ^N close"),
            ]
        } else {
            vec![lang::l(
                "^N notlara geç   Alt+N buraya not",
                "^N to the notes   Alt+N note here",
            )]
        };
        let list_h = inner.height.saturating_sub(2 + foot.len() as u16 + 1) as usize;
        if rows.is_empty() {
            let empty = lang::l(
                "Not yok.\n\np  panodan yapıştır (bir AI'ın ya da okurun cevabı)\ni  bir dosyadan al\nn  imlecin yerine not yaz",
                "No notes.\n\np  paste (an AI's or a reader's reply)\ni  take a file\nn  write one at the cursor",
            );
            f.render_widget(
                Paragraph::new(empty).style(Style::default().bg(bg).fg(MUTED)),
                Rect {
                    y: inner.y + 2,
                    height: list_h as u16,
                    ..inner
                },
            );
        }
        // The chosen note's rows on screen, as much of it as fits.
        let first = rows
            .iter()
            .position(|(n, _)| *n == Some(self.notes_view.sel));
        let last = rows
            .iter()
            .rposition(|(n, _)| *n == Some(self.notes_view.sel));
        let mut top = self.notes_scroll.min(rows.len().saturating_sub(1));
        if let (Some(a), Some(b)) = (first, last) {
            // Its group's heading too, when it is the first under it.
            let a = if a >= 1 && rows[a - 1].0.is_none() {
                a - 1
            } else {
                a
            };
            if a < top {
                top = a;
            } else if b >= top + list_h {
                top = (b + 1).saturating_sub(list_h).min(a);
            }
        }
        self.notes_scroll = top;
        for (k, (n, line)) in rows.iter().skip(top).take(list_h).enumerate() {
            let y = inner.y + 2 + k as u16;
            let chosen = *n == Some(self.notes_view.sel);
            let style = if chosen {
                base.bg(theme::SELECTION)
            } else {
                base
            };
            f.render_widget(
                Paragraph::new(line.clone()).style(style),
                Rect {
                    y,
                    height: 1,
                    ..inner
                },
            );
        }
        let fy = (inner.y + inner.height).saturating_sub(foot.len() as u16);
        for (k, l) in foot.iter().enumerate() {
            f.render_widget(
                Paragraph::new(*l).style(Style::default().bg(bg).fg(FAINT)),
                Rect {
                    y: fy + k as u16,
                    height: 1,
                    ..inner
                },
            );
        }
        let _ = p;
    }
}

/// Where a note is in the text, to draw.
pub struct NoteMark {
    pub range: std::ops::Range<usize>,
    pub quoted: bool,
    pub color: Color,
    pub chosen: bool,
}

/// `s` broken into lines of at most `w` characters, at spaces.
fn wrap(s: &str, w: usize) -> Vec<String> {
    let w = w.max(8);
    let mut out = Vec::new();
    let mut line = String::new();
    for word in s.split_whitespace() {
        let wl = word.chars().count();
        let ll = line.chars().count();
        if ll > 0 && ll + 1 + wl > w {
            out.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        // A word longer than the line, cut.
        if wl > w {
            let mut chars = word.chars();
            loop {
                let part: String = chars.by_ref().take(w).collect();
                if part.is_empty() {
                    break;
                }
                if part.chars().count() == w {
                    out.push(part);
                } else {
                    line = part;
                }
            }
        } else {
            line.push_str(word);
        }
    }
    if !line.is_empty() {
        out.push(line);
    }
    out
}
