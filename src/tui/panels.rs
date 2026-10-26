//! Full-screen panels over the text: the cast, the scenes, help.

use super::popup::{fit, float, inner, mark, panel, row};
use super::theme::{self, BEIGE, CLARET, FAINT, FG, MUTED, WARN};
use dactile::cast::{Kind as CastKind, Row};
use dactile::lang;
use dactile::lex::{self, Kind};
use dactile::render::Doc;
use dactile::shelf::{Trashed, Unit, UnitKind};
use dactile::sidecar::Sidecar;
use dactile::timing;
use ratatui::Frame;
use ratatui::layout::{Position, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use unicode_width::UnicodeWidthStr;

/// A panel `w`×`h` (border included) in the middle of `area`, a little high.
fn centered(area: Rect, w: u16, h: u16) -> Rect {
    let w = w.min(area.width.saturating_sub(4)).max(30.min(area.width));
    let h = h.min(area.height.saturating_sub(2)).max(6.min(area.height));
    Rect {
        x: area.x + (area.width - w) / 2,
        y: area.y + (area.height - h) / 3,
        width: w,
        height: h,
    }
}

/// The keys at the bottom of a panel.
fn keys_line(f: &mut Frame, inn: Rect, keys: &str) {
    f.render_widget(
        Paragraph::new(fit(keys, inn.width as usize)).style(theme::base().fg(FAINT)),
        Rect {
            y: inn.y + inn.height.saturating_sub(1),
            height: 1,
            ..inn
        },
    );
}

// ------------------------------------------------------------------ cast

/// A line of the cast panel.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Entry {
    Character(usize),
    /// A group's heading; its members follow.
    Group(usize),
}

pub struct CastPanel {
    pub sel: usize,
    /// Sorted by how much each one speaks instead of the play's order.
    pub by_lines: bool,
}

/// Lines and spoken words of each character, by index into the cast.
pub fn stats(doc: &Doc, text: &str) -> Vec<(usize, usize)> {
    let mut out = vec![(0, 0); doc.cast.characters.len()];
    for b in &doc.blocks[doc.body.clone()] {
        if let Kind::Line { alias, text: t } = &b.kind
            && let Some(i) = doc.cast.index(alias)
        {
            out[i].0 += 1;
            out[i].1 += lex::spoken_words(text, t.clone(), &doc.cast);
        }
    }
    out
}

/// The panel's lines, in the play's order or by lines spoken.
pub fn entries(doc: &Doc, stats: &[(usize, usize)], by_lines: bool) -> Vec<Entry> {
    if by_lines {
        let mut idx: Vec<usize> = (0..doc.cast.characters.len()).collect();
        idx.sort_by_key(|&i| std::cmp::Reverse(stats[i].0));
        return idx.into_iter().map(Entry::Character).collect();
    }
    let mut out = Vec::new();
    for r in doc.cast.rows() {
        match r {
            Row::Character(i) => out.push(Entry::Character(i)),
            Row::Group(g) => {
                out.push(Entry::Group(g));
                out.extend(
                    doc.cast.groups[g]
                        .members
                        .iter()
                        .map(|&m| Entry::Character(m)),
                );
            }
        }
    }
    for (i, c) in doc.cast.characters.iter().enumerate() {
        if matches!(c.kind, CastKind::Plain | CastKind::Hidden) {
            out.push(Entry::Character(i));
        }
    }
    out
}

