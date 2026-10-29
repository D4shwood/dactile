//! The home screen: the plays, a new play, opening a file, the settings
//! (which are also the first-run setup).

use super::popup::{self, Field, Form, FormKind, Outcome};
use super::theme::{self, BEIGE, CLARET, FAINT, FG, MUTED, WARN};
use chrono::{DateTime, Local};
use dactile::config::{self, Config};
use dactile::lang::{self, Lang};
use dactile::library::{self, Play};
use dactile::{spell, timing};
use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use std::path::{Path, PathBuf};
use unicode_width::UnicodeWidthStr;

/// How long a play's page takes to unroll.
const PAGE_OPENS: std::time::Duration = std::time::Duration::from_millis(220);

/// How long the home screen types itself out.
const TYPE_FOR: std::time::Duration = std::time::Duration::from_millis(2200);

pub enum Action {
    Stay,
    Open(PathBuf),
    Quit,
}

pub struct Home {
    /// In the order of their tree, each with whether it and the plays above
    /// it are the last of their siblings (see `library::tree`).
    plays: Vec<Play>,
    lasts: Vec<Vec<bool>>,
    sel: usize,
    form: Option<Form>,
    message: Option<(String, bool)>,
    /// The first run: the settings form can't be skipped.
    first_run: bool,
    /// A play asked for on the command line, opened once setup is done.
    pub pending: Option<PathBuf>,
    /// When the screen opened: the cursor after the name blinks from it,
    /// the lines under it type themselves out.
    born: std::time::Instant,
    /// When the form on screen opened.
    form_born: std::time::Instant,
    form_kind: Option<std::mem::Discriminant<FormKind>>,
    /// Del pressed once on this play: a second one throws it away.
    arming: Option<PathBuf>,
    /// What went to the trash last, for `u`.
    thrown: Option<(String, Vec<dactile::trash::Moved>)>,
    /// The chosen play's title being changed in its row (F2).
    inline: Option<Inline>,
    /// The chosen play's first page as the writer lays it out, kept until
    /// the play or the file changes.
    page: Option<Page>,
    /// When the page of another play came: it unrolls.
    page_born: std::time::Instant,
    page_for: Option<PathBuf>,
    /// `?`: every key in a window.
    keys_open: bool,
}

struct Page {
    path: PathBuf,
    modified: Option<std::time::SystemTime>,
    width: u16,
    rows: Vec<dactile::render::Row>,
}

/// A title typed over in its row, as a file manager renames.
struct Inline {
    value: String,
    /// The title before, to tell whether anything changed.
    was: String,
    /// The cursor, in characters.
    at: usize,
    /// All of it chosen: the first key types over it.
    all: bool,
}

impl Home {
    pub fn new(cfg: &Config, first_run: bool) -> Home {
        lang::set(lang::resolve(&cfg.lang, None));
        let mut h = Home {
            plays: Vec::new(),
            lasts: Vec::new(),
            sel: 0,
            form: None,
            message: None,
            first_run,
            pending: None,
            born: std::time::Instant::now(),
            form_born: std::time::Instant::now(),
            form_kind: None,
            arming: None,
            thrown: None,
            inline: None,
            page: None,
            page_born: std::time::Instant::now(),
            page_for: None,
            keys_open: false,
        };
        h.refresh(cfg);
        // On the play worked on last.
        if let Some(i) = (0..h.plays.len()).max_by_key(|&i| h.plays[i].modified) {
            h.sel = i;
        }
        if first_run {
            h.form = Some(settings_form(cfg, true));
        }
        h
    }

    pub fn refresh(&mut self, cfg: &Config) {
        lang::set(lang::resolve(&cfg.lang, None));
        (self.plays, self.lasts) = library::tree(library::list(cfg)).into_iter().unzip();
        self.sel = self.sel.min(self.plays.len().saturating_sub(1));
    }

