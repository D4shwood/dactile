//! F9: who plays whom, full screen. Three views, Tab between them: the
//! casting (one of the play's lineups, "A", "B"), the scene/role table, and
//! the numbers: lines, words and time by role, scene lengths.

use super::popup::{fit, mark, row};
use super::theme::{self, BEIGE, CLARET, FAINT, FG, MUTED};
use dactile::casting::{self, Presence, Role};
use dactile::lang;
use dactile::render::Doc;
use dactile::sidecar::Sidecar;
use dactile::timing;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph};
use unicode_width::UnicodeWidthStr;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Tab {
    #[default]
    Casting,
    Table,
    Stats,
}

impl Tab {
    pub fn next(self) -> Tab {
        match self {
            Tab::Casting => Tab::Table,
            Tab::Table => Tab::Stats,
            Tab::Stats => Tab::Casting,
        }
    }
}

/// Where the screen stands; kept while it is closed.
#[derive(Debug, Clone, Default)]
pub struct RolesPanel {
    pub tab: Tab,
    /// The chosen role, by index into the cast.
    pub sel: usize,
    /// The table's chosen scene and its first role column shown.
    pub row: usize,
    pub col: usize,
    /// The numbers' scroll.
    pub scroll: usize,
    /// `x` once: the casting goes on a second.
    pub confirm: bool,
    /// `k` once: which kind of copies, on a second.
    pub pick: bool,
}

/// What the three views show, read once per frame.
pub struct Data {
    pub table: Vec<Vec<Presence>>,
    pub roles: Vec<Role>,
}

impl Data {
    pub fn of(text: &str, doc: &Doc, wpm: u32) -> Data {
        let stage = dactile::stage::read(text, doc, &[]);
        let table = casting::table(text, doc, &stage);
        let roles = casting::roles(text, doc, &table, wpm);
        Data { table, roles }
    }
}

