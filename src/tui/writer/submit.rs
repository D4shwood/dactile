//! What a filled form does to the play.

use super::*;

impl Writer {
    /// Writes what a form says into the text; Err gives the form back with
    /// what is wrong.
    pub(super) fn submit(&mut self, form: Form) -> Result<(), Box<(Form, String)>> {
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
                if self.animations {
                    self.reveal = Some((form.at.start, Instant::now()));
                }
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
                if self.animations {
                    self.reveal = Some((form.at.start, Instant::now()));
                }
            }
            FormKind::Actor(alias) => {
                let actor = form.get("Oyuncu").trim().to_string();
                if self.sidecar.castings.is_empty() {
                    self.sidecar.castings.push(Casting {
                        name: "A".into(),
                        ..Casting::default()
                    });
                    self.sidecar.casting = 0;
                }
                let c = self.sidecar.casting.min(self.sidecar.castings.len() - 1);
                let actors = &mut self.sidecar.castings[c].actors;
                if actor.is_empty() {
                    actors.remove(&alias);
                } else {
                    actors.insert(alias, actor);
                }
                self.save_sidecar();
                self.popup = Popup::Roles;
            }
            FormKind::CastingName(new) => {
                let name = form.get("Ad").trim().to_string();
                if name.is_empty() {
                    let e = lang::l(
                        "Bir ad yaz: A, B, prömiyer…",
                        "Give it a name: A, B, premiere…",
                    );
                    return Err(Box::new((form, e.into())));
                }
                if new || self.sidecar.castings.is_empty() {
                    // A new one starts as a copy of the one in use.
                    let actors = self
                        .sidecar
                        .current()
                        .map(|c| c.actors.clone())
                        .unwrap_or_default();
                    self.sidecar.castings.push(Casting { name, actors });
                    self.sidecar.casting = self.sidecar.castings.len() - 1;
                } else {
                    let c = self.sidecar.casting.min(self.sidecar.castings.len() - 1);
                    self.sidecar.castings[c].name = name;
                }
                self.save_sidecar();
                self.popup = Popup::Roles;
            }
            FormKind::Wpm => match form.get("Dakikada kelime").trim().parse::<u32>() {
                Ok(n) if (30..=400).contains(&n) => {
                    self.sidecar.wpm = Some(n);
                    self.save_sidecar();
                    self.popup = Popup::Roles;
                }
                _ => {
                    let e = lang::l("30 ile 400 arası bir sayı.", "A number from 30 to 400.");
                    return Err(Box::new((form, e.into())));
                }
            },
            FormKind::Summary(back) => {
                let at = form.at.start;
                self.set_summary(at, form.get("Özet"));
                if back {
                    self.board_on(at);
                }
            }
            FormKind::NewCard(to) => {
                let at = self.add_card(to, form.get("Yer"), form.get("Özet"));
                self.board_on(at);
                self.say(
                    lang::l(
                        "Yeni kart gizli bir sahne: PDF'te yok. ⏎ ile içine yazınca görünür.",
                        "The new card is a hidden scene, out of the PDF; ⏎ to write in it shows it.",
                    ),
                    false,
                );
            }
            FormKind::Find
            | FormKind::NewPlay
            | FormKind::OpenPath
            | FormKind::Settings
            | FormKind::Copy(_)
            | FormKind::Fork(_)
            | FormKind::Rename(_) => {}
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
                    self.replace_text(t);
                    self.say(
                        lang::l("Oyun bilgisi güncellendi.", "Cover details updated."),
                        false,
                    );
                }
            }
            FormKind::Merge(from) => return self.submit_merge(form, from),
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
            FormKind::NewMember(g) => {
                let mut spec = match self.spec_from(&form, None) {
                    Ok(s) => s,
                    Err(e) => return Err(Box::new((form, e))),
                };
                spec.kind = dcast::Kind::Member;
                spec.desc.clear();
                let e = dcast::add(&self.buf.text, &self.doc.cast, &spec, Some(g));
                let t = dactile::tex::apply(&self.buf.text, &[e]);
                self.replace_text(t);
                self.say(
                    &lang::f(
                        "{} gruba eklendi: \\{}",
                        "{} added to the group: \\{}",
                        &[&spec.script, &spec.alias],
                    ),
                    false,
                );
                self.popup = Popup::Cast(CastPanel {
                    sel: self.cast_index(&spec.alias),
                    by_lines: false,
                });
            }
            FormKind::GroupName(g) => return self.submit_group_name(form, g),
            FormKind::MoveTo(alias) => return self.submit_move_to(form, alias),
            FormKind::EditCharacter(old) => return self.submit_edit_character(form, old),
            FormKind::Character => return self.submit_new_character(form),
        }
        Ok(())
    }

    /// A character merged into another.
    fn submit_merge(&mut self, form: Form, from: String) -> Result<(), Box<(Form, String)>> {
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
            let (lines, dos) = dcast::uses(&self.buf.text, self.doc.parts.body.clone(), &from);
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
        // Its actors, where the other has none.
        for c in &mut self.sidecar.castings {
            if let Some(a) = c.actors.remove(&from) {
                c.actors.entry(into.clone()).or_insert(a);
            }
        }
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
        Ok(())
    }

    /// A group named, or a new one with its members.
    fn submit_group_name(
        &mut self,
        form: Form,
        g: Option<usize>,
    ) -> Result<(), Box<(Form, String)>> {
        let name = dactile::tex::escape_text(form.get("Grup adı").trim());
        if name.is_empty() {
            return Err(Box::new((
                form,
                lang::l("Grubun bir adı olsun.", "The group needs a name.").into(),
            )));
        }
        let mut t = self.buf.text.clone();
        let g = match g {
            Some(g) => {
                if let Some(e) = dcast::rename_group(&self.doc.cast, g, &name) {
                    t = dactile::tex::apply(&t, &[e]);
                }
                g
            }
            None => {
                // Who goes into it at once: aliases, by commas or spaces.
                let wanted: Vec<String> = form
                    .get("Üyeler (kısaltmalar)")
                    .split([',', ' '])
                    .map(|a| a.trim().trim_start_matches('\\').to_string())
                    .filter(|a| !a.is_empty())
                    .collect();
                if let Some(a) = wanted.iter().find(|a| self.doc.cast.get(a).is_none()) {
                    return Err(Box::new((
                        form,
                        lang::f("\\{} diye bir karakter yok.", "No character \\{}.", &[a]),
                    )));
                }
                t = dactile::tex::apply(&t, &[dcast::add_group(&t, &self.doc.cast, &name)]);
                let g = dcast::Cast::of_play(&t).groups.len() - 1;
                for a in &wanted {
                    let c = dcast::Cast::of_play(&t);
                    if let Some(e) = dcast::move_to_group(&t, &c, a, Some(g)) {
                        t = dactile::tex::apply(&t, &e);
                    }
                }
                g
            }
        };
        self.replace_text(t);
        self.popup = Popup::Cast(CastPanel {
            sel: self.group_index(g),
            by_lines: false,
        });
        self.say(
            lang::l(
                "Grup hazır: n ile yeni üye ekler, t ile bir karakteri içine taşırsın.",
                "The group is ready: n adds a new member, t moves a character into it.",
            ),
            false,
        );
        Ok(())
    }

    /// A character moved into a group, or out of one.
    fn submit_move_to(&mut self, form: Form, alias: String) -> Result<(), Box<(Form, String)>> {
        let want = form.get("Grup").trim().to_string();
        let none = want.is_empty() || want == lang::l("(grup yok)", "(no group)");
        let groups = &self.doc.cast.groups;
        let mut t = self.buf.text.clone();
        let target = if none {
            None
        } else if let Some(g) = groups.iter().position(|g| g.desc == want) {
            Some(g)
        } else {
            // A name that isn't a group yet: a new one.
            let name = dactile::tex::escape_text(&want);
            t = dactile::tex::apply(&t, &[dcast::add_group(&t, &self.doc.cast, &name)]);
            Some(groups.len())
        };
        let c = dcast::Cast::of_play(&t);
        if let Some(e) = dcast::move_to_group(&t, &c, &alias, target) {
            t = dactile::tex::apply(&t, &e);
        }
        let lost = target.is_some()
            && self
                .doc
                .cast
                .get(&alias)
                .and_then(|c| c.desc.as_deref())
                .is_some_and(|d| !d.is_empty());
        self.replace_text(t);
        if lost {
            self.say(
                lang::l(
                    "Taşındı; grup üyelerinin açıklaması olmaz, açıklaması gitti (^Z geri alır).",
                    "Moved; group members have no description, so it went (^Z undoes).",
                ),
                false,
            );
        }
        self.popup = Popup::Cast(CastPanel {
            sel: self.cast_index(&alias),
            by_lines: false,
        });
        Ok(())
    }

    /// A character changed, renamed everywhere if its alias changed.
    fn submit_edit_character(
        &mut self,
        form: Form,
        old: String,
    ) -> Result<(), Box<(Form, String)>> {
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
        self.replace_text(t);
        self.popup = Popup::Cast(CastPanel {
            sel: self.cast_index(&spec.alias),
            by_lines: false,
        });
        Ok(())
    }

    /// A new character, and its first use where it was typed.
    fn submit_new_character(&mut self, form: Form) -> Result<(), Box<(Form, String)>> {
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
                format!("\\{alias} kullanılıyor ya da bir LaTeX komutu; başka bir kısaltma seç."),
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
        Ok(())
    }

    /// The character a form describes. `old` is the alias being edited.
    pub(super) fn spec_from(&self, form: &Form, old: Option<&str>) -> Result<Spec, String> {
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

    /// Where a group's heading is in the cast panel.
    pub(super) fn group_index(&self, g: usize) -> usize {
        self.cast_entries(false)
            .iter()
            .position(|e| *e == Entry::Group(g))
            .unwrap_or(0)
    }

    /// The whole text replaced in one undo step (changes in the preamble);
    /// the cursor keeps its place counted from the end.
    pub(super) fn replace_text(&mut self, t: String) {
        if t == self.buf.text || !self.editable() {
            return;
        }
        let from_end = self.buf.text.len() - self.buf.cursor;
        let all = 0..self.buf.text.len();
        let at = t.len().saturating_sub(from_end);
        self.buf.apply(&[dactile::tex::Edit::new(all, t)], at);
        self.edited();
        self.refresh_known();
    }

    pub(super) fn member_form(&self, g: usize) -> Form {
        let mut f = self.character_form("", None);
        f.kind = FormKind::NewMember(g);
        let name = &self.doc.cast.groups[g].desc;
        f.title = lang::f("yeni üye — {}", "new member — {}", &[name]);
        f.fields.retain(|x| {
            !x.label.starts_with("Kişiler")
                && !x.label.starts_with("Gizli")
                && x.label != "Açıklama"
        });
        f
    }

    pub(super) fn group_form(&self, g: Option<usize>) -> Form {
        let now = g.map_or(String::new(), |g| {
            dactile::tex::unescape_text(&self.doc.cast.groups[g].desc)
        });
        let mut fields = vec![Field::text("Grup adı", &now, Vec::new())];
        if g.is_none() {
            fields.push(Field::text("Üyeler (kısaltmalar)", "", Vec::new()));
        }
        Form {
            kind: FormKind::GroupName(g),
            title: if g.is_some() {
                lang::l("grubun adı", "the group's name").into()
            } else {
                lang::l(
                    "yeni grup (üyeler: an, mi gibi; boş da olur)",
                    "a new group (members: an, mi; or none)",
                )
                .into()
            },
            fields,
            focus: 0,
            at: 0..0,
            error: None,
            confirmed: false,
        }
    }

    pub(super) fn move_form(&self, alias: &str) -> Form {
        let c = self.doc.cast.get(alias);
        let now = c
            .and_then(|c| c.group)
            .map_or(String::new(), |g| self.doc.cast.groups[g].desc.clone());
        let mut options: Vec<String> = self
            .doc
            .cast
            .groups
            .iter()
            .map(|g| g.desc.clone())
            .collect();
        options.push(lang::l("(grup yok)", "(no group)").into());
        Form {
            kind: FormKind::MoveTo(alias.to_string()),
            title: lang::f(
                "\\{} hangi gruba? (Tab gruplar arasında gezer; yeni bir ad yeni grup açar)",
                "\\{} into which group? (Tab goes through them; a new name makes a new one)",
                &[&alias],
            ),
            fields: vec![Field::text("Grup", &now, options)],
            focus: 0,
            at: 0..0,
            error: None,
            confirmed: false,
        }
    }

    /// Where a character is in the cast panel.
    pub(super) fn cast_index(&self, alias: &str) -> usize {
        self.cast_entries(false)
            .iter()
            .position(
                |e| matches!(e, Entry::Character(i) if self.doc.cast.characters[*i].alias == alias),
            )
            .unwrap_or(0)
    }
}
