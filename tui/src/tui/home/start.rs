//! The home screen's left column (Giriş, Yeni, Aç, Ayarlar, Tuşlar) and
//! what each shows: the tiles a new play starts from, the templates, the
//! ways to open a file.

use super::*;
use dactile::templates::{self, From, Template};
use ratatui::crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::widgets::{Block, BorderType, Borders};

/// The width of the left column, and the air right of it.
pub(super) const SIDE_W: u16 = 16;
const SIDE_GAP: u16 = 2;
/// A tile's height, borders and all.
const TILE_H: u16 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Section {
    Home,
    New,
    Open,
    Settings,
    Keys,
}

const SECTIONS: [Section; 5] = [
    Section::Home,
    Section::New,
    Section::Open,
    Section::Settings,
    Section::Keys,
];

impl Section {
    fn name(self) -> &'static str {
        match self {
            Section::Home => lang::l("Giriş", "Home"),
            Section::New => lang::l("Yeni", "New"),
            Section::Open => lang::l("Aç", "Open"),
            Section::Settings => lang::l("Ayarlar", "Settings"),
            Section::Keys => lang::l("Tuşlar", "Keys"),
        }
    }
}

/// Where the keys go: the column, the tiles over the list, or the section.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Focus {
    Side,
    Tiles,
    Main,
}

/// What a tile or a template does.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Start {
    Make(Template),
    Open,
    Example,
}

/// An entry under Aç.
#[derive(Debug, Clone, PartialEq)]
enum OpenItem {
    Pick,
    Import,
    Recent(PathBuf),
}

/// Where things are on screen, for the mouse.
#[derive(Default)]
pub(super) struct Hits {
    pub(super) side: Vec<(u16, Section)>,
    tiles: Vec<(Rect, usize)>,
    /// A play's first row and how many it takes.
    pub(super) list: Vec<(u16, u16, usize)>,
    grid: Vec<(Rect, usize)>,
    open: Vec<(u16, usize)>,
}

fn cur() -> Lang {
    if lang::is_tr() { Lang::Tr } else { Lang::En }
}

fn inside(r: Rect, x: u16, y: u16) -> bool {
    x >= r.x && x < r.x + r.width && y >= r.y && y < r.y + r.height
}

impl Home {
    /// The column and what is right of it; no column on a narrow screen.
    pub(super) fn split(&self, body: Rect) -> (Option<Rect>, Rect) {
        if body.width < 76 || self.first_run {
            return (None, body);
        }
        let col = Rect {
            width: SIDE_W,
            ..body
        };
        let content = Rect {
            x: body.x + SIDE_W + SIDE_GAP,
            width: body.width - SIDE_W - SIDE_GAP,
            ..body
        };
        (Some(col), content)
    }

    /// The tiles over the list: the empty play, two skeletons, opening,
    /// the example.
    pub(super) fn home_tiles(&self) -> Vec<(String, String, Start)> {
        let all = templates::all(cur());
        let mut out: Vec<(String, String, Start)> = all
            .into_iter()
            .filter(|t| !matches!(t.from, From::Own(_)))
            .take(3)
            .map(|t| {
                let name = if t.from == From::Blank {
                    format!("+ {}", t.name)
                } else {
                    t.name.clone()
                };
                (name, t.about.clone(), Start::Make(t))
            })
            .collect();
        out.push((
            lang::l("Aç / içe aktar", "Open / import").into(),
            lang::l(".tex, Word, PDF, metin", ".tex, Word, PDF, text").into(),
            Start::Open,
        ));
        out.push((
            lang::l("Örnek oyun", "The example").into(),
            lang::l("sınıfın her şeyi, bir kopya", "all the class does, a copy").into(),
            Start::Example,
        ));
        out
    }

    /// Under Yeni: every template, and the example last.
    fn new_items(&self) -> Vec<(String, String, Start)> {
        let mut out: Vec<(String, String, Start)> = self
            .templates
            .iter()
            .map(|t| (t.name.clone(), t.about.clone(), Start::Make(t.clone())))
            .collect();
        out.push((
            lang::l("Örnek oyun", "The example").into(),
            lang::l("sınıfın her şeyi, bir kopya", "all the class does, a copy").into(),
            Start::Example,
        ));
        out
    }