    pub fn key(&mut self, k: KeyEvent, cfg: &mut Config) -> Action {
        self.message = None;
        let armed = self.arming.take();
        if self.inline.is_some() {
            self.inline_key(k, cfg);
            return Action::Stay;
        }
        if let Some(form) = &mut self.form {
            match popup::edit(form, k) {
                Outcome::Stay => {
                    // The file name follows the title (or the draft number)
                    // until edited by hand.
                    let name = match &form.kind {
                        FormKind::NewPlay | FormKind::Copy(_) => {
                            let title = form.get("Başlık").to_string();
                            (!title.is_empty()).then(|| library::file_name(&title))
                        }
                        FormKind::Fork(src) => Some(fork_name(src, form.get("Taslak no"))),
                        _ => None,
                    };
                    let follows = matches!(
                        form.kind,
                        FormKind::NewPlay | FormKind::Copy(_) | FormKind::Fork(_)
                    );
                    if follows
                        && let Some(f) = form.field_mut("Dosya")
                        && !f.touched
                    {
                        f.value = name.unwrap_or_default();
                    }
                }
                Outcome::Cancel if self.first_run => {}
                Outcome::Cancel => self.form = None,
                Outcome::Submit => {
                    let form = self.form.take().unwrap();
                    return self.submit(form, cfg);
                }
            }
            return Action::Stay;
        }
        if std::mem::take(&mut self.keys_open) {
            // Any key closes the keys; Esc and ? do nothing else.
            if matches!(k.code, KeyCode::Esc | KeyCode::Char('?')) {
                return Action::Stay;
            }
        }
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        match k.code {
            KeyCode::Char('?') => self.keys_open = true,
            KeyCode::Char('q') | KeyCode::Esc => return Action::Quit,
            KeyCode::Char('c') if ctrl => return Action::Quit,
            KeyCode::Up => self.sel = self.sel.saturating_sub(1),
            KeyCode::Down => self.sel = (self.sel + 1).min(self.plays.len().saturating_sub(1)),
            KeyCode::Enter => {
                if let Some(p) = self.plays.get(self.sel) {
                    return Action::Open(p.path.clone());
                }
            }
            KeyCode::Char('n') => self.form = Some(new_play_form()),
            KeyCode::Char('p') => {
                if let Some(p) = self.plays.get(self.sel)
                    && let Err(e) = dactile::build::open(&dactile::build::pdf_path(&p.path))
                {
                    self.message = Some((format!("{e:#}"), true));
                }
            }
            KeyCode::Char('o') => {
                let dir = format!("{}/", config::tilde(&cfg.plays_dir()));
                self.form = Some(Form {
                    kind: FormKind::OpenPath,
                    title: lang::l("dosya aç", "open a file").into(),
                    fields: vec![Field::text("Dosya", &dir, Vec::new())],
                    focus: 0,
                    at: 0..0,
                    error: None,
                    confirmed: false,
                });
            }
            KeyCode::Char('s') => self.form = Some(settings_form(cfg, false)),
            KeyCode::F(2) if !k.modifiers.contains(KeyModifiers::SHIFT) => {
                if let Some(p) = self.plays.get(self.sel) {
                    let raw = dactile::info::get(
                        &std::fs::read_to_string(&p.path).unwrap_or_default(),
                        "title",
                    )
                    .map(|t| dactile::tex::unescape_text(&t))
                    .unwrap_or_else(|| p.title.clone());
                    self.inline = Some(Inline {
                        at: raw.chars().count(),
                        value: raw.clone(),
                        was: raw,
                        all: true,
                    });
                }
            }
            KeyCode::F(2) => {
                if let Some(p) = self.plays.get(self.sel) {
                    let raw = dactile::info::get(
                        &std::fs::read_to_string(&p.path).unwrap_or_default(),
                        "title",
                    )
                    .unwrap_or_default();
                    let file = p
                        .path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();
                    self.form = Some(Form {
                        kind: FormKind::Rename(p.path.clone()),
                        title: lang::l("adını değiştir", "rename").into(),
                        fields: vec![
                            Field::text("Başlık", &dactile::tex::unescape_text(&raw), Vec::new()),
                            Field::text("Dosya", &file, Vec::new()),
                        ],
                        focus: 0,
                        at: 0..0,
                        error: None,
                        confirmed: false,
                    });
                }
            }
            KeyCode::Delete => {
                if let Some(p) = self.plays.get(self.sel) {
                    let path = p.path.clone();
                    if armed.as_ref() == Some(&path) {
                        self.throw_away(cfg);
                    } else if let Some(who) = dactile::file::holder(&path) {
                        self.message = Some((
                            lang::f(
                                "Başka bir Dactile'da açık ({}).",
                                "Open in another Dactile ({}).",
                                &[&who],
                            ),
                            true,
                        ));
                    } else {
                        let drafts = self.drafts_of(&path).len();
                        let then = if drafts > 0 {
                            lang::f(
                                " Taslakları ({}) yerinde kalır.",
                                " Its drafts ({}) stay.",
                                &[&drafts],
                            )
                        } else {
                            String::new()
                        };
                        self.message = Some((
                            lang::f(
                                "«{}» çöp kutusuna gitsin mi? Del: evet, başka tuş: hayır.{}",
                                "Send «{}» to the trash? Del: yes, any other key: no.{}",
                                &[&p.title, &then],
                            ),
                            true,
                        ));
                        self.arming = Some(path);
                    }
                }
            }
            KeyCode::Char('u') => {
                if let Some((title, moved)) = self.thrown.take() {
                    match dactile::trash::restore(&moved) {
                        Ok(()) => {
                            self.refresh(cfg);
                            if !self.plays.iter().any(|p| p.path == moved[0].from) {
                                // From outside the plays folder: back in the recent list.
                                cfg.opened(&moved[0].from);
                                let _ = cfg.save();
                                self.refresh(cfg);
                            }
                            if let Some(i) = self.plays.iter().position(|p| p.path == moved[0].from)
                            {
                                self.sel = i;
                            }
                            self.message = Some((
                                lang::f("«{}» geri geldi.", "«{}» is back.", &[&title]),
                                false,
                            ));
                        }
                        Err(e) => self.message = Some((format!("{e:#}"), true)),
                    }
                }
            }
            KeyCode::Char('d') => {
                if let Some(p) = self.plays.get(self.sel) {
                    let title = lang::f("{} (kopya)", "{} (copy)", &[&p.title]);
                    self.form = Some(Form {
                        kind: FormKind::Copy(p.path.clone()),
                        title: lang::l("kopyasını çıkar", "make a copy").into(),
                        fields: vec![
                            Field::text("Başlık", &title, Vec::new()),
                            Field::text("Dosya", &library::file_name(&title), Vec::new()),
                        ],
                        focus: 0,
                        at: 0..0,
                        error: None,
                        confirmed: false,
                    });
                }
            }
            KeyCode::Char('f') => {
                if let Some(p) = self.plays.get(self.sel) {
                    let today = chrono::Local::now().format("%d.%m.%Y").to_string();
                    let draft = library::next_draft(p.draft.as_deref().unwrap_or(""), &today);
                    self.form = Some(Form {
                        kind: FormKind::Fork(p.path.clone()),
                        title: lang::f("{}: yeni taslak", "{}: a new draft", &[&p.title]),
                        fields: vec![
                            Field::text("Taslak no", &draft, Vec::new()),
                            Field::text("Dosya", &fork_name(&p.path, &draft), Vec::new()),
                        ],
                        focus: 0,
                        at: 0..0,
                        error: None,
                        confirmed: false,
                    });
                }
            }
            _ => {}
        }
        Action::Stay
    }

    /// Whether the screen is still typing itself out (the name's cursor
    /// blinks at the slow pace).
    pub fn animating(&self, cfg: &Config) -> bool {
        cfg.animations
            && (self.born.elapsed() < TYPE_FOR
                || self.form_born.elapsed() < std::time::Duration::from_millis(160)
                || self.page_born.elapsed() < PAGE_OPENS)
    }

    /// How much of `s` has been typed so far, `delay` after the screen came.
    fn typed(&self, s: &str, cfg: &Config, delay: u128) -> String {
        if !cfg.animations {
            return s.to_string();
        }
        let ms = self.born.elapsed().as_millis().saturating_sub(delay);
        s.chars().take((ms / 28) as usize).collect()
    }

