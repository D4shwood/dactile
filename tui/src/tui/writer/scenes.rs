//! ^G: the board, and hiding, trashing and bringing back scenes.

use super::*;

impl Writer {
    pub(super) fn save_sidecar(&mut self) {
        if let Err(e) = self.sidecar.save(&self.path) {
            self.say(&format!("{e:#}"), true);
        }
    }

    /// The board, on the act or scene the cursor is in.
    pub(super) fn open_board(&mut self) {
        let units = shelf::units(&self.buf.text, &self.doc);
        let at = self.buf.cursor;
        let sel = units.iter().rposition(|u| u.span.start <= at).unwrap_or(0);
        self.board_at(sel);
    }

    pub(super) fn board_at(&mut self, sel: usize) {
        self.trash = shelf::load(&shelf::trash_path(&self.path));
        self.popup = Popup::Scenes(ScenesPanel {
            sel,
            cards: self.board_cards,
            ..Default::default()
        });
    }

    /// The board on the unit that starts at `at`.
    pub(super) fn board_on(&mut self, at: usize) {
        let units = shelf::units(&self.buf.text, &self.doc);
        let sel = units
            .iter()
            .position(|u| u.span.start == at)
            .or_else(|| units.iter().rposition(|u| u.span.start <= at))
            .unwrap_or(0);
        self.board_at(sel);
    }

    pub(super) fn board_key(&mut self, k: KeyEvent, units: Vec<shelf::Unit>) {
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        let alt = k.modifiers.contains(KeyModifiers::ALT);
        let Popup::Scenes(p) = &mut self.popup else {
            return;
        };
        let n = units.len();
        let sel = p.sel.min(n.saturating_sub(1));
        if let Some(to) = p.grab {
            let places = structure::places(&units, sel);
            let i = places.iter().position(|&u| u == to).unwrap_or(0);
            match k.code {
                KeyCode::Esc => p.grab = None,
                KeyCode::Up | KeyCode::Left => p.grab = places.get(i.saturating_sub(1)).copied(),
                KeyCode::Down | KeyCode::Right => {
                    p.grab = places
                        .get((i + 1).min(places.len().saturating_sub(1)))
                        .copied()
                }
                KeyCode::Enter => {
                    p.grab = None;
                    self.move_units(&units, sel, to);
                }
                _ => {}
            }
            return;
        }
        let kinds: Vec<shelf::UnitKind> = units.iter().map(|u| u.kind).collect();
        match k.code {
            KeyCode::Esc => self.popup = Popup::None,
            KeyCode::Char('g') if ctrl => self.popup = Popup::None,
            KeyCode::Tab => {
                p.cards = !p.cards;
                self.board_cards = p.cards;
            }
            KeyCode::Char('c' | 'ç') => p.trash = true,
            KeyCode::Up | KeyCode::Down if alt => {
                let down = k.code == KeyCode::Down;
                match structure::step(&units, sel, down) {
                    Some(to) => self.move_units(&units, sel, to),
                    None => self.say(lang::l("Daha öteye gitmez.", "It can't go further."), false),
                }
            }
            KeyCode::Left | KeyCode::Right if alt => {
                // A scene to the end of the act before or after.
                let right = k.code == KeyCode::Right;
                let own = (0..=sel).rev().find(|&u| kinds[u] == shelf::UnitKind::Act);
                let to = if kinds.get(sel) != Some(&shelf::UnitKind::Scene) {
                    structure::step(&units, sel, right)
                } else if right {
                    let next = (sel + 1..n).find(|&u| kinds[u] == shelf::UnitKind::Act);
                    next.map(|a| {
                        (a + 1..n)
                            .find(|&u| kinds[u] == shelf::UnitKind::Act)
                            .unwrap_or(n)
                    })
                } else {
                    own
                };
                match to.filter(|t| structure::places(&units, sel).contains(t)) {
                    Some(to) => self.move_units(&units, sel, to),
                    None => self.say(lang::l("Daha öteye gitmez.", "It can't go further."), false),
                }
            }
            KeyCode::Up => p.sel = sel.saturating_sub(1),
            KeyCode::Down => p.sel = (sel + 1).min(n.saturating_sub(1)),
            KeyCode::Left | KeyCode::Right if p.cards => {
                p.sel = board::neighbour(&kinds, sel, k.code == KeyCode::Right);
            }
            KeyCode::Home => p.sel = 0,
            KeyCode::End => p.sel = n.saturating_sub(1),
            KeyCode::Char(' ') => {
                if let Some(u) = units.get(sel) {
                    if !self.marks.remove(&u.span.start) {
                        self.marks.insert(u.span.start);
                    }
                    p.sel = (sel + 1).min(n.saturating_sub(1));
                }
            }
            KeyCode::Char('m') if n > 0 => p.grab = Some(sel),
            KeyCode::Char('h') => self.hide_or_show(&units, sel),
            KeyCode::Char('t') => self.throw_away(&units, sel),
            KeyCode::Char('e') => {
                if let Some(u) = units.get(sel).filter(|u| u.kind == shelf::UnitKind::Scene) {
                    let at = u.span.start;
                    self.summary_form(at, true);
                }
            }
            KeyCode::Char('n') => {
                let to = if n == 0 { 0 } else { sel + 1 };
                self.new_card_form(to);
            }
            KeyCode::Char('j') => {
                if let Some(k) = units
                    .get(sel)
                    .filter(|u| u.kind == shelf::UnitKind::Scene && !u.hidden)
                    .and_then(|u| self.doc.scene_of(self.doc.block_at(u.span.start)?))
                {
                    self.join(k);
                    let at = units[sel].span.start;
                    self.board_on(at);
                }
            }
            KeyCode::Enter => {
                if let Some(u) = units.get(sel).cloned() {
                    self.popup = Popup::None;
                    self.enter_unit(&u);
                }
            }
            _ => {}
        }
        if let Popup::Scenes(p) = &mut self.popup {
            let n = shelf::units(&self.buf.text, &self.doc).len();
            p.sel = p.sel.min(n.saturating_sub(1));
        }
    }