    fn open_items(&self, cfg: &Config) -> Vec<(String, String, OpenItem)> {
        let mut out = vec![
            (
                lang::l("Dosya aç…", "Open a file…").to_string(),
                lang::l("bir .tex, yolunu yazarak", "a .tex, by its path").to_string(),
                OpenItem::Pick,
            ),
            (
                lang::l("İçe aktar…", "Import…").to_string(),
                lang::l(
                    "Word, LibreOffice, PDF ya da metin belgesi",
                    "a Word, LibreOffice, PDF or text document",
                )
                .to_string(),
                OpenItem::Import,
            ),
        ];
        for r in &cfg.recent {
            let p = config::expand(r);
            if p.is_file() {
                let name = p
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                let at = p.parent().map(config::tilde).unwrap_or_default();
                out.push((name, at, OpenItem::Recent(p)));
            }
        }
        out
    }

    /// Goes to a section, the keys in it.
    pub(super) fn go_section(&mut self, s: Section, cfg: &Config) {
        self.section = s;
        self.focus = Focus::Main;
        match s {
            Section::New => {
                self.templates = templates::all(cur());
                self.grid = self.grid.min(self.templates.len());
            }
            Section::Settings => self.form = Some(settings_form(cfg, false)),
            _ => {}
        }
    }

    /// What a tile or a template does.
    pub(super) fn start(&mut self, s: Start, cfg: &mut Config) -> Action {
        match s {
            Start::Make(t) => {
                let mut form = new_play_form(cfg);
                if t.from != From::Blank {
                    form.title = format!("{} — {}", form.title, t.name);
                }
                self.new_from = t;
                self.form = Some(form);
            }
            Start::Open => self.go_section(Section::Open, cfg),
            Start::Example => match templates::copy_example(&cfg.plays_dir()) {
                Ok(p) => return Action::Open(p),
                Err(e) => self.message = Some((format!("{e:#}"), true)),
            },
        }
        Action::Stay
    }

    fn run_open(&mut self, item: OpenItem, cfg: &Config) -> Action {
        match item {
            OpenItem::Pick => self.open_form(cfg),
            OpenItem::Import => self.import(),
            OpenItem::Recent(p) => return Action::Open(p),
        }
        Action::Stay
    }