    fn submit(&mut self, mut form: Form, cfg: &mut Config) -> Action {
        let fail = |this: &mut Home, mut form: Form, e: String| {
            form.error = Some(e);
            this.form = Some(form);
            Action::Stay
        };
        match form.kind {
            FormKind::Settings => {
                let lang_value = match form.get("Dil") {
                    v if v.starts_with("tr") || v.starts_with("Türk") => "tr",
                    v if v.starts_with("en") || v.starts_with("Eng") => "en",
                    _ => "auto",
                };
                cfg.lang = lang_value.into();
                cfg.plays_dir = form.get("Oyunlar klasörü").to_string();
                cfg.author = form.get("Adın").to_string();
                cfg.menu = if form.get("Öneri menüsü").contains("^K") {
                    "manual"
                } else {
                    "auto"
                }
                .into();
                cfg.animations = form
                    .fields
                    .iter()
                    .find(|f| f.label == "Animasyonlar")
                    .is_some_and(Field::on);
                cfg.keyboard = match form.get("Klavye") {
                    k if k.ends_with('F') || k.ends_with('f') => "f",
                    k if k.eq_ignore_ascii_case("us") => "us",
                    _ => "q",
                }
                .into();
                match form.get("Dakikada kelime").parse::<u32>() {
                    Ok(n) if (30..=400).contains(&n) => cfg.wpm = n,
                    _ => {
                        return fail(
                            self,
                            form,
                            lang::l(
                                "Dakikada kelime 30 ile 400 arasında olmalı.",
                                "Words a minute: between 30 and 400.",
                            )
                            .into(),
                        );
                    }
                }
                if cfg.plays_dir.trim().is_empty() {
                    return fail(
                        self,
                        form,
                        lang::l(
                            "Oyunlar klasörü boş olamaz.",
                            "The plays folder can't be empty.",
                        )
                        .into(),
                    );
                }
                if let Err(e) = std::fs::create_dir_all(cfg.plays_dir())
                    .map_err(anyhow::Error::from)
                    .and_then(|_| cfg.save())
                {
                    return fail(self, form, format!("{e:#}"));
                }
                self.first_run = false;
                self.refresh(cfg);
                if let Some(p) = self.pending.take() {
                    return Action::Open(p);
                }
                self.message = Some((
                    lang::l("Ayarlar kaydedildi.", "Settings saved.").into(),
                    false,
                ));
                Action::Stay
            }
            FormKind::NewPlay => {
                let title = form.get("Başlık").to_string();
                if title.is_empty() {
                    return fail(
                        self,
                        form,
                        lang::l("Oyunun bir adı olmalı.", "The play needs a title.").into(),
                    );
                }
                let name = form.get("Dosya").to_string();
                let path = resolve_path(&name, cfg);
                if path.exists() {
                    return fail(
                        self,
                        form,
                        lang::f(
                            "{} zaten var.",
                            "{} already exists.",
                            &[&config::tilde(&path)],
                        ),
                    );
                }
                let lang = match cfg.lang.as_str() {
                    "en" => Lang::En,
                    "tr" => Lang::Tr,
                    _ => lang::system(),
                };
                let text = library::template(&title, &cfg.author, lang);
                let made = path
                    .parent()
                    .map_or(Ok(()), std::fs::create_dir_all)
                    .map_err(anyhow::Error::from)
                    .and_then(|_| dactile::file::write_atomic(&path, &text));
                match made {
                    Ok(()) => Action::Open(path),
                    Err(e) => {
                        form.error = Some(format!("{e:#}"));
                        self.form = Some(form);
                        Action::Stay
                    }
                }
            }
            FormKind::Copy(ref src) | FormKind::Fork(ref src) => {
                let src = src.clone();
                let fork = matches!(form.kind, FormKind::Fork(_));
                let name = form.get("Dosya").trim().to_string();
                let (title, draft) = if fork {
                    (None, Some(form.get("Taslak no").trim().to_string()))
                } else {
                    (Some(form.get("Başlık").trim().to_string()), None)
                };
                if title.as_deref() == Some("") || name.is_empty() {
                    return fail(
                        self,
                        form,
                        lang::l(
                            "Başlık ve dosya adı boş olamaz.",
                            "Title and file name can't be empty.",
                        )
                        .into(),
                    );
                }
                // Next to the play it comes from.
                let dir = src.parent().map(Path::to_path_buf).unwrap_or_default();
                let name = if name.ends_with(".tex") {
                    name
                } else {
                    format!("{name}.tex")
                };
                let path = dir.join(&name);
                if path.exists() {
                    return fail(
                        self,
                        form,
                        lang::f(
                            "{} zaten var.",
                            "{} already exists.",
                            &[&config::tilde(&path)],
                        ),
                    );
                }
                let made = dactile::file::read(&src).and_then(|text| {
                    let text = library::derive(&text, title.as_deref(), draft.as_deref());
                    dactile::file::write_atomic(&path, &text)
                });
                if let Err(e) = made {
                    return fail(self, form, format!("{e:#}"));
                }
                // The character cards and the play's own words go along; a
                // draft remembers what it is a draft of, a copy does not.
                let mut side = dactile::sidecar::Sidecar::load(&src).unwrap_or_default();
                side.parent = fork
                    .then(|| src.file_name().map(|n| n.to_string_lossy().to_string()))
                    .flatten();
                let _ = side.save(&path);
                self.refresh(cfg);
                if let Some(i) = self.plays.iter().position(|p| p.path == path) {
                    self.sel = i;
                } else {
                    // Outside the plays folder: keep it in the recent list.
                    cfg.opened(&path);
                    let _ = cfg.save();
                    self.refresh(cfg);
                    self.sel = self.plays.iter().position(|p| p.path == path).unwrap_or(0);
                }
                self.message = Some((
                    lang::f("{} oluşturuldu.", "{} made.", &[&config::tilde(&path)]),
                    false,
                ));
                Action::Stay
            }
            FormKind::Rename(ref from) => {
                let from = from.clone();
                let title = form.get("Başlık").trim().to_string();
                let name = form.get("Dosya").trim().to_string();
                match self.rename_play(&from, &title, &name, cfg) {
                    Ok(()) => Action::Stay,
                    Err(e) => fail(self, form, e),
                }
            }
            FormKind::OpenPath => {
                let path = resolve_path(form.get("Dosya"), cfg);
                if !path.is_file() {
                    return fail(
                        self,
                        form,
                        lang::f("{} bulunamadı.", "{} not found.", &[&config::tilde(&path)]),
                    );
                }
                Action::Open(path)
            }
            _ => Action::Stay,
        }
    }

