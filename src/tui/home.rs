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

/// How long the home screen types itself out.
const TYPE_FOR: std::time::Duration = std::time::Duration::from_millis(2200);

pub enum Action {
    Stay,
    Open(PathBuf),
    Quit,
}

pub struct Home {
    plays: Vec<Play>,
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
}

impl Home {
    pub fn new(cfg: &Config, first_run: bool) -> Home {
        lang::set(lang::resolve(&cfg.lang, None));
        let mut h = Home {
            plays: library::list(cfg),
            sel: 0,
            form: None,
            message: None,
            first_run,
            pending: None,
            born: std::time::Instant::now(),
            form_born: std::time::Instant::now(),
            form_kind: None,
        };
        if first_run {
            h.form = Some(settings_form(cfg, true));
        }
        h
    }

    pub fn refresh(&mut self, cfg: &Config) {
        lang::set(lang::resolve(&cfg.lang, None));
        self.plays = library::list(cfg);
        self.sel = self.sel.min(self.plays.len().saturating_sub(1));
    }

    pub fn key(&mut self, k: KeyEvent, cfg: &mut Config) -> Action {
        self.message = None;
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
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        match k.code {
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
                || self.form_born.elapsed() < std::time::Duration::from_millis(160))
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
                // The character cards and the play's own words go along.
                let side = dactile::sidecar::path_for(&src);
                if side.is_file() {
                    let _ = std::fs::copy(&side, dactile::sidecar::path_for(&path));
                }
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
            // The list, and a card for the chosen play when there is room.
            let card_w = if body.width >= 90 { 40 } else { 0 };
            let list = Rect {
                width: body
                    .width
                    .saturating_sub(card_w + if card_w > 0 { 4 } else { 0 }),
                ..body
            };
            self.draw_list(f, list);
            if card_w > 0
                && let Some(p) = self.plays.get(self.sel)
            {
                let card = Rect {
                    x: body.x + body.width - card_w,
                    width: card_w,
                    height: body.height.min(16),
                    ..body
                };
                draw_card(f, card, p, cfg.theme());
            }
        }
        // The spell checker, so a missing dictionary is noticed here.
        let dict = match spell::find() {
            Some((aff, _)) => Span::styled(
                lang::f(
                    "yazım denetimi: {}",
                    "spell check: {}",
                    &[&config::tilde(&aff.with_extension(""))],
                ),
                Style::default().fg(FAINT),
            ),
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
        let keys = [
            ("⏎", lang::l("aç", "open")),
            ("n", lang::l("yeni oyun", "new play")),
            ("o", lang::l("dosya aç", "open a file")),
            ("d", lang::l("kopya", "copy")),
            ("f", lang::l("yeni taslak", "new draft")),
            ("s", lang::l("ayarlar", "settings")),
            ("q", lang::l("çık", "quit")),
        ];
        let mut spans = Vec::new();
        for (k, what) in keys {
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

    /// The plays, two rows each: the title and when, then who and how long.
    fn draw_list(&self, f: &mut Frame, area: Rect) {
        let per = 3usize;
        let rows = (area.height as usize / per).max(1);
        let start = self.sel.saturating_sub(rows.saturating_sub(1));
        let mut lines = Vec::new();
        for (i, p) in self.plays.iter().enumerate().skip(start).take(rows) {
            let sel = i == self.sel;
            let w = area.width as usize;
            let stamp = when(p.modified);
            let title = popup::fit(&p.title, w.saturating_sub(stamp.width() + 6));
            let gap = w.saturating_sub(2 + title.width() + stamp.width());
            lines.push(popup::row(
                vec![
                    popup::mark(sel),
                    Span::styled(
                        title,
                        Style::default()
                            .fg(if sel { BEIGE } else { FG })
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" ".repeat(gap)),
                    Span::styled(stamp, Style::default().fg(FAINT)),
                ],
                sel,
                area.width,
            ));
            let mut about = Vec::new();
            if !p.author.is_empty() {
                about.push(p.author.clone());
            }
            if let Some(d) = &p.draft {
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
                    Span::styled(
                        popup::fit(&about.join("  ·  "), w.saturating_sub(2)),
                        Style::default().fg(MUTED),
                    ),
                ],
                sel,
                area.width,
            ));
            lines.push(Line::from(""));
        }
        f.render_widget(Paragraph::new(lines).style(theme::base()), area);
    }
}

/// The chosen play's cover in miniature, on a page like the writer's, with
/// its numbers under it.
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

/// `kar-masali.tex` with draft 0.2 → `kar-masali-0.2.tex`; from
/// `kar-masali-0.2.tex`, 0.3 → `kar-masali-0.3.tex`.
fn fork_name(src: &Path, draft: &str) -> String {
    let stem = src
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    // Without an earlier draft's number at its end.
    let base = stem
        .rsplit_once('-')
        .filter(|(_, tail)| tail.chars().all(|c| c.is_ascii_digit() || c == '.'))
        .map_or(stem.as_str(), |(b, _)| b)
        .to_string();
    // Letters, digits and dots stay; the rest becomes a dash.
    let draft: String = draft
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
    let draft = draft.trim_matches('-');
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