    /// The keys the column and the sections take; `None` leaves the key to
    /// the list's.
    pub(super) fn section_key(&mut self, k: KeyEvent, cfg: &mut Config) -> Option<Action> {
        let has_side = self.side_shown;
        match k.code {
            KeyCode::Tab | KeyCode::BackTab => {
                let back = k.code == KeyCode::BackTab;
                let order: &[Focus] = match (self.section, has_side) {
                    (Section::Home, true) => &[Focus::Side, Focus::Tiles, Focus::Main],
                    (Section::Home, false) => &[Focus::Tiles, Focus::Main],
                    (_, true) => &[Focus::Side, Focus::Main],
                    (_, false) => &[Focus::Main],
                };
                let i = order.iter().position(|f| *f == self.focus).unwrap_or(0);
                let n = order.len();
                self.focus = order[if back { (i + n - 1) % n } else { (i + 1) % n }];
                return Some(Action::Stay);
            }
            _ => {}
        }
        match self.focus {
            Focus::Side => {
                let i = SECTIONS
                    .iter()
                    .position(|s| *s == self.section)
                    .unwrap_or(0);
                match k.code {
                    KeyCode::Up => self.section = SECTIONS[(i + 4) % 5],
                    KeyCode::Down => self.section = SECTIONS[(i + 1) % 5],
                    KeyCode::Right | KeyCode::Enter => {
                        self.go_section(self.section, cfg);
                        if self.section == Section::Home {
                            self.focus = Focus::Tiles;
                        }
                    }
                    KeyCode::Esc if self.section != Section::Home => self.section = Section::Home,
                    _ => return None,
                }
                Some(Action::Stay)
            }
            Focus::Tiles => {
                let n = self.home_tiles().len();
                match k.code {
                    KeyCode::Left if self.tile == 0 && has_side => self.focus = Focus::Side,
                    KeyCode::Left => self.tile = self.tile.saturating_sub(1),
                    KeyCode::Right => self.tile = (self.tile + 1).min(n - 1),
                    KeyCode::Down => self.focus = Focus::Main,
                    KeyCode::Up => {}
                    KeyCode::Enter => {
                        let s = self.home_tiles().swap_remove(self.tile.min(n - 1)).2;
                        return Some(self.start(s, cfg));
                    }
                    _ => return None,
                }
                Some(Action::Stay)
            }
            Focus::Main => match self.section {
                Section::Home => match k.code {
                    KeyCode::Up if self.sel == 0 || self.plays.is_empty() => {
                        self.focus = Focus::Tiles;
                        Some(Action::Stay)
                    }
                    KeyCode::Left if has_side => {
                        self.focus = Focus::Side;
                        Some(Action::Stay)
                    }
                    KeyCode::Enter if self.plays.is_empty() => {
                        self.focus = Focus::Tiles;
                        Some(Action::Stay)
                    }
                    _ => None,
                },
                Section::New => {
                    let items = self.new_items();
                    let n = items.len();
                    let cols = self.grid_cols.max(1);
                    let g = self.grid.min(n - 1);
                    match k.code {
                        KeyCode::Left if g.is_multiple_of(cols) && has_side => {
                            self.focus = Focus::Side
                        }
                        KeyCode::Left => self.grid = g.saturating_sub(1),
                        KeyCode::Right => self.grid = (g + 1).min(n - 1),
                        KeyCode::Up => self.grid = g.saturating_sub(cols),
                        KeyCode::Down => self.grid = (g + cols).min(n - 1),
                        KeyCode::Enter => {
                            let s = items.into_iter().nth(g).map(|i| i.2);
                            return s.map(|s| self.start(s, cfg));
                        }
                        // One's own template, to the trash.
                        KeyCode::Delete | KeyCode::Char('d') => {
                            if let Some(Start::Make(Template {
                                from: From::Own(p),
                                name,
                                ..
                            })) = items.into_iter().nth(g).map(|i| i.2)
                            {
                                if self.arming.as_ref() == Some(&p) {
                                    match dactile::trash::throw(
                                        std::slice::from_ref(&p),
                                        &dactile::trash::bin(),
                                    ) {
                                        Ok(_) => {
                                            self.message = Some((
                                                lang::f(
                                                    "{} çöp kutusuna gitti.",
                                                    "{} went to the trash.",
                                                    &[&name],
                                                ),
                                                false,
                                            ));
                                            self.templates = templates::all(cur());
                                        }
                                        Err(e) => self.message = Some((format!("{e:#}"), true)),
                                    }
                                } else {
                                    self.arming = Some(p);
                                    self.message = Some((
                                        lang::f(
                                            "{} şablonu çöpe atılsın mı? Bir kez daha bas.",
                                            "Throw the template {} away? Press it again.",
                                            &[&name],
                                        ),
                                        true,
                                    ));
                                }
                            } else {
                                self.message = Some((
                                    lang::l(
                                        "Yalnız kendi şablonların silinir.",
                                        "Only your own templates can go.",
                                    )
                                    .into(),
                                    true,
                                ));
                            }
                        }
                        KeyCode::Esc => {
                            self.section = Section::Home;
                            self.focus = Focus::Main;
                        }
                        _ => return None,
                    }
                    Some(Action::Stay)
                }
                Section::Open => {
                    let items = self.open_items(cfg);
                    let n = items.len();
                    match k.code {
                        KeyCode::Up => self.open_sel = self.open_sel.saturating_sub(1),
                        KeyCode::Down => self.open_sel = (self.open_sel + 1).min(n - 1),
                        KeyCode::Left if has_side => self.focus = Focus::Side,
                        KeyCode::Enter => {
                            let item = items.into_iter().nth(self.open_sel).map(|i| i.2);
                            return item.map(|i| self.run_open(i, cfg));
                        }
                        KeyCode::Esc => {
                            self.section = Section::Home;
                            self.focus = Focus::Main;
                        }
                        _ => return None,
                    }
                    Some(Action::Stay)
                }
                Section::Settings => match k.code {
                    // The form closed: Enter opens it again.
                    KeyCode::Enter => {
                        self.form = Some(settings_form(cfg, false));
                        Some(Action::Stay)
                    }
                    KeyCode::Left if has_side => {
                        self.focus = Focus::Side;
                        Some(Action::Stay)
                    }
                    KeyCode::Esc => {
                        self.section = Section::Home;
                        Some(Action::Stay)
                    }
                    _ => None,
                },
                Section::Keys => match k.code {
                    KeyCode::Left if has_side => {
                        self.focus = Focus::Side;
                        Some(Action::Stay)
                    }
                    KeyCode::Esc | KeyCode::Char('?') => {
                        self.section = Section::Home;
                        Some(Action::Stay)
                    }
                    _ => None,
                },
            },
        }
    }