    /// A play's new title and file name, from the form or the row; on the
    /// screen after it, chosen.
    fn rename_play(
        &mut self,
        from: &Path,
        title: &str,
        name: &str,
        cfg: &mut Config,
    ) -> Result<(), String> {
        if title.is_empty() || name.is_empty() {
            return Err(lang::l(
                "Başlık ve dosya adı boş olamaz.",
                "Title and file name can't be empty.",
            )
            .into());
        }
        if name.contains('/') {
            return Err(lang::l(
                "Dosya adında / olamaz: oyun yerinde kalır.",
                "No / in the file name: the play stays where it is.",
            )
            .into());
        }
        let name = if name.ends_with(".tex") {
            name.to_string()
        } else {
            format!("{name}.tex")
        };
        if let Some(who) = dactile::file::holder(from) {
            return Err(lang::f(
                "Başka bir Dactile'da açık ({}).",
                "Open in another Dactile ({}).",
                &[&who],
            ));
        }
        let to = from.with_file_name(&name);
        if to != from {
            if to.exists() {
                return Err(lang::f(
                    "{} zaten var.",
                    "{} already exists.",
                    &[&config::tilde(&to)],
                ));
            }
            let drafts = self.drafts_of(from);
            library::rename(from, &to, &drafts).map_err(|e| format!("{e:#}"))?;
            let (old, new) = (from.to_string_lossy(), to.to_string_lossy());
            for r in cfg.recent.iter_mut().filter(|r| **r == old) {
                *r = new.to_string();
            }
            let _ = cfg.save();
        }
        let text = dactile::file::read(&to).unwrap_or_default();
        let old_title = dactile::info::get(&text, "title").unwrap_or_default();
        if dactile::tex::unescape_text(&old_title) != title {
            let new_text = library::derive(&text, Some(title), None);
            dactile::file::write_atomic(&to, &new_text).map_err(|e| format!("{e:#}"))?;
        }
        self.refresh(cfg);
        if let Some(i) = self.plays.iter().position(|p| p.path == to) {
            self.sel = i;
        }
        self.message = Some((
            lang::f(
                "Artık «{}»: {}.",
                "Now «{}»: {}.",
                &[&title, &config::tilde(&to)],
            ),
            false,
        ));
        Ok(())
    }

    /// A key while the chosen play's title is being changed in its row.
    fn inline_key(&mut self, k: KeyEvent, cfg: &mut Config) {
        let Some(ed) = self.inline.as_mut() else {
            return;
        };
        let len = ed.value.chars().count();
        let byte = |v: &str, at: usize| v.char_indices().nth(at).map_or(v.len(), |(b, _)| b);
        match k.code {
            KeyCode::Esc => self.inline = None,
            KeyCode::Enter => {
                let Some(p) = self.plays.get(self.sel) else {
                    self.inline = None;
                    return;
                };
                let title = ed.value.trim().to_string();
                let from = p.path.clone();
                if title.is_empty() || title == ed.was {
                    self.inline = None;
                    return;
                }
                let name = inline_name(&from, &title, p.draft.as_deref());
                match self.rename_play(&from, &title, &name, cfg) {
                    Ok(()) => self.inline = None,
                    Err(e) => self.message = Some((e, true)),
                }
            }
            KeyCode::Char(c) if !k.modifiers.contains(KeyModifiers::CONTROL) => {
                if ed.all {
                    ed.value.clear();
                    ed.at = 0;
                    ed.all = false;
                }
                let b = byte(&ed.value, ed.at);
                ed.value.insert(b, c);
                ed.at += 1;
            }
            KeyCode::Backspace | KeyCode::Delete if ed.all => {
                ed.value.clear();
                ed.at = 0;
                ed.all = false;
            }
            KeyCode::Backspace if ed.at > 0 => {
                let b = byte(&ed.value, ed.at - 1);
                ed.value.remove(b);
                ed.at -= 1;
            }
            KeyCode::Delete if ed.at < len => {
                let b = byte(&ed.value, ed.at);
                ed.value.remove(b);
            }
            KeyCode::Left => {
                ed.at = if ed.all { 0 } else { ed.at.saturating_sub(1) };
                ed.all = false;
            }
            KeyCode::Right => {
                ed.at = if ed.all { len } else { (ed.at + 1).min(len) };
                ed.all = false;
            }
            KeyCode::Home => (ed.at, ed.all) = (0, false),
            KeyCode::End => (ed.at, ed.all) = (len, false),
            _ => {}
        }
    }

    /// The plays whose parent is `path`.
    fn drafts_of(&self, path: &Path) -> Vec<PathBuf> {
        self.plays
            .iter()
            .filter(|p| p.parent.as_deref() == Some(path))
            .map(|p| p.path.clone())
            .collect()
    }

    /// The chosen play, and what goes with it, to the desktop's trash.
    fn throw_away(&mut self, cfg: &mut Config) {
        let Some(p) = self.plays.get(self.sel).cloned() else {
            return;
        };
        match library::throw_away(&p.path, &dactile::trash::bin()) {
            Ok(moved) => {
                let gone = p.path.to_string_lossy();
                cfg.recent.retain(|r| *r != gone);
                let _ = cfg.save();
                self.refresh(cfg);
                self.message = Some((
                    lang::f(
                        "«{}» çöp kutusunda. u: geri al.",
                        "«{}» is in the trash. u: undo.",
                        &[&p.title],
                    ),
                    false,
                ));
                self.thrown = Some((p.title, moved));
            }
            Err(e) => self.message = Some((format!("{e:#}"), true)),
        }
    }

