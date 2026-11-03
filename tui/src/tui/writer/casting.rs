//! F9: castings, actors, the table and memorizing.

use super::*;

impl Writer {
    /// Memorizing `role` from its first line at or after the cursor.
    pub(super) fn open_memo(&mut self, role: String, back: bool) {
        let cues = dactile::memorize::cues(&self.buf.text, &self.doc, &role);
        let ours = memorize::ours(&cues);
        let here = self.block().unwrap_or(0);
        let at = ours
            .iter()
            .copied()
            .find(|&i| cues[i].block >= here)
            .or(ours.first().copied())
            .unwrap_or(0);
        let name = self
            .doc
            .cast
            .get(&role)
            .map_or(role.clone(), |c| c.script.clone());
        self.popup = Popup::Memorize(Memo {
            role,
            name,
            at,
            shown: 0,
            back,
        });
    }

    pub(super) fn memo_key(&mut self, k: KeyEvent) {
        let Popup::Memorize(m) = &self.popup else {
            return;
        };
        let cues = dactile::memorize::cues(&self.buf.text, &self.doc, &m.role);
        let ours = memorize::ours(&cues);
        let Popup::Memorize(m) = &mut self.popup else {
            return;
        };
        let k_at = ours.iter().position(|&i| i == m.at).unwrap_or(0);
        let next = ours.get(k_at + 1).copied();
        match k.code {
            KeyCode::Esc => {
                self.popup = if m.back { Popup::Roles } else { Popup::None };
            }
            KeyCode::Char(' ') | KeyCode::Enter => {
                if m.shown < 2 {
                    m.shown += 1;
                } else if let Some(n) = next {
                    m.at = n;
                    m.shown = 0;
                } else {
                    self.say(
                        lang::l(
                            "Rolün sonu. Esc kapatır.",
                            "The end of the role. Esc closes.",
                        ),
                        false,
                    );
                }
            }
            KeyCode::Down | KeyCode::Right => {
                if let Some(n) = next {
                    m.at = n;
                    m.shown = 0;
                }
            }
            KeyCode::Backspace | KeyCode::Up | KeyCode::Left if k_at > 0 => {
                m.at = ours[k_at - 1];
                m.shown = 0;
            }
            _ => {}
        }
    }

    /// Shift+F8: the cursor's speaker's lines lit, the rest faint; again, off.
    pub(super) fn toggle_role_view(&mut self) {
        if self.role_view.take().is_some() {
            self.say(lang::l("Rol görünümü kapalı.", "Role view off."), false);
            return;
        }
        let who = self
            .block()
            .and_then(|b| self.doc.speaker(b))
            .map(str::to_string);
        match who.as_deref().and_then(|a| self.doc.cast.get(a)) {
            Some(c) => {
                self.say(
                    &lang::f(
                        "Rol görünümü: {}. Shift+F8 kapatır.",
                        "Role view: {}. Shift+F8 turns it off.",
                        &[&c.script],
                    ),
                    false,
                );
                self.role_view = who;
            }
            None => self.say(
                lang::l(
                    "İmleci bir rolün satırına götür, sonra Shift+F8.",
                    "Put the cursor on a role's line, then Shift+F8.",
                ),
                true,
            ),
        }
    }

    /// Whether block `b` stays lit in the role view: the role's lines and
    /// directions, its entrances and exits, and the headings.
    pub(super) fn role_lit(&self, b: usize) -> bool {
        let Some(role) = &self.role_view else {
            return true;
        };
        match self.doc.blocks.get(b).map(|b| &b.kind) {
            Some(Kind::Line { alias, .. }) => alias == role,
            Some(Kind::Do { aliases, .. } | Kind::Move { aliases, .. }) => aliases.contains(role),
            Some(Kind::Act { .. } | Kind::Scene { .. } | Kind::Section { .. }) => true,
            _ => false,
        }
    }