pub fn draw_cast(f: &mut Frame, area: Rect, doc: &Doc, text: &str, side: &Sidecar, p: &CastPanel) {
    let st = stats(doc, text);
    let list = entries(doc, &st, p.by_lines);
    let alias_head = lang::l("kısaltma", "alias");
    let alias_w = doc
        .cast
        .characters
        .iter()
        .map(|c| c.alias.width() + 1)
        .max()
        .unwrap_or(4)
        .max(alias_head.width())
        + 2;
    let name_w = doc
        .cast
        .characters
        .iter()
        .map(|c| c.script.width())
        .max()
        .unwrap_or(4)
        .max(4)
        + 2;
    // mark, group bar, star, \alias, name, lines, words, card
    let fixed = 2 + 2 + 2 + alias_w + name_w + 8 + 8 + 2 + 2;
    let desc_w = doc
        .cast
        .characters
        .iter()
        .map(|c| c.desc.as_deref().unwrap_or("").width())
        .max()
        .unwrap_or(0)
        .max(12);
    let w = (fixed + desc_w + 6).max(78) as u16;
    // Header, list, a blank, the chosen one's details, keys.
    let h = list.len() as u16 + 6;
    let r = centered(area, w.min(110), h);
    float(f, r);
    let order = if p.by_lines {
        lang::l("en çok konuşan", "most lines")
    } else {
        lang::l("oyundaki sıra", "the play's order")
    };
    let count = Line::from(Span::styled(
        lang::f(
            " {} kişi ",
            " {} characters ",
            &[&doc.cast.characters.len()],
        ),
        Style::default().fg(FAINT),
    ))
    .right_aligned();
    f.render_widget(
        panel(&lang::f("kişiler — {}", "cast — {}", &[&order])).title_bottom(count),
        r,
    );
    let inn = inner(r);
    let room = (inn.width as usize).saturating_sub(fixed);
    let head = Style::default().fg(FAINT);
    let mut lines: Vec<Line> = vec![Line::from(vec![
        Span::styled(format!("      {alias_head:<alias_w$}"), head),
        Span::styled(format!("{:<name_w$}", lang::l("ad", "name")), head),
        Span::styled(
            format!(
                "{:>8}{:>8}  ",
                lang::l("replik", "lines"),
                lang::l("kelime", "words")
            ),
            head,
        ),
        Span::styled(lang::l("açıklama", "description").to_string(), head),
    ])];
    let rows = (inn.height as usize).saturating_sub(4);
    let start = p.sel.saturating_sub(rows.saturating_sub(1));
    for (n, e) in list.iter().enumerate().skip(start).take(rows) {
        let sel = n == p.sel;
        match *e {
            Entry::Group(g) => {
                let grp = &doc.cast.groups[g];
                let name = if grp.desc.is_empty() {
                    lang::l("grup", "group")
                } else {
                    &grp.desc
                };
                lines.push(row(
                    vec![
                        mark(sel),
                        Span::styled(
                            format!("┌ {name}"),
                            Style::default().fg(MUTED).add_modifier(Modifier::ITALIC),
                        ),
                    ],
                    sel,
                    inn.width,
                ));
            }
            Entry::Character(i) => {
                let c = &doc.cast.characters[i];
                let bar = if c.group.is_some() && !p.by_lines {
                    "│ "
                } else {
                    "  "
                };
                let star = if c.star { "★ " } else { "  " };
                let kind = match c.kind {
                    CastKind::Hidden => lang::l("gizli", "hidden"),
                    CastKind::Plain => lang::l("tabloda yok", "not in the table"),
                    _ => "",
                };
                let name_style = Style::default()
                    .fg(if c.star { BEIGE } else { FG })
                    .add_modifier(if sel {
                        Modifier::BOLD
                    } else {
                        Modifier::empty()
                    });
                let card = if side.cards.get(&c.alias).is_some_and(|k| !k.is_empty()) {
                    " ▤"
                } else {
                    "  "
                };
                let (lines_n, words) = st[i];
                let desc = c.desc.as_deref().filter(|d| !d.is_empty()).unwrap_or(kind);
                lines.push(row(
                    vec![
                        mark(sel),
                        Span::styled(bar, Style::default().fg(FAINT)),
                        Span::styled(star, Style::default().fg(BEIGE)),
                        Span::styled(
                            format!("{:<alias_w$}", format!("\\{}", c.alias)),
                            Style::default().fg(MUTED),
                        ),
                        Span::styled(format!("{:<name_w$}", c.script), name_style),
                        Span::styled(
                            format!("{lines_n:>8}{words:>8}  "),
                            Style::default().fg(MUTED),
                        ),
                        Span::styled(
                            format!("{:<room$}", fit(desc, room)),
                            Style::default().fg(FAINT),
                        ),
                        Span::styled(card, Style::default().fg(BEIGE)),
                    ],
                    sel,
                    inn.width,
                ));
            }
        }
    }
    f.render_widget(
        Paragraph::new(lines).style(theme::base()),
        Rect {
            height: inn.height.saturating_sub(3),
            ..inn
        },
    );
    // The chosen one in full: the table cuts long descriptions.
    if let Some(Entry::Character(i)) = list.get(p.sel) {
        let c = &doc.cast.characters[*i];
        let mut parts = vec![c.display_name().to_string()];
        if let Some(d) = c.desc.as_deref().filter(|d| !d.is_empty()) {
            parts.push(d.to_string());
        }
        if let Some(k) = side.cards.get(&c.alias).filter(|k| !k.is_empty()) {
            if !k.age.is_empty() {
                parts.push(lang::f("{} yaş", "age {}", &[&k.age]));
            }
            if !k.relations.is_empty() {
                parts.push(k.relations.clone());
            }
        }
        f.render_widget(
            Paragraph::new(fit(&parts.join(" · "), inn.width as usize))
                .style(theme::base().fg(MUTED)),
            Rect {
                y: inn.y + inn.height.saturating_sub(2),
                height: 1,
                ..inn
            },
        );
    }
    let keys = if p.by_lines {
        lang::l(
            "⏎ kart   e düzenle   n yeni   m birleştir   b böl   d sil   s sıra   Esc kapat",
            "⏎ card   e edit   n new   m merge   b split   d delete   s order   Esc close",
        )
    } else {
        lang::l(
            "⏎ kart  e düzenle  n yeni  m birleştir  b böl  d sil  Alt+↑↓ taşı  s sıra  Esc",
            "⏎ card  e edit  n new  m merge  b split  d delete  Alt+↑↓ move  s order  Esc",
        )
    };
    keys_line(f, inn, keys);
}