    pub(super) fn draw_side(&mut self, f: &mut Frame, r: Rect) {
        self.hits.side.clear();
        let mut lines = Vec::new();
        for (i, s) in SECTIONS.iter().enumerate() {
            let on = *s == self.section;
            let lit = on && self.focus == Focus::Side;
            let mut style = Style::default().fg(if on { BEIGE } else { MUTED });
            if on {
                style = style.add_modifier(Modifier::BOLD);
            }
            let bar = Span::styled(if on { "▌ " } else { "  " }, Style::default().fg(CLARET));
            let text = format!("{:<w$}", s.name(), w = SIDE_W as usize - 2);
            let mut spans = vec![bar, Span::styled(text, style)];
            if lit {
                for sp in &mut spans {
                    sp.style = sp.style.bg(theme::SELECTION);
                }
            }
            lines.push(Line::from(spans));
            lines.push(Line::from(""));
            self.hits.side.push((r.y + i as u16 * 2, *s));
        }
        f.render_widget(Paragraph::new(lines).style(theme::base()), r);
        // A thin rule between the column and the rest.
        let x = r.x + r.width;
        for y in r.y..r.y + r.height {
            f.render_widget(
                Paragraph::new("│").style(theme::base().fg(FAINT)),
                Rect {
                    x,
                    y,
                    width: 1,
                    height: 1,
                },
            );
        }
    }