    pub(super) fn roles_key(&mut self, k: KeyEvent) {
        let n = self.doc.cast.characters.len();
        let scenes = self.doc.outline.scenes.len();
        let castings = self.sidecar.castings.len();
        let p = &mut self.roles;
        p.sel = p.sel.min(n.saturating_sub(1));
        p.row = p.row.min(scenes.saturating_sub(1));
        if p.pick {
            p.pick = false;
            use dactile::casting::CopyKind;
            let kind = match k.code {
                KeyCode::Char('1') | KeyCode::Enter => CopyKind::Full,
                KeyCode::Char('2') => CopyKind::Rehearsal,
                KeyCode::Char('3') => CopyKind::Light,
                _ => return,
            };
            self.actor_copies(kind);
            return;
        }
        if p.confirm {
            p.confirm = false;
            if k.code == KeyCode::Char('x') && castings > 0 {
                let c = self.sidecar.casting.min(castings - 1);
                self.sidecar.castings.remove(c);
                self.sidecar.casting = c.saturating_sub(1);
                self.save_sidecar();
            }
            return;
        }
        match (p.tab, k.code) {
            (_, KeyCode::Esc | KeyCode::F(9)) => self.popup = Popup::None,
            (_, KeyCode::Tab) => p.tab = p.tab.next(),
            (_, KeyCode::BackTab) => p.tab = p.tab.next().next(),
            (roles::Tab::Casting, KeyCode::Up) => p.sel = p.sel.saturating_sub(1),
            (roles::Tab::Casting, KeyCode::Down) => p.sel = (p.sel + 1).min(n.saturating_sub(1)),
            (roles::Tab::Casting, KeyCode::Left | KeyCode::Right) if castings > 1 => {
                let c = self.sidecar.casting.min(castings - 1);
                self.sidecar.casting = if k.code == KeyCode::Left {
                    (c + castings - 1) % castings
                } else {
                    (c + 1) % castings
                };
                self.save_sidecar();
            }
            (roles::Tab::Casting, KeyCode::Enter) if n > 0 => {
                let alias = self.doc.cast.characters[p.sel].alias.clone();
                self.actor_form(alias);
            }
            (roles::Tab::Casting, KeyCode::Char('n')) => self.casting_form(true),
            (roles::Tab::Casting, KeyCode::Char('r')) => self.casting_form(false),
            (roles::Tab::Casting, KeyCode::Char('x')) if castings > 0 => p.confirm = true,
            (roles::Tab::Casting, KeyCode::Char('k')) => p.pick = true,
            (roles::Tab::Casting, KeyCode::Char('z')) if n > 0 => {
                let alias = self.doc.cast.characters[p.sel].alias.clone();
                self.open_memo(alias, true);
            }
            (roles::Tab::Table, KeyCode::Up) => p.row = p.row.saturating_sub(1),
            (roles::Tab::Table, KeyCode::Down) => p.row = (p.row + 1).min(scenes.saturating_sub(1)),
            (roles::Tab::Table, KeyCode::Left) => p.col = p.col.saturating_sub(1),
            (roles::Tab::Table, KeyCode::Right) => p.col = (p.col + 1).min(n.saturating_sub(1)),
            (roles::Tab::Table, KeyCode::Enter) if scenes > 0 => {
                let b = self.doc.body.start + self.doc.outline.scenes[p.row].block;
                let at = self.doc.blocks[b].span.start;
                self.popup = Popup::None;
                self.go(at, false);
                self.relayout();
                if let Some(s) = self.lay.next_stop(self.doc.blocks[b].span.end) {
                    self.go(s.byte, false);
                }
            }
            (roles::Tab::Table, KeyCode::Char('x')) => self.table_csv(),
            (roles::Tab::Table, KeyCode::Char('p')) => self.table_pdf(),
            (_, KeyCode::Char('o')) => match &self.aside_out {
                Some(out) => {
                    if let Err(e) = build::open(out) {
                        self.say(&format!("{e:#}"), true);
                    }
                }
                None => self.say(
                    lang::l(
                        "Henüz bir tablo PDF'i ya da oyuncu kopyası yok.",
                        "No table PDF or actor copies yet.",
                    ),
                    false,
                ),
            },
            (roles::Tab::Stats, KeyCode::Up) => p.scroll = p.scroll.saturating_sub(1),
            (roles::Tab::Stats, KeyCode::Down) => p.scroll += 1,
            (roles::Tab::Stats, KeyCode::PageUp) => p.scroll = p.scroll.saturating_sub(10),
            (roles::Tab::Stats, KeyCode::PageDown) => p.scroll += 10,
            (roles::Tab::Stats, KeyCode::Char('w')) => self.wpm_form(),
            _ => {}
        }
    }

