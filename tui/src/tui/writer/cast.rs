//! ^O: the cast panel and splitting a character.

use super::*;

impl Writer {
    pub(super) fn wpm(&self) -> u32 {
        self.sidecar.wpm.unwrap_or(self.default_wpm)
    }

    fn others_form(&mut self) {
        if !self.editable() {
            return;
        }
        let now = dcast::others(&self.buf.text);
        self.popup = Popup::Form(Form {
            kind: FormKind::Others,
            title: lang::l(
                "kişiler listesinin altındakiler (boş: satır gider)",
                "those under the cast list (empty: the line goes)",
            )
            .into(),
            fields: vec![Field::text("Diğer kişiler", &now, Vec::new())],
            focus: 0,
            at: 0..0,
            error: None,
            confirmed: false,
        });
    }

    /// Where `alias` first speaks, the panel gone.
    fn go_to_first_line(&mut self, alias: &str) {
        let first = self.doc.blocks[self.doc.body.clone()]
            .iter()
            .find(|b| matches!(&b.kind, Kind::Line { alias: a, .. } if a == alias));
        match first.map(|b| b.span.start) {
            Some(at) => {
                self.popup = Popup::None;
                self.go(at, false);
                self.snap_forward();
                self.keep_visible();
            }
            None => self.say(
                lang::l("Bu kişi hiç konuşmuyor.", "This character never speaks."),
                false,
            ),
        }
    }

    pub(super) fn cast_entries(&self, by_lines: bool) -> Vec<Entry> {
        let st = panels::stats(&self.doc, &self.buf.text);
        panels::entries(&self.doc, &st, by_lines)
    }

    pub(super) fn cast_key(&mut self, k: KeyEvent) {
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
        // The group the selection is in: its heading or one of its members.
        let group = match chosen {
            Some(Entry::Group(g)) => Some(g),
            Some(Entry::Character(i)) if !by_lines => self.doc.cast.characters[i].group,
            _ => None,
        };
        let member = matches!(chosen, Some(Entry::Character(i)) if self.doc.cast.characters[i].group.is_some());
        match k.code {
            KeyCode::Esc => self.popup = Popup::None,
            KeyCode::Up | KeyCode::Down if alt && !by_lines && member => {
                let a = alias.clone().unwrap_or_default();
                let up = k.code == KeyCode::Up;
                if let Some(e) = dcast::move_member(&self.doc.cast, &a, up, &self.buf.text) {
                    let t = dactile::tex::apply(&self.buf.text, &e);
                    self.replace_text(t);
                    let at = self.cast_index(&a);
                    if let Popup::Cast(p) = &mut self.popup {
                        p.sel = at;
                    }
                }
            }
            KeyCode::Char('o') if ctrl => self.popup = Popup::None,
            KeyCode::Char('o') => self.others_form(),
            KeyCode::Char('f') => {
                if let Some(a) = alias {
                    self.go_to_first_line(&a);
                }
            }
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
            KeyCode::Char('n') => {
                self.popup = Popup::Form(match group {
                    Some(g) => self.member_form(g),
                    None => self.character_form("", None),
                })
            }
            KeyCode::Char('g') => self.popup = Popup::Form(self.group_form(None)),
            KeyCode::Char('t') => {
                if let Some(a) = &alias {
                    self.popup = Popup::Form(self.move_form(a));
                }
            }
            KeyCode::Char('e') if matches!(chosen, Some(Entry::Group(_))) => {
                self.popup = Popup::Form(self.group_form(group));
            }
            KeyCode::Char('d') if matches!(chosen, Some(Entry::Group(_))) => {
                let g = group.unwrap_or(0);
                if let Some(e) = dcast::dissolve_group(&self.buf.text, &self.doc.cast, g) {
                    let t = dactile::tex::apply(&self.buf.text, &[e]);
                    self.replace_text(t);
                    self.say(
                        lang::l(
                            "Grup dağıldı; üyeleri kişiler tablosunda kaldı (^Z geri alır).",
                            "The group is gone; its members stay in the cast table (^Z undoes).",
                        ),
                        false,
                    );
                }
            }
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
    pub(super) fn split_into(&mut self, p: SplitPanel) {
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

    /// Does what a palette entry says.
    pub(super) fn run(&mut self, act: PAct) {
        match act {
            PAct::Key(code, mods) => self.key(KeyEvent::new(code, mods)),
            PAct::Theme(t) => {
                self.theme = t.to_string();
                self.say(
                    &lang::f("Tema: {}", "Theme: {}", &[&theme::theme_name(t)]),
                    false,
                );
            }
            PAct::Animations => {
                self.animations = !self.animations;
                self.say(
                    if self.animations {
                        lang::l("Animasyonlar açık.", "Animations on.")
                    } else {
                        lang::l("Animasyonlar kapalı.", "Animations off.")
                    },
                    false,
                );
            }
            PAct::Board => self.open_board(),
            PAct::NotesPdf => self.notes_pdf(),
            PAct::NotesPaste => {
                let text = self
                    .clipboard
                    .as_mut()
                    .and_then(|c| c.get_text().ok())
                    .unwrap_or_else(|| self.copied.clone());
                self.take_notes(&text);
            }
            PAct::NotesFile => self.pick_notes_file(),
            PAct::Insert(s) => self.insert_snippet(s, true),
            PAct::InsertInline(s) => self.insert_snippet(s, false),
            PAct::FoldRibbon => self.fold_ribbon(),
            PAct::Info => self.popup = Popup::Backstage(0),
            PAct::PlaySettings => self.popup = Popup::PlaySettings(0),
            PAct::SaveTemplate => {
                let title = self.doc.title(&self.buf.text).unwrap_or_default();
                self.popup = Popup::Form(Form {
                    kind: FormKind::TemplateName,
                    title: lang::l(
                        "şablon olarak kaydet: perdeler, sahneler, kişiler; replikler değil",
                        "save as a template: acts, scenes, cast; not the lines",
                    )
                    .into(),
                    fields: vec![Field::text("Şablon adı", &title, Vec::new())],
                    focus: 0,
                    at: 0..0,
                    error: None,
                    confirmed: false,
                });
            }
            PAct::About => self.say(
                &lang::f(
                    "Dactile Light {} · tiyatro sınıfı {}",
                    "Dactile Light {} · the tiyatro class {}",
                    &[&env!("CARGO_PKG_VERSION"), &dactile::class::version()],
                ),
                false,
            ),
            PAct::Memorize => match self.block().and_then(|b| self.doc.speaker(b)) {
                Some(a) => self.open_memo(a.to_string(), false),
                None => self.say(
                    lang::l(
                        "İmleci bir rolün satırına götür ya da F9'da rolü seçip z.",
                        "Put the cursor on a role's line, or pick the role on F9 and press z.",
                    ),
                    true,
                ),
            },
            PAct::NewScene | PAct::NewAct => {
                if self.editable() {
                    let at = self.buf.cursor;
                    self.popup = Popup::Form(if act == PAct::NewScene {
                        self.scene_form(at)
                    } else {
                        self.act_form(at)
                    });
                }
            }
            PAct::NewCharacter => self.popup = Popup::Form(self.character_form("", None)),
        }
    }
}