    /// To a unit from the board: the first line after its heading. An empty
    /// hidden scene, a new card, is shown to be written in.
    pub(super) fn enter_unit(&mut self, u: &shelf::Unit) {
        let nl = source::line_ending(&self.buf.text).to_string();
        let empty = u.hidden
            && u.kind == shelf::UnitKind::Scene
            && self.buf.text[u.span.clone()].lines().count() == 1;
        if empty && self.editable() {
            let shown = shelf::strip(&self.buf.text[u.span.clone()]);
            let cursor = u.span.start + shown.len() + nl.len() * 2;
            let edits = vec![
                dactile::tex::Edit::new(u.span.clone(), shown),
                dactile::tex::Edit::new(u.span.end..u.span.end, format!("{nl}{nl}")),
            ];
            self.buf.apply(&edits, cursor);
            self.raw_here = false;
            self.edited();
            self.keep_visible();
            return;
        }
        let at = u.span.start;
        if let Some(b) = self.doc.block_at(at) {
            self.go(at, false);
            self.relayout();
            if !u.hidden
                && let Some(s) = self.lay.next_stop(self.doc.blocks[b].span.end)
            {
                self.go(s.byte, false);
            }
        }
    }

    /// Before the session's first move, split or merge: the play as it is,
    /// kept as a draft. What to add to the message about it.
    pub(super) fn keep_shape(&mut self) -> String {
        if self.kept.is_some() {
            return String::new();
        }
        match library::keep_draft(&self.path, &self.buf.text) {
            Ok(p) => {
                let name = p
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                self.kept = Some(p);
                lang::f(
                    " Önceki hâli taslak olarak saklandı: {}",
                    " The play as it was is kept as a draft: {}",
                    &[&name],
                )
            }
            Err(e) => format!(" {e:#}"),
        }
    }