    pub(super) fn actor_form(&mut self, alias: String) {
        if !self.editable() {
            return;
        }
        let now = self.sidecar.actor(&alias).unwrap_or("").to_string();
        let script = self
            .doc
            .cast
            .get(&alias)
            .map(|c| c.script.clone())
            .unwrap_or_default();
        let casting = self
            .sidecar
            .current()
            .map(|c| c.name.clone())
            .unwrap_or_else(|| "A".into());
        // Everyone already cast somewhere, to pick with Tab.
        let mut known: Vec<String> = self
            .sidecar
            .castings
            .iter()
            .flat_map(|c| c.actors.values().cloned())
            .collect();
        known.sort();
        known.dedup();
        self.popup = Popup::Form(Form {
            kind: FormKind::Actor(alias),
            title: lang::f(
                "{} kimin? ({} dağıtımı; PDF'e girmez)",
                "Who plays {}? (casting {}; never printed)",
                &[&script, &casting],
            ),
            fields: vec![Field::text("Oyuncu", &now, known)],
            focus: 0,
            at: 0..0,
            error: None,
            confirmed: false,
        });
    }

    pub(super) fn casting_form(&mut self, new: bool) {
        if !self.editable() {
            return;
        }
        let now = if new {
            String::new()
        } else {
            self.sidecar
                .current()
                .map(|c| c.name.clone())
                .unwrap_or_default()
        };
        let title = if new && !self.sidecar.castings.is_empty() {
            lang::l(
                "yeni dağıtım (şimdikinin kopyası)",
                "a new casting (a copy of this one)",
            )
        } else if new {
            lang::l("yeni dağıtım", "a new casting")
        } else {
            lang::l("dağıtımın adı", "the casting's name")
        };
        self.popup = Popup::Form(Form {
            kind: FormKind::CastingName(new),
            title: title.into(),
            fields: vec![Field::text("Ad", &now, Vec::new())],
            focus: 0,
            at: 0..0,
            error: None,
            confirmed: false,
        });
    }

    pub(super) fn wpm_form(&mut self) {
        if !self.editable() {
            return;
        }
        self.popup = Popup::Form(Form {
            kind: FormKind::Wpm,
            title: lang::l(
                "bu oyunun konuşma hızı (süreler buna göre)",
                "this play's pace (the times follow it)",
            )
            .into(),
            fields: vec![Field::text(
                "Dakikada kelime",
                &self.wpm().to_string(),
                Vec::new(),
            )],
            focus: 0,
            at: 0..0,
            error: None,
            confirmed: false,
        });
    }

    /// The scene/role table beside the play, for a spreadsheet.
    pub(super) fn table_csv(&mut self) {
        let data = roles::Data::of(&self.buf.text, &self.doc, self.wpm());
        let text = dactile::casting::csv(&self.doc, &self.sidecar, &data.table);
        let path = dactile::casting::table_path(&self.path, "csv");
        match dactile::file::write_atomic(&path, &text) {
            Ok(()) => self.say(
                &lang::f(
                    "Tablo yazıldı: {}",
                    "The table is written: {}",
                    &[&dactile::config::tilde(&path)],
                ),
                false,
            ),
            Err(e) => self.say(&format!("{e:#}"), true),
        }
    }
}