// ------------------------------------------------------------------ scenes

/// ^G: the acts and scenes, to go to, hide or throw away; Tab shows the
/// trash.
#[derive(Debug, Clone, Default)]
pub struct ScenesPanel {
    pub sel: usize,
    /// Showing the trash instead of the play.
    pub trash: bool,
    pub trash_sel: usize,
    /// x was pressed once in the trash: once more destroys.
    pub confirm: bool,
}

#[allow(clippy::too_many_arguments)]
pub fn draw_scenes(
    f: &mut Frame,
    area: Rect,
    doc: &Doc,
    units: &[Unit],
    marks: &std::collections::BTreeSet<usize>,
    trash: &[Trashed],
    wpm: u32,
    p: &ScenesPanel,
) {
    let rows_wanted = if p.trash { trash.len() } else { units.len() };
    let h = (rows_wanted + 6).max(8) as u16;
    let r = centered(area, 96, h);
    float(f, r);
    let inn = inner(r);
    if p.trash {
        f.render_widget(
            panel(&lang::f("çöp kutusu — {}", "trash — {}", &[&trash.len()])),
            r,
        );
        let mut lines: Vec<Line> = Vec::new();
        if trash.is_empty() {
            lines.push(Line::from(Span::styled(
                lang::l("Çöp kutusu boş.", "The trash is empty."),
                Style::default().fg(MUTED),
            )));
        }
        let rows = inn.height.saturating_sub(3) as usize;
        let start = p.trash_sel.saturating_sub(rows.saturating_sub(1));
        for (n, t) in trash.iter().enumerate().skip(start).take(rows) {
            let sel = n == p.trash_sel;
            let room = (inn.width as usize).saturating_sub(t.when.width() + 6);
            let what = fit(&t.what, room);
            let gap = room.saturating_sub(what.width()) + 2;
            lines.push(row(
                vec![
                    mark(sel),
                    Span::styled(what, Style::default().fg(if sel { BEIGE } else { FG })),
                    Span::raw(" ".repeat(gap)),
                    Span::styled(t.when.clone(), Style::default().fg(FAINT)),
                ],
                sel,
                inn.width,
            ));
        }
        f.render_widget(
            Paragraph::new(lines).style(theme::base()),
            Rect {
                height: inn.height.saturating_sub(2),
                ..inn
            },
        );
        let keys = if p.confirm {
            lang::l(
                "x'e bir daha bas: kalıcı olarak silinir (geçmişte bir kopyası kalır)",
                "press x again: it is destroyed (a copy stays in the history)",
            )
        } else {
            lang::l(
                "r geri koy   x yok et   Tab sahneler   Esc kapat",
                "r put back   x destroy   Tab scenes   Esc close",
            )
        };
        f.render_widget(
            Paragraph::new(fit(keys, inn.width as usize)).style(theme::base().fg(if p.confirm {
                WARN
            } else {
                FAINT
            })),
            Rect {
                y: inn.y + inn.height.saturating_sub(1),
                height: 1,
                ..inn
            },
        );
        return;
    }
    let total: u32 = doc
        .outline
        .scenes
        .iter()
        .map(|s| timing::scene_secs(s, wpm))
        .sum();
    f.render_widget(
        panel(&lang::f(
            "sahneler — toplam ≈ {}",
            "scenes — total ≈ {}",
            &[&timing::format(total)],
        )),
        r,
    );
    let rows = inn.height.saturating_sub(3) as usize;
    let start = p
        .sel
        .saturating_sub(rows / 2)
        .min(units.len().saturating_sub(rows));
    let mut lines: Vec<Line> = Vec::new();
    if units.is_empty() {
        lines.push(Line::from(Span::styled(
            lang::l(
                "Henüz sahne yok: #sahne ile başla.",
                "No scenes yet: start one with #sahne.",
            ),
            Style::default().fg(MUTED),
        )));
    }
    for (n, u) in units.iter().enumerate().skip(start).take(rows) {
        let sel = n == p.sel;
        let marked = marks.contains(&u.span.start);
        let scene = (!u.hidden && u.kind == UnitKind::Scene)
            .then(|| doc.scene_of(doc.block_at(u.span.start)?))
            .flatten()
            .map(|s| &doc.outline.scenes[s]);
        let stats = match (scene, u.hidden) {
            (_, true) => lang::f("gizli · {} replik", "hidden · {} lines", &[&u.lines]),
            (Some(s), _) => lang::f(
                "{} replik · {} kelime · ≈ {}",
                "{} lines · {} words · ≈ {}",
                &[
                    &s.lines,
                    &s.words,
                    &timing::format(timing::scene_secs(s, wpm)),
                ],
            ),
            (None, _) => String::new(),
        };
        let indent = if u.kind == UnitKind::Scene { "  " } else { "" };
        let room = (inn.width as usize).saturating_sub(stats.width() + 8);
        let name = fit(&format!("{indent}{}", u.name()), room);
        let gap = room.saturating_sub(name.width()) + 2;
        let color = if u.hidden {
            FAINT
        } else if u.kind == UnitKind::Act {
            CLARET
        } else if sel {
            BEIGE
        } else {
            FG
        };
        let mut style = Style::default().fg(color);
        if u.kind == UnitKind::Act || sel {
            style = style.add_modifier(Modifier::BOLD);
        }
        if u.hidden {
            style = style.add_modifier(Modifier::ITALIC);
        }
        lines.push(row(
            vec![
                mark(sel),
                Span::styled(
                    if marked { "■ " } else { "□ " },
                    Style::default().fg(if marked { WARN } else { FAINT }),
                ),
                Span::styled(name, style),
                Span::raw(" ".repeat(gap)),
                Span::styled(
                    stats,
                    Style::default().fg(if u.hidden { FAINT } else { MUTED }),
                ),
            ],
            sel,
            inn.width,
        ));
    }
    f.render_widget(
        Paragraph::new(lines).style(theme::base()),
        Rect {
            height: inn.height.saturating_sub(2),
            ..inn
        },
    );
    let n = marks.len();
    let keys = if n > 0 {
        lang::f(
            "{} işaretli: h gizle/göster   t çöpe at   ␣ işaret   Esc kapat",
            "{} marked: h hide/show   t to the trash   ␣ mark   Esc close",
            &[&n],
        )
    } else {
        lang::f(
            "⏎ git   ␣ işaretle   h gizle/göster   t çöpe at   Tab çöp kutusu ({})   Esc kapat",
            "⏎ go   ␣ mark   h hide/show   t to the trash   Tab trash ({})   Esc close",
            &[&trash.len()],
        )
    };
    keys_line(f, inn, &keys);
}