pub fn draw(
    f: &mut Frame,
    area: Rect,
    doc: &Doc,
    side: &Sidecar,
    data: &Data,
    wpm: u32,
    p: &RolesPanel,
) {
    f.render_widget(Clear, area);
    f.render_widget(Block::default().style(theme::base()), area);
    let mut head = vec![Span::styled(
        lang::l(" dağıtım", " casting"),
        Style::default().fg(CLARET).add_modifier(Modifier::BOLD),
    )];
    head.push(Span::raw("   "));
    if side.castings.is_empty() {
        head.push(Span::styled(
            lang::l("henüz dağıtım yok", "no casting yet"),
            Style::default().fg(FAINT),
        ));
    }
    for (i, c) in side.castings.iter().enumerate() {
        if i > 0 {
            head.push(Span::styled(" · ", Style::default().fg(FAINT)));
        }
        let name = if c.name.is_empty() { "?" } else { &c.name };
        head.push(if i == side.casting {
            Span::styled(
                format!("[{name}]"),
                Style::default().fg(BEIGE).add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled(name.to_string(), Style::default().fg(MUTED))
        });
    }
    f.render_widget(Paragraph::new(Line::from(head)), Rect { height: 1, ..area });
    let views = match p.tab {
        Tab::Casting => lang::l(
            "[dağıtım] · tablo · sayılar ",
            "[casting] · table · numbers ",
        ),
        Tab::Table => lang::l(
            "dağıtım · [tablo] · sayılar ",
            "casting · [table] · numbers ",
        ),
        Tab::Stats => lang::l(
            "dağıtım · tablo · [sayılar] ",
            "casting · table · [numbers] ",
        ),
    };
    let vw = views.width() as u16;
    f.render_widget(
        Paragraph::new(Span::styled(views, Style::default().fg(FAINT))),
        Rect {
            x: area.x + area.width.saturating_sub(vw),
            width: vw.min(area.width),
            height: 1,
            ..area
        },
    );
    let body = Rect {
        x: area.x + 1,
        y: area.y + 2,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(4),
    };
    if doc.cast.characters.is_empty() {
        f.render_widget(
            Paragraph::new(Span::styled(
                lang::l(
                    "Henüz karakter yok: ^O ile ekle.",
                    "No characters yet: ^O adds one.",
                ),
                Style::default().fg(MUTED),
            )),
            body,
        );
    } else {
        match p.tab {
            Tab::Casting => draw_casting(f, body, doc, side, data, p),
            Tab::Table => draw_table(f, body, doc, side, data, p),
            Tab::Stats => draw_stats(f, body, doc, data, wpm, p),
        }
    }
    let keys = keys_text(p, side, wpm);
    f.render_widget(
        Paragraph::new(fit(&keys, area.width.saturating_sub(2) as usize))
            .style(theme::base().fg(if p.confirm || p.pick { BEIGE } else { FAINT })),
        Rect {
            x: area.x + 1,
            y: area.y + area.height.saturating_sub(1),
            width: area.width.saturating_sub(2),
            height: 1,
        },
    );
}

fn keys_text(p: &RolesPanel, side: &Sidecar, wpm: u32) -> String {
    if p.confirm {
        let name = side.current().map(|c| c.name.as_str()).unwrap_or("");
        return lang::f(
            "\"{}\" dağıtımı silinsin mi? x evet, başka bir tuş hayır",
            "Delete the casting \"{}\"? x yes, any other key no",
            &[&name],
        );
    }
    if p.pick {
        return lang::l(
            "Kopyalar: 1 tam  2 prova (replikler boş)  3 prova, hafif (ilk kelimeler)  Esc vazgeç",
            "Copies: 1 full  2 rehearsal (lines left blank)  3 rehearsal, light (first words)  Esc cancel",
        )
        .to_string();
    }
    match p.tab {
        Tab::Casting => lang::l(
            "⏎ oyuncu  ←→ dağıtım  n yeni dağıtım  r adı  x sil  k oyuncu kopyaları  o aç  z ezber  Tab tablo  Esc",
            "⏎ actor  ←→ casting  n new casting  r name  x delete  k actor copies  o open  z memorize  Tab table  Esc",
        )
        .to_string(),
        Tab::Table => lang::l(
            "● konuşur  ○ sahnede   ↑↓ sahne  ←→ kaydır  ⏎ sahneye git  x CSV  p PDF  o aç  Tab sayılar  Esc",
            "● speaks  ○ on stage   ↑↓ scene  ←→ scroll  ⏎ go there  x CSV  p PDF  o open  Tab numbers  Esc",
        )
        .to_string(),
        Tab::Stats => lang::f(
            "↑↓ kaydır  w dakikada kelime ({})  Tab dağıtım  Esc",
            "↑↓ scroll  w words a minute ({})  Tab casting  Esc",
            &[&wpm],
        ),
    }
}

fn name_w(doc: &Doc) -> usize {
    doc.cast
        .characters
        .iter()
        .map(|c| c.script.width())
        .max()
        .unwrap_or(0)
        .clamp(6, 24)
}

fn draw_casting(f: &mut Frame, area: Rect, doc: &Doc, side: &Sidecar, data: &Data, p: &RolesPanel) {
    let nw = name_w(doc);
    let n = doc.cast.characters.len();
    let h = area.height as usize;
    let top = p.sel.saturating_sub(h.saturating_sub(1));
    let mut lines = Vec::new();
    for i in (0..n).skip(top).take(h) {
        let c = &doc.cast.characters[i];
        let r = &data.roles[i];
        let sel = i == p.sel;
        let numbers = lang::f(
            "{} replik · {} kelime · ≈ {} · {} sahne",
            "{} lines · {} words · ≈ {} · {} scenes",
            &[&r.lines, &r.words, &timing::format(r.secs), &r.scenes.len()],
        );
        let left = 2 + nw + 2;
        let room = (area.width as usize).saturating_sub(left + numbers.width() + 2);
        let actor = match side.actor(&c.alias) {
            Some(a) => Span::styled(fit(a, room), Style::default().fg(FG)),
            None => Span::styled(fit(lang::l("—", "—"), room), Style::default().fg(FAINT)),
        };
        let aw = actor.content.width();
        lines.push(row(
            vec![
                mark(sel),
                Span::styled(
                    format!("{:<nw$}", fit(&c.script, nw)),
                    Style::default().fg(BEIGE).add_modifier(Modifier::BOLD),
                ),
                Span::raw("  "),
                actor,
                Span::raw(" ".repeat(room.saturating_sub(aw) + 2)),
                Span::styled(numbers, Style::default().fg(MUTED)),
            ],
            sel,
            area.width,
        ));
    }
    f.render_widget(Paragraph::new(lines), area);
}

/// A role's column: as wide as its name, within limits.
fn col_w(doc: &Doc, i: usize) -> usize {
    doc.cast.characters[i].script.width().clamp(3, 10) + 2
}

fn draw_table(f: &mut Frame, area: Rect, doc: &Doc, side: &Sidecar, data: &Data, p: &RolesPanel) {
    let n = doc.cast.characters.len();
    let scene_w = 22.min(area.width as usize / 3);
    let mut cols = Vec::new();
    let mut used = scene_w;
    for i in p.col.min(n.saturating_sub(1))..n {
        let w = col_w(doc, i);
        if used + w > area.width as usize {
            break;
        }
        used += w;
        cols.push(i);
    }
    let center = |s: &str, w: usize| {
        let s = fit(s, w.saturating_sub(1));
        let pad = w.saturating_sub(s.width());
        format!("{}{}{}", " ".repeat(pad / 2), s, " ".repeat(pad - pad / 2))
    };
    let mut lines = Vec::new();
    let mut names = vec![Span::raw(" ".repeat(scene_w))];
    let mut actors = vec![Span::raw(" ".repeat(scene_w))];
    for &i in &cols {
        let w = col_w(doc, i);
        names.push(Span::styled(
            center(&doc.cast.characters[i].script, w),
            Style::default().fg(BEIGE).add_modifier(Modifier::BOLD),
        ));
        let actor = side.actor(&doc.cast.characters[i].alias).unwrap_or("");
        actors.push(Span::styled(center(actor, w), Style::default().fg(MUTED)));
    }
    lines.push(Line::from(names));
    lines.push(Line::from(actors));
    let h = (area.height as usize).saturating_sub(2);
    let top = p.row.saturating_sub(h.saturating_sub(1));
    for s in (0..data.table.len()).skip(top).take(h) {
        let sel = s == p.row;
        let scene = &doc.outline.scenes[s];
        let label = format!("{} {}", casting::scene_name(doc, s), scene.place);
        let mut spans = vec![
            mark(sel),
            Span::styled(
                format!("{:<w$}", fit(&label, scene_w - 3), w = scene_w - 2),
                Style::default().fg(FG),
            ),
        ];
        for &i in &cols {
            let (c, style) = match data.table[s][i] {
                Presence::Speaks => ("●", Style::default().fg(CLARET)),
                Presence::On => ("○", Style::default().fg(MUTED)),
                Presence::Absent => ("·", Style::default().fg(FAINT)),
            };
            spans.push(Span::styled(center(c, col_w(doc, i)), style));
        }
        lines.push(row(spans, sel, area.width));
    }
    if cols.last().is_some_and(|&l| l + 1 < n) || p.col > 0 {
        let more = lang::f("←→ {} rol daha", "←→ {} more roles", &[&(n - cols.len())]);
        lines.push(Line::from(Span::styled(more, Style::default().fg(FAINT))));
    }
    f.render_widget(Paragraph::new(lines), area);
}

/// The numbers as lines, to scroll through.
pub fn stats_lines(doc: &Doc, data: &Data, wpm: u32, width: u16) -> Vec<Line<'static>> {
    let nw = name_w(doc);
    let head = Style::default().fg(CLARET).add_modifier(Modifier::BOLD);
    let muted = Style::default().fg(MUTED);
    let total: usize = data.roles.iter().map(|r| r.words).sum();
    let secs: u32 = doc
        .outline
        .scenes
        .iter()
        .map(|s| timing::scene_secs(s, wpm))
        .sum();
    let mut out = vec![Line::from(vec![
        Span::styled(lang::l("Oyun", "The play"), head),
        Span::styled(
            lang::f(
                "   ≈ {} · {} kelime konuşulur · {} · {} sahne",
                "   ≈ {} · {} words spoken · {} · {} scenes",
                &[
                    &timing::format(secs),
                    &total,
                    &dactile::outline::acts_text(doc.outline.acts.len()),
                    &doc.outline.scenes.len(),
                ],
            ),
            muted,
        ),
    ])];
    out.push(Line::raw(""));
    out.push(Line::from(Span::styled(lang::l("Roller", "Roles"), head)));
    let mut idx: Vec<usize> = (0..data.roles.len()).collect();
    idx.sort_by_key(|&i| std::cmp::Reverse(data.roles[i].words));
    let bar_w = (width as usize).saturating_sub(nw + 52).min(30);
    let most = idx.first().map_or(1, |&i| data.roles[i].words.max(1));
    for i in idx {
        let r = &data.roles[i];
        let share = (r.words * 100).checked_div(total).unwrap_or(0);
        let bar = "▇".repeat(r.words * bar_w / most);
        out.push(Line::from(vec![
            Span::styled(
                format!("  {:<nw$}", fit(&doc.cast.characters[i].script, nw)),
                Style::default().fg(BEIGE),
            ),
            Span::styled(
                format!(
                    "{:>5} {:<7}{:>6} {:<8}{:>9}  {:<4} ",
                    r.lines,
                    lang::l("replik", "lines"),
                    r.words,
                    lang::l("kelime", "words"),
                    timing::format(r.secs),
                    if lang::is_tr() {
                        format!("%{share}")
                    } else {
                        format!("{share}%")
                    }
                ),
                muted,
            ),
            Span::styled(bar, Style::default().fg(CLARET)),
        ]));
    }
    out.push(Line::raw(""));
    out.push(Line::from(Span::styled(
        lang::l("Sahneler", "Scenes"),
        head,
    )));
    let mut act = None;
    for (s, scene) in doc.outline.scenes.iter().enumerate() {
        if scene.act != act {
            act = scene.act;
            if let Some(a) = act {
                let secs: u32 = doc
                    .outline
                    .scenes
                    .iter()
                    .filter(|x| x.act == act)
                    .map(|x| timing::scene_secs(x, wpm))
                    .sum();
                let a = &doc.outline.acts[a];
                out.push(Line::from(vec![
                    Span::styled(
                        format!(
                            "  {}. {}",
                            dactile::outline::roman(a.number),
                            lang::l("Perde", "Act")
                        ),
                        Style::default().fg(FG).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(format!("  {}  ≈ {}", a.title, timing::format(secs)), muted),
                ]));
            }
        }
        out.push(Line::from(vec![
            Span::styled(
                format!(
                    "    {:<7}{:<24}",
                    casting::scene_name(doc, s),
                    fit(&scene.place, 23)
                ),
                Style::default().fg(FG),
            ),
            Span::styled(
                lang::f(
                    "{}  {} replik",
                    "{}  {} lines",
                    &[
                        &format!("{:>8}", timing::format(timing::scene_secs(scene, wpm))),
                        &scene.lines,
                    ],
                ),
                muted,
            ),
        ]));
    }
    out
}

fn draw_stats(f: &mut Frame, area: Rect, doc: &Doc, data: &Data, wpm: u32, p: &RolesPanel) {
    let lines = stats_lines(doc, data, wpm, area.width);
    let top = p
        .scroll
        .min(lines.len().saturating_sub(area.height as usize));
    f.render_widget(
        Paragraph::new(lines.into_iter().skip(top).collect::<Vec<_>>()),
        area,
    );
}