    pub fn draw(&mut self, f: &mut Frame, cfg: &Config) {
        let area = f.area();
        let kind = self.form.as_ref().map(|x| std::mem::discriminant(&x.kind));
        if kind != self.form_kind {
            self.form_kind = kind;
            self.form_born = std::time::Instant::now();
        }
        f.render_widget(
            ratatui::widgets::Block::default().style(theme::base()),
            area,
        );
        let side = 3u16.min(area.width / 10);
        let inner_w = area.width.saturating_sub(side * 2);
        // The masthead: the name spaced out, what it is, and a rule.
        let spaced: String = "DACTILE".chars().map(|c| format!("{c} ")).collect();
        let folder = config::tilde(&cfg.plays_dir());
        // A typewriter's cursor after the name, blinking.
        let on = (self.born.elapsed().as_millis() / 600).is_multiple_of(2);
        let left = Line::from(vec![
            Span::styled(
                spaced,
                Style::default().fg(BEIGE).add_modifier(Modifier::BOLD),
            ),
            Span::styled(if on { "█" } else { " " }, Style::default().fg(CLARET)),
        ]);
        f.render_widget(
            Paragraph::new(left).style(theme::base()),
            Rect {
                x: side,
                y: area.y + 1,
                width: inner_w,
                height: 1,
            },
        );
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(
                    self.typed("Dynamic Stagewright", cfg, 150),
                    Style::default().fg(MUTED),
                ),
                Span::styled(
                    self.typed(
                        &format!(
                            "   {}",
                            lang::l("tiyatro oyunu yazmak için", "for writing stage plays")
                        ),
                        cfg,
                        150 + 19 * 28,
                    ),
                    Style::default().fg(FAINT),
                ),
            ]))
            .style(theme::base()),
            Rect {
                x: side,
                y: area.y + 2,
                width: inner_w,
                height: 1,
            },
        );
        // Only as wide as itself, so it leaves the line's left part alone.
        let label = lang::l("oyunlar  ", "plays  ");
        let fw = (label.width() + folder.width()) as u16;
        if fw + 40 < inner_w {
            f.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled(label, Style::default().fg(FAINT)),
                    Span::styled(folder, Style::default().fg(MUTED)),
                ]))
                .style(theme::base()),
                Rect {
                    x: side + inner_w - fw,
                    y: area.y + 2,
                    width: fw,
                    height: 1,
                },
            );
        }
        f.render_widget(
            Paragraph::new("─".repeat(inner_w as usize)).style(theme::base().fg(CLARET)),
            Rect {
                x: side,
                y: area.y + 3,
                width: inner_w,
                height: 1,
            },
        );
        let body = Rect {
            x: side,
            y: area.y + 5,
            width: inner_w,
            height: area.height.saturating_sub(9),
        };
        if self.first_run {
            let hello = lang::l(
                "Hoş geldin. Önce birkaç şey: dil, oyunların duracağı klasör ve adın.",
                "Welcome. A few things first: language, where plays live, your name.",
            );
            f.render_widget(
                Paragraph::new(hello).style(theme::base().fg(MUTED)),
                Rect { height: 1, ..body },
            );
        } else if self.plays.is_empty() {
            let lines = vec![
                Line::from(Span::styled(
                    lang::l("Henüz oyun yok.", "No plays yet."),
                    Style::default().fg(FG).add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(vec![
                    Span::styled("n", Style::default().fg(BEIGE)),
                    Span::styled(
                        lang::l("  yeni bir oyun başlat", "  start a new play"),
                        Style::default().fg(MUTED),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("o", Style::default().fg(BEIGE)),
                    Span::styled(
                        lang::l(
                            "  başka bir yerdeki bir .tex dosyasını aç",
                            "  open a .tex file from somewhere else",
                        ),
                        Style::default().fg(MUTED),
                    ),
                ]),
            ];
            f.render_widget(Paragraph::new(lines).style(theme::base()), body);
        } else {
            // The list, and the chosen play's first page when there is room,
            // as tall as the screen.
            let page_w = if body.width >= 90 {
                (body.width * 42 / 100).clamp(44, 64)
            } else {
                0
            };
            let mut list = Rect {
                width: body
                    .width
                    .saturating_sub(page_w + if page_w > 0 { 5 } else { 0 }),
                ..body
            };
            // A line of the whole shelf at the list's foot, while the list
            // leaves room for it.
            if self.plays.len() * 3 + 2 <= list.height as usize {
                self.draw_footing(
                    f,
                    Rect {
                        y: list.y + list.height - 1,
                        height: 1,
                        ..list
                    },
                );
                list.height -= 2;
            }
            self.draw_list(f, list);
            if page_w > 0 {
                let page = Rect {
                    x: body.x + body.width - page_w,
                    width: page_w,
                    y: area.y + 5,
                    // Its shadow ends above the message line.
                    height: area.height.saturating_sub(8),
                };
                let path = self.plays.get(self.sel).map(|p| p.path.clone());
                if path != self.page_for {
                    self.page_for = path;
                    self.page_born = std::time::Instant::now();
                }
                let before = (cfg.animations && self.page_born.elapsed() < PAGE_OPENS)
                    .then(|| f.buffer_mut().clone());
                self.draw_page(f, page, cfg.theme());
                if let Some(before) = before {
                    super::anim::unroll(f.buffer_mut(), &before, self.page_born, PAGE_OPENS);
                }
            }
        }
        // A missing dictionary is noticed here.
        let dict = match spell::find() {
            Some(_) => Span::raw(""),
            None => Span::styled(
                lang::l(
                    "Türkçe sözlük yok, yazım denetimi kapalı: paru -S hunspell-tr",
                    "No Turkish dictionary, spell check off: install hunspell-tr",
                ),
                Style::default().fg(WARN),
            ),
        };
        let y = area.y + area.height.saturating_sub(3);
        let status = match &self.message {
            Some((msg, warn)) => Span::styled(
                msg.clone(),
                Style::default().fg(if *warn { WARN } else { MUTED }),
            ),
            None => dict,
        };
        f.render_widget(
            Paragraph::new(Line::from(status)).style(theme::base()),
            Rect {
                x: side,
                y,
                width: inner_w,
                height: 1,
            },
        );
        let keys: &[(&str, &str)] = if self.inline.is_some() {
            &[
                ("⏎", lang::l("tamam", "done")),
                ("Esc", lang::l("vazgeç", "cancel")),
            ]
        } else {
            &[
                ("⏎", lang::l("aç", "open")),
                ("n", lang::l("yeni oyun", "new play")),
                ("f", lang::l("yeni taslak", "new draft")),
                ("p", "PDF"),
                ("?", lang::l("bütün tuşlar", "all keys")),
            ]
        };
        let mut spans = Vec::new();
        for &(k, what) in keys {
            spans.push(Span::styled(k, Style::default().fg(MUTED)));
            spans.push(Span::styled(
                format!(" {what}   "),
                Style::default().fg(FAINT),
            ));
        }
        f.render_widget(
            Paragraph::new(Line::from(spans)).style(theme::base()),
            Rect {
                x: side,
                y: area.y + area.height.saturating_sub(1),
                width: inner_w,
                height: 1,
            },
        );
        if self.keys_open {
            draw_keys(f, area);
        }
        if let Some(form) = &self.form {
            // The form unrolls over what was there.
            let opens = std::time::Duration::from_millis(160);
            let before = (cfg.animations && self.form_born.elapsed() < opens)
                .then(|| f.buffer_mut().clone());
            popup::draw_form(f, area, form);
            if let Some(before) = before {
                super::anim::unroll(f.buffer_mut(), &before, self.form_born, opens);
            }
        }
    }

    /// "3 oyun · 4 taslak · ≈ 1 sa 52 dk": the shelf in one faint line.
    fn draw_footing(&self, f: &mut Frame, area: Rect) {
        let drafts = self.plays.iter().filter(|p| p.parent.is_some()).count();
        let roots = self.plays.len() - drafts;
        // The plays' own running times, not their drafts' again.
        let secs: u32 = self
            .plays
            .iter()
            .filter(|p| p.parent.is_none())
            .map(|p| p.secs)
            .sum();
        let mut s = lang::f("{} oyun", "{} plays", &[&roots]);
        if drafts > 0 {
            s.push_str(&lang::f(" · {} taslak", " · {} drafts", &[&drafts]));
        }
        s.push_str(&format!(" · ≈ {}", timing::format(secs)));
        f.render_widget(
            Paragraph::new(s).style(theme::base().fg(FAINT)),
            Rect {
                x: area.x + 2,
                width: area.width.saturating_sub(2),
                ..area
            },
        );
    }

    /// The chosen play's first page, laid out as the writer lays it out, on
    /// paper as tall as the screen, its foot fading as if it went on.
    fn draw_page(&mut self, f: &mut Frame, area: Rect, theme_name: &str) {
        let Some(p) = self.plays.get(self.sel) else {
            return;
        };
        let pal = theme::palette(theme_name);
        // The paper, and its shadow on the right and below.
        let paper = Rect {
            width: area.width.saturating_sub(1),
            height: area.height.saturating_sub(1),
            ..area
        };
        let text_w = paper.width.saturating_sub(6);
        let modified = dactile::file::modified(&p.path);
        let fresh = self
            .page
            .as_ref()
            .is_some_and(|c| c.path == p.path && c.modified == modified && c.width == text_w);
        if !fresh {
            self.page = std::fs::read_to_string(&p.path).ok().map(|text| {
                let doc = dactile::render::Doc::new(&text);
                let lay = dactile::render::layout(
                    &text,
                    &doc,
                    dactile::render::Options {
                        width: text_w,
                        raw_block: None,
                        raw_all: false,
                        preamble_open: false,
                        focus_scene: None,
                        page_rows: None,
                    },
                );
                Page {
                    path: p.path.clone(),
                    modified,
                    width: text_w,
                    // Without the folded preamble's line.
                    rows: lay.rows.into_iter().filter(|r| r.block.is_some()).collect(),
                }
            });
        }
        let Some(page) = &self.page else {
            draw_card(
                f,
                Rect {
                    height: area.height.min(16),
                    ..area
                },
                p,
                theme_name,
            );
            return;
        };
        let buf = f.buffer_mut();
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                let on_paper = x < paper.x + paper.width && y < paper.y + paper.height;
                let shadow = !on_paper && x > area.x && y > area.y;
                let bg = if on_paper {
                    pal.bg
                } else if shadow {
                    theme::SHADOW
                } else {
                    theme::BG
                };
                buf[(x, y)].set_char(' ').set_style(Style::default().bg(bg));
            }
        }
        // A row of paper above the text, one below.
        let room = paper.height.saturating_sub(2) as usize;
        let fade = 4.min(room);
        for (i, row) in page.rows.iter().take(room).enumerate() {
            let y = paper.y + 1 + i as u16;
            // The last rows fade into the paper.
            let t = (i + fade + 1).saturating_sub(room) as f32 / (fade + 1) as f32;
            for c in &row.cells {
                let x = paper.x + 3 + c.col;
                if x >= paper.x + paper.width - 3 {
                    continue;
                }
                let mut st = theme::on(c.style, pal);
                if t > 0.0
                    && let Some(fg) = st.fg
                {
                    st = st.fg(super::anim::mix(fg, pal.bg, t));
                }
                buf[(x, y)].set_char(c.ch).set_style(st);
            }
        }
    }

    /// The plays, two rows each: the title and when, then who and how long.
    /// A play's drafts stand above it as a tree.
    fn draw_list(&self, f: &mut Frame, area: Rect) {
        let per = 3usize;
        let rows = (area.height as usize / per).max(1);
        let start = self.sel.saturating_sub(rows.saturating_sub(1));
        let guide = Style::default().fg(FAINT);
        let mut lines = Vec::new();
        for (i, p) in self.plays.iter().enumerate().skip(start).take(rows) {
            let sel = i == self.sel;
            let w = area.width as usize;
            // The lines down the left, the tree growing upward: one per
            // level, a line where siblings stand above, then the branch.
            let firsts = &self.lasts[i];
            let level = |&first: &bool| if first { "   " } else { "│  " };
            let above: String = firsts
                .iter()
                .take(firsts.len().saturating_sub(1))
                .map(level)
                .collect();
            let branch = match firsts.last() {
                Some(true) => "┌─ ",
                Some(false) => "├─ ",
                None => "",
            };
            // Under a draft its line goes on down to the play it comes from.
            let under: String = firsts
                .iter()
                .take(firsts.len().saturating_sub(1))
                .map(level)
                .chain(firsts.last().map(|_| "│  "))
                .chain(std::iter::once("  "))
                .collect();
            // A draft of the same title goes by its draft number.
            let parent = p
                .parent
                .as_ref()
                .and_then(|up| self.plays.iter().find(|x| x.path == *up));
            let same = parent.is_some_and(|x| x.title == p.title);
            let name = match (&p.draft, same) {
                (Some(d), true) => format!("{} {d}", lang::l("taslak", "draft")),
                (None, true) => p
                    .path
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default(),
                _ => p.title.clone(),
            };
            let stamp = when(p.modified);
            let lead = above.width() + branch.width();
            let room = w.saturating_sub(stamp.width() + 6 + lead);
            let mut head = vec![
                popup::mark(sel),
                Span::styled(format!("{above}{branch}"), guide),
            ];
            match self.inline.as_ref().filter(|_| sel) {
                // Being renamed: a band to type on, the cursor in it.
                Some(ed) => {
                    let chars: Vec<char> = ed.value.chars().collect();
                    let width = |cs: &[char]| cs.iter().collect::<String>().width();
                    let mut skip = 0;
                    while skip < ed.at && width(&chars[skip..ed.at]) + 1 > room {
                        skip += 1;
                    }
                    let mut shown = String::new();
                    for &c in &chars[skip..] {
                        if (shown.width() + c.to_string().width()) >= room {
                            break;
                        }
                        shown.push(c);
                    }
                    let band = Style::default().fg(FG).bg(popup::FIELD);
                    let pad = room.saturating_sub(shown.width());
                    head.push(Span::styled(
                        shown,
                        if ed.all {
                            band.add_modifier(Modifier::REVERSED)
                        } else {
                            band
                        },
                    ));
                    head.push(Span::styled(" ".repeat(pad), band));
                    let gap = w.saturating_sub(2 + lead + room + stamp.width());
                    head.push(Span::raw(" ".repeat(gap)));
                    let x = area.x as usize + 2 + lead + width(&chars[skip..ed.at]);
                    let y = area.y as usize + (i - start) * per;
                    f.set_cursor_position(ratatui::layout::Position::new(x as u16, y as u16));
                }
                None => {
                    let title = popup::fit(&name, room);
                    let gap = w.saturating_sub(2 + lead + title.width() + stamp.width());
                    head.push(Span::styled(
                        title,
                        Style::default()
                            .fg(if sel { BEIGE } else { FG })
                            .add_modifier(Modifier::BOLD),
                    ));
                    head.push(Span::raw(" ".repeat(gap)));
                }
            }
            head.push(Span::styled(stamp, Style::default().fg(FAINT)));
            lines.push(popup::row(head, sel, area.width));
            let mut about = Vec::new();
            if !p.author.is_empty() && !parent.is_some_and(|x| x.author == p.author) {
                about.push(p.author.clone());
            }
            if let Some(d) = &p.draft
                && !same
            {
                about.push(format!("{} {d}", lang::l("taslak", "draft")));
            }
            about.push(lang::f(
                "{} perde · {} sahne · ≈ {}",
                "{} acts · {} scenes · ≈ {}",
                &[&p.acts, &p.scenes, &timing::format(p.secs)],
            ));
            if p.elsewhere {
                about.push(config::tilde(&p.path));
            }
            lines.push(popup::row(
                vec![
                    Span::raw("  "),
                    Span::styled(under.clone(), guide),
                    Span::styled(
                        popup::fit(&about.join("  ·  "), w.saturating_sub(2 + under.width())),
                        Style::default().fg(MUTED),
                    ),
                ],
                sel,
                area.width,
            ));
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(under, guide),
            ]));
        }
        f.render_widget(Paragraph::new(lines).style(theme::base()), area);
    }
}