/// Splitting a character: its scenes, the picked ones going to the new one.
#[allow(clippy::too_many_arguments)]
pub fn draw_split(
    f: &mut Frame,
    area: Rect,
    doc: &Doc,
    from: &str,
    new: &str,
    scenes: &[usize],
    picked: &std::collections::BTreeSet<usize>,
    sel: usize,
) {
    let r = centered(area, 90, scenes.len() as u16 + 6);
    float(f, r);
    let old = doc
        .cast
        .get(from)
        .map_or(from.to_string(), |c| c.script.clone());
    f.render_widget(
        panel(&lang::f(
            "{} → {}: hangi sahneler?",
            "{} → {}: which scenes?",
            &[&old, &new],
        )),
        r,
    );
    let inn = inner(r);
    let mut lines: Vec<Line> = Vec::new();
    if scenes.is_empty() {
        lines.push(Line::from(Span::styled(
            lang::l(
                "Bu karakter hiçbir sahnede konuşmuyor.",
                "This character speaks in no scene.",
            ),
            Style::default().fg(MUTED),
        )));
    }
    let rows = inn.height.saturating_sub(3) as usize;
    let start = sel.saturating_sub(rows.saturating_sub(1));
    for (n, &i) in scenes.iter().enumerate().skip(start).take(rows) {
        let sc = &doc.outline.scenes[i];
        let is_sel = n == sel;
        let on = picked.contains(&i);
        let act = sc
            .act
            .map(|a| format!("{}. ", dactile::outline::roman(doc.outline.acts[a].number)))
            .unwrap_or_default();
        let mut name = format!("{act}{} {}", lang::l("Sahne", "Scene"), sc.label);
        if !sc.place.trim().is_empty() {
            name.push_str(&format!(" · {}", sc.place.trim()));
        }
        let base = doc.body.start;
        let said = doc.blocks[sc.block + base..sc.end + base]
            .iter()
            .filter(|b| matches!(&b.kind, Kind::Line { alias, .. } if alias == from))
            .count();
        let count = lang::f("{} replik", "{} lines", &[&said]);
        lines.push(row(
            vec![
                mark(is_sel),
                Span::styled(
                    if on { "■ " } else { "□ " },
                    Style::default().fg(if on { WARN } else { FAINT }),
                ),
                Span::styled(
                    {
                        let w = (inn.width as usize).saturating_sub(18);
                        format!("{:<w$}", fit(&name, w))
                    },
                    Style::default().fg(if is_sel { BEIGE } else { FG }),
                ),
                Span::styled(count, Style::default().fg(MUTED)),
            ],
            is_sel,
            inn.width,
        ));
    }
    f.render_widget(
        Paragraph::new(lines).style(theme::base()),
        Rect {
            height: inn.height.saturating_sub(2),
            ..inn
        },
    );
    keys_line(
        f,
        inn,
        &lang::f(
            "␣ seç   ⏎ seçilen {} sahne → {}   Esc vazgeç",
            "␣ pick   ⏎ give the {} picked scenes to {}   Esc cancel",
            &[&picked.len(), &new],
        ),
    );
}