    /// Moves the chosen unit, or the marked ones, to place `to`.
    pub(super) fn move_units(&mut self, units: &[shelf::Unit], sel: usize, to: usize) {
        if !self.editable() {
            return;
        }
        let Some(me) = units.get(sel) else {
            return;
        };
        let picked: Vec<usize> = if self.marks.is_empty() {
            vec![sel]
        } else {
            let targets = self.targets(units, sel);
            units
                .iter()
                .enumerate()
                .filter(|(_, u)| targets.iter().any(|t| t.span == u.span))
                .map(|(i, _)| i)
                .collect()
        };
        if picked.iter().any(|&i| units[i].kind != me.kind) {
            self.say(
                lang::l(
                    "Perdeler ve sahneler birlikte taşınmaz: yalnız birini işaretle.",
                    "Acts and scenes don't move together: mark only one kind.",
                ),
                true,
            );
            return;
        }
        let text = self.buf.text.clone();
        let moved = if picked.len() == 1 {
            structure::relocate(&text, units, picked[0], to).map(|(e, at)| (vec![e], at))
        } else {
            structure::gather(&text, units, &picked, to)
        };
        let Some((edits, at)) = moved else {
            return;
        };
        let what = if picked.len() == 1 {
            units[picked[0]].name()
        } else {
            lang::f("{} parça", "{} pieces", &[&picked.len()])
        };
        let items = board::items(&text, &self.doc, units, &self.sidecar.summaries, self.wpm());
        let place = board::place_name(&items, sel, to);
        let kept = self.keep_shape();
        self.buf.apply(&edits, at);
        self.raw_here = false;
        self.edited();
        self.keep_visible();
        self.say(&format!("{what} → {place}.{kept}"), false);
        if matches!(self.popup, Popup::Scenes(_)) {
            self.board_on(at);
        }
    }

    /// Splits the cursor's scene at its line.
    pub(super) fn split_here(&mut self) {
        if !self.editable() {
            return;
        }
        let Some((e, cursor)) = structure::split(&self.buf.text, &self.doc, self.buf.cursor) else {
            self.say(
                lang::l(
                    "Bölmek için imleci sahnenin içinde, yeni sahnenin başlayacağı satıra koy.",
                    "To split, put the cursor in the scene, on the line the new one starts with.",
                ),
                false,
            );
            return;
        };
        let kept = self.keep_shape();
        self.buf.apply(&[e], cursor);
        self.raw_here = false;
        self.edited();
        self.keep_visible();
        self.say(
            &format!(
                "{}{kept}",
                lang::l(
                    "Sahne ikiye bölündü; imleç yeni başlıkta, yerini değiştirebilirsin.",
                    "The scene is split in two; the cursor is on the new heading, to change its place.",
                )
            ),
            false,
        );
    }

    /// Scene `k` and the one after it as one; their summaries join.
    pub(super) fn join(&mut self, k: usize) {
        if !self.editable() {
            return;
        }
        let text = self.buf.text.clone();
        let Some((e, second)) = structure::merge(&text, &self.doc, k) else {
            self.say(
                lang::l(
                    "Birleşecek bir sonraki sahne yok (aynı perdede, hemen ardından).",
                    "No next scene to join (in the same act, right after it).",
                ),
                false,
            );
            return;
        };
        let scene = &self.doc.outline.scenes[k];
        let head = self.doc.blocks[self.doc.body.start + scene.block]
            .span
            .start;
        let mut edits = vec![e];
        let theirs = second
            .as_ref()
            // Kept under its own id too: an undo brings the scene back.
            .and_then(|id| self.sidecar.summaries.get(id).cloned())
            .filter(|s| !s.trim().is_empty());
        if let Some(theirs) = theirs {
            let id = scene.id.clone().unwrap_or_else(|| {
                let id = structure::new_id(&text);
                edits.push(structure::give_id(&text, head, &id));
                id
            });
            let mine = self.sidecar.summaries.get(&id).cloned().unwrap_or_default();
            let joined = format!("{} {}", mine.trim(), theirs.trim())
                .trim()
                .to_string();
            self.sidecar.summaries.insert(id, joined);
            self.save_sidecar();
        }
        let label = self.doc.outline.scenes[k].title();
        let kept = self.keep_shape();
        let cursor = self.buf.cursor.min(head.max(self.buf.cursor));
        self.buf.apply(&edits, cursor);
        self.raw_here = false;
        self.edited();
        self.keep_visible();
        self.say(
            &lang::f(
                "{} ile sonraki birleşti; ikincinin yeri yönerge oldu.{}",
                "{} and the next are one; the second's place became a direction.{}",
                &[&label, &kept],
            ),
            false,
        );
    }

    /// Alt+E: the summary of the cursor's scene.
    pub(super) fn summary_here(&mut self) {
        let Some(k) = self.block().and_then(|b| self.doc.scene_of(b)) else {
            self.say(
                lang::l(
                    "İmleç bir sahnenin içinde değil.",
                    "The cursor is in no scene.",
                ),
                false,
            );
            return;
        };
        let b = self.doc.body.start + self.doc.outline.scenes[k].block;
        let at = self.doc.blocks[b].span.start;
        self.summary_form(at, false);
    }