    /// A tile: a name and a line on it, framed.
    fn draw_tile(f: &mut Frame, r: Rect, name: &str, about: &str, sel: bool, lit: bool) {
        let border = if lit {
            CLARET
        } else if sel {
            MUTED
        } else {
            FAINT
        };
        f.render_widget(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(border))
                .style(theme::base()),
            r,
        );
        let w = r.width.saturating_sub(4) as usize;
        let name_style = if lit {
            Style::default().fg(BEIGE).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(FG)
        };
        f.render_widget(
            Paragraph::new(vec![
                Line::from(Span::styled(popup::fit(name, w), name_style)),
                Line::from(Span::styled(
                    popup::fit(about, w),
                    Style::default().fg(FAINT),
                )),
            ])
            .style(theme::base()),
            Rect {
                x: r.x + 2,
                y: r.y + 1,
                width: r.width.saturating_sub(4),
                height: r.height.saturating_sub(2),
            },
        );
    }

    /// The tiles over the list; what they leave below.
    pub(super) fn draw_tiles(&mut self, f: &mut Frame, area: Rect) -> Rect {
        self.hits.tiles.clear();
        let tiles = self.home_tiles();
        let gap = 1u16;
        let n = tiles.len() as u16;
        let w = (area.width.saturating_sub(gap * (n - 1)) / n).max(16);
        let fit = ((area.width + gap) / (w + gap)).min(n);
        self.tile = self.tile.min(tiles.len() - 1);
        // The chosen tile stays among those shown.
        let first = (self.tile as u16 + 1).saturating_sub(fit) as usize;
        for (k, (i, (name, about, _))) in tiles
            .iter()
            .enumerate()
            .skip(first)
            .take(fit as usize)
            .enumerate()
        {
            let r = Rect {
                x: area.x + k as u16 * (w + gap),
                y: area.y,
                width: w,
                height: TILE_H.min(area.height),
            };
            let sel = i == self.tile;
            Self::draw_tile(f, r, name, about, sel, sel && self.focus == Focus::Tiles);
            self.hits.tiles.push((r, i));
        }
        let label_y = area.y + TILE_H + 1;
        f.render_widget(
            Paragraph::new(Span::styled(
                lang::l("Son oyunlar", "Recent plays"),
                Style::default().fg(MUTED).add_modifier(Modifier::BOLD),
            ))
            .style(theme::base()),
            Rect {
                y: label_y,
                height: 1,
                ..area
            },
        );
        let top = label_y + 2;
        Rect {
            y: top,
            height: (area.y + area.height).saturating_sub(top),
            ..area
        }
    }

    pub(super) fn draw_section(&mut self, f: &mut Frame, area: Rect, cfg: &Config) {
        let title = |f: &mut Frame, s: &str, about: &str| {
            f.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled(
                        s.to_string(),
                        Style::default().fg(BEIGE).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(format!("   {about}"), Style::default().fg(FAINT)),
                ]))
                .style(theme::base()),
                Rect { height: 1, ..area },
            );
        };
        let below = Rect {
            y: area.y + 2,
            height: area.height.saturating_sub(2),
            ..area
        };
        match self.section {
            Section::Home => {}
            Section::New => {
                title(
                    f,
                    lang::l("Yeni oyun", "A new play"),
                    lang::l(
                        "⏎ bununla başla · d kendi şablonunu siler · yazıcıda Dosya › Şablon olarak kaydet",
                        "⏎ start from it · d deletes one of yours · in the writer, File › Save as a template",
                    ),
                );
                self.hits.grid.clear();
                let items = self.new_items();
                let gap = 1u16;
                let w = 24u16.min(below.width);
                let cols = ((below.width + gap) / (w + gap)).max(1);
                self.grid_cols = cols as usize;
                let g = self.grid.min(items.len() - 1);
                let rows_fit = ((below.height + 1) / (TILE_H + 1)).max(1) as usize;
                let row_of = g / cols as usize;
                let first_row = (row_of + 1).saturating_sub(rows_fit);
                for (i, (name, about, _)) in items.iter().enumerate() {
                    let (row, col) = (i / cols as usize, i % cols as usize);
                    if row < first_row || row >= first_row + rows_fit {
                        continue;
                    }
                    let r = Rect {
                        x: below.x + col as u16 * (w + gap),
                        y: below.y + (row - first_row) as u16 * (TILE_H + 1),
                        width: w,
                        height: TILE_H,
                    };
                    let sel = i == g;
                    Self::draw_tile(f, r, name, about, sel, sel && self.focus == Focus::Main);
                    self.hits.grid.push((r, i));
                }
            }
            Section::Open => {
                title(
                    f,
                    lang::l("Aç", "Open"),
                    lang::l(
                        "⏎ aç · son açılanlar aşağıda",
                        "⏎ open · the recent ones below",
                    ),
                );
                self.hits.open.clear();
                let items = self.open_items(cfg);
                self.open_sel = self.open_sel.min(items.len() - 1);
                let mut lines = Vec::new();
                for (i, (name, about, item)) in items.iter().enumerate() {
                    if i == 2 {
                        lines.push(Line::from(""));
                        lines.push(Line::from(Span::styled(
                            lang::l("Son açılanlar", "Opened lately"),
                            Style::default().fg(MUTED).add_modifier(Modifier::BOLD),
                        )));
                    }
                    let sel = i == self.open_sel;
                    let y = below.y + lines.len() as u16;
                    if y >= below.y + below.height {
                        break;
                    }
                    self.hits.open.push((y, i));
                    let lit = sel && self.focus == Focus::Main;
                    let name_w = if matches!(item, OpenItem::Recent(_)) {
                        28
                    } else {
                        16
                    };
                    lines.push(popup::row(
                        vec![
                            popup::mark(sel),
                            Span::styled(
                                format!("{:<name_w$}", popup::fit(name, name_w)),
                                Style::default().fg(if sel { BEIGE } else { FG }),
                            ),
                            Span::styled(
                                format!(
                                    "  {}",
                                    popup::fit(
                                        about,
                                        (below.width as usize).saturating_sub(name_w + 4)
                                    )
                                ),
                                Style::default().fg(FAINT),
                            ),
                        ],
                        lit,
                        below.width,
                    ));
                }
                f.render_widget(Paragraph::new(lines).style(theme::base()), below);
            }
            Section::Settings => {
                title(
                    f,
                    lang::l("Ayarlar", "Settings"),
                    lang::l("⏎ düzenle", "⏎ edit"),
                );
                // The form shows here when open; when not, the values.
                if self.form.is_some() {
                    return;
                }
                let form = settings_form(cfg, false);
                let lines: Vec<Line> = form
                    .fields
                    .iter()
                    .map(|fl| {
                        Line::from(vec![
                            Span::styled(
                                format!("{:<18}", popup::label_of(&fl.label)),
                                Style::default().fg(MUTED),
                            ),
                            Span::styled(
                                if fl.toggle {
                                    (if fl.on() {
                                        lang::l("evet", "yes")
                                    } else {
                                        lang::l("hayır", "no")
                                    })
                                    .to_string()
                                } else {
                                    fl.value.clone()
                                },
                                Style::default().fg(FG),
                            ),
                        ])
                    })
                    .collect();
                f.render_widget(Paragraph::new(lines).style(theme::base()), below);
            }
            Section::Keys => draw_keys(f, area),
        }
    }

    pub fn mouse(&mut self, m: MouseEvent, cfg: &mut Config) -> Action {
        let MouseEventKind::Down(MouseButton::Left) = m.kind else {
            if self.section == Section::Home && self.form.is_none() {
                match m.kind {
                    MouseEventKind::ScrollUp => self.step_sel(false),
                    MouseEventKind::ScrollDown => self.step_sel(true),
                    _ => {}
                }
            }
            return Action::Stay;
        };
        if self.form.is_some()
            || self.importing.is_some()
            || self.inline.is_some()
            || self.files.is_some()
        {
            return Action::Stay;
        }
        let (x, y) = (m.column, m.row);
        if let Some(&(_, s)) = self.hits.side.iter().find(|(sy, _)| *sy == y)
            && x < self.side_right
        {
            self.go_section(s, cfg);
            if s != Section::Home && s != Section::Settings {
                self.focus = Focus::Main;
            }
            return Action::Stay;
        }
        match self.section {
            Section::Home => {
                if let Some(&(_, i)) = self.hits.tiles.iter().find(|(r, _)| inside(*r, x, y)) {
                    self.tile = i;
                    self.focus = Focus::Tiles;
                    let s = self.home_tiles().swap_remove(i).2;
                    return self.start(s, cfg);
                }
                if let Some(&(_, _, i)) = self
                    .hits
                    .list
                    .iter()
                    .find(|(top, h, _)| y >= *top && y < top + h)
                    && x >= self.side_right
                {
                    self.focus = Focus::Main;
                    if self.sel == i {
                        return Action::Open(self.plays[i].path.clone());
                    }
                    self.sel = i;
                }
            }
            Section::New => {
                if let Some(&(_, i)) = self.hits.grid.iter().find(|(r, _)| inside(*r, x, y)) {
                    self.grid = i;
                    self.focus = Focus::Main;
                    if let Some(s) = self.new_items().into_iter().nth(i).map(|i| i.2) {
                        return self.start(s, cfg);
                    }
                }
            }
            Section::Open => {
                if let Some(&(_, i)) = self.hits.open.iter().find(|(oy, _)| *oy == y) {
                    self.open_sel = i;
                    self.focus = Focus::Main;
                    if let Some(item) = self.open_items(cfg).into_iter().nth(i).map(|i| i.2) {
                        return self.run_open(item, cfg);
                    }
                }
            }
            Section::Settings => self.form = Some(settings_form(cfg, false)),
            Section::Keys => {}
        }
        Action::Stay
    }
}
