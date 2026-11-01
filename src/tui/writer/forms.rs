//! The popups: their keys, the menus and the forms they open.

use super::*;

impl Writer {
    /// Keys a popup takes; true when it took this one.
    pub(super) fn popup_key(&mut self, k: KeyEvent) -> bool {
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
            Popup::Roles => {
                self.roles_key(k);
                true
            }
            Popup::Memorize(_) => {
                self.memo_key(k);
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
                        KeyCode::Esc | KeyCode::Tab | KeyCode::Char('c' | 'ç') => p.trash = false,
                        KeyCode::Char('g') if ctrl => self.popup = Popup::None,
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
                self.board_key(k, units);
                true
            }
            Popup::Palette(q, sel) => {
                let n = palette_items(q).len();
                match k.code {
                    KeyCode::Esc => self.popup = Popup::None,
                    KeyCode::Char('p') if ctrl => self.popup = Popup::None,
                    KeyCode::Up => *sel = sel.saturating_sub(1),
                    KeyCode::Down => *sel = (*sel + 1).min(n.saturating_sub(1)),
                    KeyCode::Backspace => {
                        q.pop();
                        *sel = 0;
                    }
                    KeyCode::Char(c) if !ctrl => {
                        q.push(c);
                        *sel = 0;
                    }
                    KeyCode::Enter => {
                        let chosen = palette_items(q).get(*sel).map(|i| i.act);
                        self.popup = Popup::None;
                        if let Some(act) = chosen {
                            self.run(act);
                        }
                    }
                    _ => {}
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
    pub(super) fn follow_popups(&mut self) {
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
    pub(super) fn choose(&mut self, action: Action, range: std::ops::Range<usize>) {
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

    pub(super) fn scene_form(&self, at: usize) -> Form {
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

    pub(super) fn act_form(&self, at: usize) -> Form {
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
    pub(super) fn character_form(&self, alias: &str, insert_at: Option<usize>) -> Form {
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

    pub(super) fn form_key(&mut self, k: KeyEvent) {
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
                self.popup = match form.kind {
                    FormKind::Actor(_) | FormKind::CastingName(_) | FormKind::Wpm => Popup::Roles,
                    FormKind::NewMember(_) | FormKind::GroupName(_) | FormKind::MoveTo(_) => {
                        Popup::Cast(CastPanel {
                            sel: 0,
                            by_lines: false,
                        })
                    }
                    _ => Popup::None,
                };
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
        if matches!(
            form.kind,
            FormKind::Character | FormKind::Split(_) | FormKind::NewMember(_)
        ) {
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
}