    pub(super) fn summary_form(&mut self, at: usize, back: bool) {
        if !self.editable() {
            return;
        }
        let line_end = self.buf.text[at..]
            .find('\n')
            .map_or(self.buf.text.len(), |n| at + n);
        let now = structure::id_in(&self.buf.text[at..line_end])
            .and_then(|id| self.sidecar.summaries.get(id))
            .cloned()
            .unwrap_or_default();
        self.popup = Popup::Form(Form {
            kind: FormKind::Summary(back),
            title: lang::l(
                "sahnenin özeti (PDF'e girmez)",
                "the scene's summary (never printed)",
            )
            .into(),
            fields: vec![Field::text("Özet", &now, Vec::new())],
            focus: 0,
            at: at..at,
            error: None,
            confirmed: false,
        });
    }

    pub(super) fn new_card_form(&mut self, to: usize) {
        if !self.editable() {
            return;
        }
        self.popup = Popup::Form(Form {
            kind: FormKind::NewCard(to),
            title: lang::l(
                "yeni kart: gizli bir sahne, yazınca görünür",
                "a new card: a hidden scene, shown once written in",
            )
            .into(),
            fields: vec![
                Field::text("Yer", "", complete::places(&self.doc)),
                Field::text("Özet", "", Vec::new()),
            ],
            focus: 0,
            at: 0..0,
            error: None,
            confirmed: false,
        });
    }

    /// The summary typed for the heading at `at`: its id given first if it
    /// has none.
    pub(super) fn set_summary(&mut self, at: usize, summary: &str) {
        let text = self.buf.text.clone();
        let line_end = text[at..].find('\n').map_or(text.len(), |n| at + n);
        let id = match structure::id_in(&text[at..line_end]) {
            Some(id) => id.to_string(),
            None if summary.trim().is_empty() => return,
            None => {
                let id = structure::new_id(&text);
                let cursor = self.buf.cursor;
                let e = structure::give_id(&text, at, &id);
                let cursor = if cursor > e.range.start {
                    cursor + e.text.len()
                } else {
                    cursor
                };
                self.buf.apply(&[e], cursor);
                id
            }
        };
        if summary.trim().is_empty() {
            self.sidecar.summaries.remove(&id);
        } else {
            self.sidecar
                .summaries
                .insert(id, summary.trim().to_string());
        }
        self.save_sidecar();
        self.edited();
    }

    /// A new hidden scene at place `to` of the board.
    pub(super) fn add_card(&mut self, to: usize, place: &str, summary: &str) -> usize {
        let text = self.buf.text.clone();
        let nl = source::line_ending(&text).to_string();
        let units = shelf::units(&text, &self.doc);
        let place = dactile::tex::escape_text(place.trim());
        let hash = !info::has_option(&text, "nohash");
        let mut head = match (hash, place.is_empty()) {
            (true, true) => "#sahne".to_string(),
            (true, false) => format!("#sahne {place}"),
            (false, true) => "\\sahne".to_string(),
            (false, false) => format!("\\sahne[{place}]"),
        };
        let id = (!summary.trim().is_empty()).then(|| structure::new_id(&text));
        if let Some(id) = &id {
            head.push_str(&format!(" {}{id}", structure::MARK));
        }
        let head = format!("{} {head}", shelf::HIDDEN);
        let (e, at) = match units.get(to) {
            Some(u) => (
                dactile::tex::Edit::new(u.span.start..u.span.start, format!("{head}{nl}{nl}")),
                u.span.start,
            ),
            None => match units.last() {
                Some(u) => (
                    dactile::tex::Edit::new(u.span.end..u.span.end, format!("{nl}{nl}{head}")),
                    u.span.end + nl.len() * 2,
                ),
                None => {
                    let end = self.doc.parts.body.end;
                    (
                        dactile::tex::Edit::new(end..end, format!("{nl}{head}{nl}")),
                        end + nl.len(),
                    )
                }
            },
        };
        let cursor = self.buf.cursor;
        let cursor = if cursor >= at {
            cursor + e.text.len()
        } else {
            cursor
        };
        self.buf.apply(&[e], cursor);
        if let Some(id) = id {
            self.sidecar
                .summaries
                .insert(id, summary.trim().to_string());
            self.save_sidecar();
        }
        self.edited();
        at
    }