/// The chosen play's cover in miniature, on a page like the writer's, with
/// its numbers under it.
/// `?`: every key of the home screen.
fn draw_keys(f: &mut Frame, area: Rect) {
    let keys: [(&str, &str, &str); 12] = [
        ("⏎", "oyunu aç", "open the play"),
        ("n", "yeni oyun", "a new play"),
        (
            "o",
            "başka yerden bir .tex aç",
            "open a .tex from elsewhere",
        ),
        ("d", "kopyasını çıkar", "make a copy"),
        ("f", "yeni taslak", "a new draft"),
        ("p", "PDF'i aç", "open the PDF"),
        ("F2", "adını satırda değiştir", "rename in its row"),
        (
            "Shift+F2",
            "başlık ve dosya adı ayrı",
            "title and file name apart",
        ),
        ("Del Del", "çöp kutusuna", "to the trash"),
        ("u", "çöpten geri getir", "back from the trash"),
        ("s", "ayarlar", "settings"),
        ("q", "çık", "quit"),
    ];
    let w = 46u16.min(area.width.saturating_sub(4));
    let h = keys.len() as u16 + 4;
    let r = Rect {
        x: area.x + area.width.saturating_sub(w) / 2,
        y: area.y + area.height.saturating_sub(h) / 3,
        width: w,
        height: h.min(area.height),
    };
    popup::float(f, r);
    f.render_widget(popup::panel(lang::l("tuşlar", "keys")), r);
    let lines: Vec<Line> = keys
        .iter()
        .map(|&(k, tr, en)| {
            Line::from(vec![
                Span::styled(format!("{k:>9}  "), Style::default().fg(BEIGE)),
                Span::styled(
                    if lang::is_tr() { tr } else { en },
                    Style::default().fg(MUTED),
                ),
            ])
        })
        .collect();
    f.render_widget(Paragraph::new(lines).style(theme::base()), popup::inner(r));
}