// ------------------------------------------------------------------ spelling

pub fn draw_suggest(
    f: &mut Frame,
    area: Rect,
    at: Position,
    word: &str,
    items: &[String],
    sel: usize,
) {
    let rows = items.len().min(9) as u16;
    let w = items
        .iter()
        .map(|s| s.chars().count() + 4)
        .max()
        .unwrap_or(10)
        .clamp(20, 50) as u16;
    let x = at.x.min(area.x + area.width.saturating_sub(w));
    let y = if at.y + 1 + rows + 2 <= area.y + area.height {
        at.y + 1
    } else {
        at.y.saturating_sub(rows + 2)
    };
    let r = Rect {
        x,
        y,
        width: w,
        height: rows + 2,
    };
    float(f, r);
    f.render_widget(panel(word), r);
    let lines: Vec<Line> = items
        .iter()
        .take(9)
        .enumerate()
        .map(|(i, s)| {
            let is_sel = i == sel;
            let add = s.starts_with('+');
            row(
                vec![
                    mark(is_sel),
                    Span::styled(
                        s.clone(),
                        Style::default().fg(if add {
                            WARN
                        } else if is_sel {
                            BEIGE
                        } else {
                            FG
                        }),
                    ),
                ],
                is_sel,
                inner(r).width,
            )
        })
        .collect();
    f.render_widget(Paragraph::new(lines).style(theme::base()), inner(r));
}