    // ------------------------------------------------------------ putting away

    /// The marked units, or the chosen one; a scene inside a chosen act goes
    /// with the act.
    pub(super) fn targets(&self, units: &[shelf::Unit], sel: usize) -> Vec<shelf::Unit> {
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
    pub(super) fn hide_or_show(&mut self, units: &[shelf::Unit], sel: usize) {
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
    pub(super) fn throw_away(&mut self, units: &[shelf::Unit], sel: usize) {
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
    pub(super) fn put_back(&mut self, i: usize) {
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
    pub(super) fn destroy(&mut self, i: usize) {
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

    pub(super) fn edit_form(&self, alias: &str) -> Form {
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

    pub(super) fn card_form(&self, alias: &str) -> Form {
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

    /// Which part to restyle: `Kapak: Başlık`, `Kişiler: Adlar`.
    pub(super) fn part_form(&self) -> Form {
        let cover = info::COVER_PARTS.iter().map(|(_, n)| format!("Kapak: {n}"));
        let cast = info::CAST_PARTS
            .iter()
            .map(|(_, n)| format!("Kişiler: {n}"));
        let all: Vec<String> = cover.chain(cast).collect();
        Form {
            kind: FormKind::CoverPart,
            title: lang::l("kapak ve kişiler", "cover and cast").into(),
            fields: vec![Field::text("Öğe", &all[0], all.clone())],
            focus: 0,
            at: 0..0,
            error: None,
            confirmed: false,
        }
    }

    /// How a part looks; what stays empty is the class's own.
    pub(super) fn part_look_form(&self, command: &str, part: &str) -> Form {
        let look = info::part_look(&self.buf.text, command, part);
        let words = |w: &[&str]| w.iter().map(ToString::to_string).collect();
        Form {
            kind: FormKind::PartLook(command.to_string(), part.to_string()),
            title: part.to_string(),
            fields: vec![
                Field::text("Hiza", &look.align, words(&["left", "center", "right"])),
                Field::text(
                    "Boyut",
                    &look.size,
                    words(&[
                        "tiny", "small", "normal", "large", "Large", "LARGE", "huge", "Huge",
                    ]),
                ),
                Field::text(
                    "Stil",
                    &look.style,
                    words(&[
                        "normal",
                        "bold",
                        "italic",
                        "bolditalic",
                        "smallcaps",
                        "boldsmallcaps",
                    ]),
                ),
                Field::text("Üst boşluk", &look.before, Vec::new()),
                Field::text("Alt boşluk", &look.after, Vec::new()),
            ],
            focus: 0,
            at: 0..0,
            error: None,
            confirmed: false,
        }
    }

    pub(super) fn info_form(&self) -> Form {
        let mut fields: Vec<Field> = info::FIELDS
            .iter()
            .map(|(cmd, label)| {
                let v = info::get(&self.buf.text, cmd).unwrap_or_default();
                Field::text(label, &dactile::tex::unescape_text(&v), Vec::new())
            })
            .collect();
        fields.push(Field::flag("Final (baskı)", info::is_final(&self.buf.text)));
        let sides = [
            lang::l("orta", "center"),
            lang::l("sol", "left"),
            lang::l("sağ", "right"),
        ];
        let side = match info::scene_align(&self.buf.text) {
            info::Side::Left => sides[1],
            info::Side::Right => sides[2],
            info::Side::Center => sides[0],
        };
        fields.push(Field::text(
            lang::l("Sahne başlıkları", "Scene headings"),
            side,
            sides.iter().map(ToString::to_string).collect(),
        ));
        fields.push(Field::text(
            lang::l("Yazı tipi", "Font"),
            &info::play_font(&self.buf.text),
            Vec::new(),
        ));
        fields.push(Field::text(
            lang::l("Yazı boyutu", "Text size"),
            &info::play_font_size(&self.buf.text),
            ["", "10", "11", "12", "13", "14"]
                .map(String::from)
                .to_vec(),
        ));
        fields.push(Field::flag(
            "Kişiler ayrı sayfada",
            info::cast_own_page(&self.buf.text),
        ));
        fields.push(Field::flag(
            "Kapaktan sonra içindekiler",
            info::contents_page(&self.buf.text),
        ));
        fields.push(Field::flag(
            "Latince başlık (Dramatis Personae)",
            info::latin_cast(&self.buf.text),
        ));
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
}