fn draw_card(f: &mut Frame, area: Rect, p: &Play, theme_name: &str) {
    let pal = theme::palette(theme_name);
    let base = Style::default().fg(pal.fg).bg(pal.bg);
    let w = area.width.saturating_sub(4) as usize;
    let center = |s: String, st: Style| Line::from(Span::styled(popup::fit(&s, w), st)).centered();
    let mut lines = vec![
        Line::from(""),
        Line::from(""),
        center(
            dactile::cast::upper_tr(&p.title),
            base.add_modifier(Modifier::BOLD),
        ),
        center(p.author.clone(), base),
    ];
    if p.final_mode {
        if !p.starred.is_empty() {
            lines.push(Line::from(""));
            lines.push(center(
                p.starred.join(", "),
                base.fg(pal.star).add_modifier(Modifier::BOLD),
            ));
        }
    } else {
        let draft = match &p.draft {
            Some(d) => format!("{} {d}", lang::l("TASLAK", "DRAFT")),
            None => lang::l("TASLAK", "DRAFT").to_string(),
        };
        lines.push(center(draft, base.fg(pal.muted)));
    }
    lines.push(Line::from(""));
    lines.push(center("─".repeat(w / 2), base.fg(pal.faint)));
    lines.push(Line::from(""));
    let fact = |s: String| center(s, base.fg(pal.muted));
    lines.push(fact(lang::f(
        "{} perde · {} sahne",
        "{} acts · {} scenes",
        &[&p.acts, &p.scenes],
    )));
    lines.push(fact(lang::f(
        "{} kişi · {} kelime",
        "{} characters · {} words",
        &[&p.characters, &p.words],
    )));
    lines.push(fact(lang::f(
        "≈ {} oynanır",
        "plays ≈ {}",
        &[&timing::format(p.secs)],
    )));
    f.render_widget(Paragraph::new(lines).style(base), area);
}

/// The file name a title typed in its row gives: the title's, with a
/// draft's number kept at its end (`kar-masali-0.2.tex`, "Kış Masalı" →
/// `kis-masali-0.2.tex`).
fn inline_name(path: &Path, title: &str, draft: Option<&str>) -> String {
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let fresh = library::file_name(title);
    let fresh = fresh.trim_end_matches(".tex");
    // The tail a new draft got from its number, or a number of digits.
    let tag = draft.map(draft_tag).filter(|t| !t.is_empty());
    let tail = match tag {
        Some(t) if stem.ends_with(&format!("-{t}")) && stem.len() > t.len() + 1 => {
            format!("-{t}")
        }
        _ => stem[library::draft_base(&stem).len()..].to_string(),
    };
    format!("{fresh}{tail}.tex")
}

/// A draft number as a file name ends with it: letters, digits and dots
/// stay, the rest becomes a dash (`İkinci el` → `ikinci-el`).
fn draft_tag(draft: &str) -> String {
    let tag: String = draft
        .trim()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' {
                c.to_ascii_lowercase().to_string()
            } else {
                let f = dactile::cast::fold_ascii(&c.to_string());
                if f.is_empty() { "-".into() } else { f }
            }
        })
        .collect();
    tag.trim_matches('-').to_string()
}

/// `kar-masali.tex` with draft 0.2 → `kar-masali-0.2.tex`; from
/// `kar-masali-0.2.tex`, 0.3 → `kar-masali-0.3.tex`.
fn fork_name(src: &Path, draft: &str) -> String {
    let stem = src
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    // Without an earlier draft's number at its end.
    let base = library::draft_base(&stem).to_string();
    let draft = draft_tag(draft);
    if draft.is_empty() {
        format!("{base}-2.tex")
    } else {
        format!("{base}-{draft}.tex")
    }
}

/// A path typed in a form: `~/…`, absolute, or inside the plays folder.
fn resolve_path(s: &str, cfg: &Config) -> PathBuf {
    let s = s.trim();
    if s.starts_with('~') || s.starts_with('/') {
        config::expand(s)
    } else {
        cfg.plays_dir().join(s)
    }
}

fn new_play_form() -> Form {
    Form {
        kind: FormKind::NewPlay,
        title: lang::l("yeni oyun", "new play").into(),
        fields: vec![
            Field::text("Başlık", "", Vec::new()),
            Field::text("Dosya", "", Vec::new()),
        ],
        focus: 0,
        at: 0..0,
        error: None,
        confirmed: false,
    }
}

fn settings_form(cfg: &Config, first: bool) -> Form {
    let langs = vec!["auto".to_string(), "tr".to_string(), "en".to_string()];
    let menus = vec![
        lang::l("otomatik (\\ ile)", "automatic (on \\)").to_string(),
        lang::l("elle (yalnız ^K)", "by hand (^K only)").to_string(),
    ];
    let keyboards = vec![
        lang::l("Türkçe Q", "Turkish Q").to_string(),
        lang::l("Türkçe F", "Turkish F").to_string(),
        "US".to_string(),
    ];
    let keyboard = match cfg.keyboard.as_str() {
        "f" => keyboards[1].clone(),
        "us" => keyboards[2].clone(),
        _ => keyboards[0].clone(),
    };
    let menu = if cfg.menu == "manual" {
        menus[1].clone()
    } else {
        menus[0].clone()
    };
    Form {
        kind: FormKind::Settings,
        title: if first {
            lang::l("Dactile'a hoş geldin", "welcome to Dactile")
        } else {
            lang::l("ayarlar", "settings")
        }
        .into(),
        fields: vec![
            Field::text("Dil", &cfg.lang, langs),
            Field::text("Oyunlar klasörü", &cfg.plays_dir, Vec::new()),
            Field::text("Adın", &cfg.author, Vec::new()),
            Field::text("Öneri menüsü", &menu, menus),
            Field::text("Dakikada kelime", &cfg.wpm.to_string(), Vec::new()),
            Field::text("Klavye", &keyboard, keyboards),
            Field::flag("Animasyonlar", cfg.animations),
        ],
        focus: 0,
        at: 0..0,
        error: None,
        confirmed: false,
    }
}

/// "bugün 14:20", "dün 09:05", "12.10.2026".
fn when(t: Option<std::time::SystemTime>) -> String {
    let Some(t) = t else { return String::new() };
    let t: DateTime<Local> = t.into();
    let today = Local::now().date_naive();
    let d = t.date_naive();
    if d == today {
        format!("{} {}", lang::l("bugün", "today"), t.format("%H:%M"))
    } else if today.pred_opt() == Some(d) {
        format!("{} {}", lang::l("dün", "yesterday"), t.format("%H:%M"))
    } else {
        t.format("%d.%m.%Y").to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_typed_in_the_row() {
        assert_eq!(
            inline_name(Path::new("/x/kar-masali.tex"), "Kış Masalı", Some("0.1")),
            "kis-masali.tex"
        );
        assert_eq!(
            inline_name(
                Path::new("/x/kar-masali-0.2.tex"),
                "Kış Masalı",
                Some("0.2")
            ),
            "kis-masali-0.2.tex"
        );
        assert_eq!(
            inline_name(Path::new("/x/kar-masali-26.10.2026.tex"), "Kar 2", None),
            "kar-2-26.10.2026.tex"
        );
        assert_eq!(
            inline_name(Path::new("/x/kar-masali-0.2b.tex"), "Kış", Some("0.2b")),
            "kis-0.2b.tex"
        );
        assert_eq!(
            inline_name(
                Path::new("/x/kar-masali-ikinci-el.tex"),
                "Kış",
                Some("İkinci el")
            ),
            "kis-ikinci-el.tex"
        );
    }

    #[test]
    fn draft_file_names() {
        let p = Path::new("/x/kar-masali.tex");
        assert_eq!(fork_name(p, "0.2"), "kar-masali-0.2.tex");
        assert_eq!(
            fork_name(Path::new("/x/kar-masali-0.2.tex"), "0.3"),
            "kar-masali-0.3.tex"
        );
        assert_eq!(fork_name(p, "26.10.2026"), "kar-masali-26.10.2026.tex");
        assert_eq!(fork_name(p, "İkinci el"), "kar-masali-ikinci-el.tex");
        assert_eq!(fork_name(p, ""), "kar-masali-2.tex");
    }
}
