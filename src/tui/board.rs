//! The board: the play's acts and scenes as a list with their summaries or
//! as cards in columns, full screen; and the strip beside the writing with
//! the cursor's scene open.

use super::panels::ScenesPanel;
use super::popup::{fit, mark, row};
use super::theme::{self, BEIGE, CLARET, FAINT, FG, MUTED, SELECTION, WARN};
use dactile::lang;
use dactile::render::Doc;
use dactile::shelf::{Unit, UnitKind};
use dactile::{structure, timing};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};
use std::collections::{BTreeMap, BTreeSet};
use unicode_width::UnicodeWidthStr;

/// The strip's width, its border included.
pub const STRIP_W: u16 = 34;

/// An act or a scene as the board shows it.
pub struct Item {
    pub kind: UnitKind,
    pub hidden: bool,
    pub start: usize,
    /// "Sahne 2", "I. Perde".
    pub label: String,
    /// The place or the act's title.
    pub title: String,
    pub secs: Option<u32>,
    pub lines: usize,
    /// Who is in it, as the script writes them.
    pub cast: Vec<String>,
    pub summary: Option<String>,
}

impl Item {
    pub fn name(&self) -> String {
        if self.title.trim().is_empty() {
            self.label.clone()
        } else {
            format!("{} · {}", self.label, self.title.trim())
        }
    }

    fn stats(&self) -> String {
        match (self.hidden, self.secs) {
            (true, _) => lang::f("gizli · {} replik", "hidden · {} lines", &[&self.lines]),
            (false, Some(s)) => lang::f(
                "{} replik · ≈ {}",
                "{} lines · ≈ {}",
                &[&self.lines, &timing::format(s)],
            ),
            (false, None) => String::new(),
        }
    }
}

pub fn items(
    text: &str,
    doc: &Doc,
    units: &[Unit],
    summaries: &BTreeMap<String, String>,
    wpm: u32,
) -> Vec<Item> {
    units
        .iter()
        .map(|u| {
            let scene = (!u.hidden && u.kind == UnitKind::Scene)
                .then(|| doc.scene_of(doc.block_at(u.span.start)?))
                .flatten()
                .map(|s| &doc.outline.scenes[s]);
            let summary = (u.kind == UnitKind::Scene)
                .then(|| structure::id_in(&text[u.span.clone()]))
                .flatten()
                .and_then(|id| summaries.get(id))
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());
            Item {
                kind: u.kind,
                hidden: u.hidden,
                start: u.span.start,
                label: u.label.clone(),
                title: u.title.clone(),
                secs: scene.map(|s| timing::scene_secs(s, wpm)),
                lines: scene.map_or(u.lines, |s| s.lines),
                cast: scene.map_or_else(Vec::new, |s| {
                    s.cast
                        .iter()
                        .map(|a| doc.cast.get(a).map_or(a.clone(), |c| c.script.clone()))
                        .collect()
                }),
                summary,
            }
        })
        .collect()
}

/// `s` in lines of at most `w` columns, broken between words.
pub fn wrap(s: &str, w: usize) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut line = String::new();
    for word in s.split_whitespace() {
        if !line.is_empty() && line.width() + 1 + word.width() > w {
            out.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() {
        out.push(line);
    }
    out.into_iter().map(|l| fit(&l, w)).collect()
}

/// Acts with their scenes: (the act's index, if any; its scenes' indexes).
fn groups(items: &[Item]) -> Vec<(Option<usize>, Vec<usize>)> {
    let mut out: Vec<(Option<usize>, Vec<usize>)> = Vec::new();
    for (i, it) in items.iter().enumerate() {
        match it.kind {
            UnitKind::Act => out.push((Some(i), Vec::new())),
            UnitKind::Scene => match out.last_mut() {
                Some(g) => g.1.push(i),
                None => out.push((None, vec![i])),
            },
        }
    }
    out
}

/// On the cards, the unit beside `sel` in the column to its left or right:
/// as far down, or the column's act.
pub fn neighbour(kinds: &[UnitKind], sel: usize, right: bool) -> usize {
    let mut gs: Vec<(Option<usize>, Vec<usize>)> = Vec::new();
    for (i, k) in kinds.iter().enumerate() {
        match k {
            UnitKind::Act => gs.push((Some(i), Vec::new())),
            UnitKind::Scene => match gs.last_mut() {
                Some(g) => g.1.push(i),
                None => gs.push((None, vec![i])),
            },
        }
    }
    let Some(g) = gs
        .iter()
        .position(|(a, s)| *a == Some(sel) || s.contains(&sel))
    else {
        return sel;
    };
    let to = if right { g + 1 } else { g.wrapping_sub(1) };
    let Some((act, scenes)) = gs.get(to) else {
        return sel;
    };
    let pos = gs[g].1.iter().position(|&s| s == sel);
    match pos {
        Some(p) if !scenes.is_empty() => scenes[p.min(scenes.len() - 1)],
        _ => act.or(scenes.first().copied()).unwrap_or(sel),
    }
}

/// The group a place (from `structure::places`) falls in, and before which
/// of its scenes (their count: at its end).
fn place_in(groups: &[(Option<usize>, Vec<usize>)], n: usize, u: usize) -> (usize, usize) {
    let end_of = |g: usize| (g, groups.get(g).map_or(0, |x| x.1.len()));
    if u >= n {
        return end_of(groups.len().saturating_sub(1));
    }
    for (g, (act, scenes)) in groups.iter().enumerate() {
        if let Some(p) = scenes.iter().position(|&s| s == u) {
            return (g, p);
        }
        // Before an act: at the end of the one before it.
        if *act == Some(u) {
            return if g == 0 { (0, 0) } else { end_of(g - 1) };
        }
    }
    (0, 0)
}

/// Where unit `moving` would go at place `u`, in words.
pub fn place_name(items: &[Item], moving: usize, u: usize) -> String {
    let Some(it) = items.get(u) else {
        return lang::l("en sona", "to the end").into();
    };
    let scene = items.get(moving).is_some_and(|m| m.kind == UnitKind::Scene);
    match items[..u].iter().rposition(|x| x.kind == UnitKind::Act) {
        Some(a) if scene && it.kind == UnitKind::Act => {
            lang::f("{} sonuna", "to the end of {}", &[&items[a].label])
        }
        _ => lang::f("{} önüne", "before {}", &[&it.name()]),
    }
}

fn keys_text(p: &ScenesPanel, items: &[Item], marks: usize, trash: usize) -> String {
    if let Some(to) = p.grab {
        let what = items.get(p.sel).map(Item::name).unwrap_or_default();
        return lang::f(
            "{} → {}   ↑↓ yer seç   ⏎ bırak   Esc vazgeç",
            "{} → {}   ↑↓ choose   ⏎ put it there   Esc cancel",
            &[&what, &place_name(items, p.sel, to)],
        );
    }
    if marks > 0 {
        return lang::f(
            "{} işaretli: h gizle/göster   t çöpe at   m taşı   ␣ işaret   Esc kapat",
            "{} marked: h hide/show   t to the trash   m move   ␣ mark   Esc close",
            &[&marks],
        );
    }
    let view = if p.cards {
        lang::l("Tab liste", "Tab list")
    } else {
        lang::l("Tab kartlar", "Tab cards")
    };
    lang::f(
        "⏎ git  e özet  n yeni kart  Alt+↑↓ taşı  m tut  j birleştir  ␣ işaret  h gizle  t çöpe  c çöp ({})  {}  Esc",
        "⏎ go  e summary  n new card  Alt+↑↓ move  m pick up  j join  ␣ mark  h hide  t trash  c trash ({})  {}  Esc",
        &[&trash, &view],
    )
}

/// The board, full screen.
#[allow(clippy::too_many_arguments)]
pub fn draw(
    f: &mut Frame,
    area: Rect,
    doc: &Doc,
    items: &[Item],
    marks: &BTreeSet<usize>,
    trash: usize,
    p: &ScenesPanel,
) {
    f.render_widget(Clear, area);
    f.render_widget(Block::default().style(theme::base()), area);
    let acts = items.iter().filter(|i| i.kind == UnitKind::Act).count();
    let scenes = items.iter().filter(|i| i.kind == UnitKind::Scene).count();
    let total: u32 = items.iter().filter_map(|i| i.secs).sum();
    let _ = doc;
    let head = Line::from(vec![
        Span::styled(
            " pano",
            Style::default().fg(CLARET).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            lang::f(
                "   {} perde · {} sahne · ≈ {}",
                "   {} acts · {} scenes · ≈ {}",
                &[&acts, &scenes, &timing::format(total)],
            ),
            Style::default().fg(MUTED),
        ),
    ]);
    f.render_widget(Paragraph::new(head), Rect { height: 1, ..area });
    let views = if p.cards {
        lang::l("liste · [kartlar] ", "list · [cards] ")
    } else {
        lang::l("[liste] · kartlar ", "[list] · cards ")
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
    if items.is_empty() {
        f.render_widget(
            Paragraph::new(Span::styled(
                lang::l(
                    "Henüz sahne yok: n ile bir kart aç ya da yazıda #sahne.",
                    "No scenes yet: n opens a card, or #sahne in the writing.",
                ),
                Style::default().fg(MUTED),
            )),
            body,
        );
    } else if p.cards {
        draw_cards(f, body, items, marks, p);
    } else {
        draw_list(f, body, items, marks, p);
    }
    let keys = keys_text(p, items, marks.len(), trash);
    f.render_widget(
        Paragraph::new(fit(&keys, area.width.saturating_sub(2) as usize))
            .style(theme::base().fg(if p.grab.is_some() { BEIGE } else { FAINT })),
        Rect {
            x: area.x + 1,
            y: area.y + area.height.saturating_sub(1),
            width: area.width.saturating_sub(2),
            height: 1,
        },
    );
}

fn here_line(w: u16) -> Line<'static> {
    let s = lang::l(" buraya ", " here ");
    let side = (w as usize).saturating_sub(s.width() + 6) / 2;
    Line::from(Span::styled(
        format!("    {}{s}{}", "─".repeat(side), "─".repeat(side)),
        Style::default().fg(CLARET),
    ))
}

fn draw_list(f: &mut Frame, area: Rect, items: &[Item], marks: &BTreeSet<usize>, p: &ScenesPanel) {
    let w = area.width;
    // Each line with the item it belongs to.
    let mut lines: Vec<(Line<'static>, Option<usize>)> = Vec::new();
    let n = items.len();
    for (i, it) in items.iter().enumerate() {
        if p.grab == Some(i) && p.grab != Some(p.sel) {
            lines.push((here_line(w), None));
        }
        let sel = i == p.sel;
        let moving = p.grab.is_some() && sel;
        if it.kind == UnitKind::Act && i > 0 {
            lines.push((Line::raw(""), Some(i)));
        }
        let marked = marks.contains(&it.start);
        let stats = it.stats();
        let indent = if it.kind == UnitKind::Scene { "  " } else { "" };
        let room = (w as usize).saturating_sub(stats.width() + 8);
        let name = fit(&format!("{indent}{}", it.name()), room);
        let gap = room.saturating_sub(name.width()) + 2;
        let color = if moving {
            WARN
        } else if it.hidden {
            FAINT
        } else if it.kind == UnitKind::Act {
            CLARET
        } else if sel {
            BEIGE
        } else {
            FG
        };
        let mut style = Style::default().fg(color);
        if it.kind == UnitKind::Act || sel {
            style = style.add_modifier(Modifier::BOLD);
        }
        if it.hidden {
            style = style.add_modifier(Modifier::ITALIC);
        }
        lines.push((
            row(
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
                        Style::default().fg(if it.hidden { FAINT } else { MUTED }),
                    ),
                ],
                sel,
                w,
            ),
            Some(i),
        ));
        if it.kind == UnitKind::Scene {
            let pad = "        ";
            let room = (w as usize).saturating_sub(pad.len() + 2);
            match &it.summary {
                Some(s) => {
                    for l in wrap(s, room).into_iter().take(3) {
                        lines.push((
                            Line::from(Span::styled(
                                format!("{pad}{l}"),
                                Style::default().fg(MUTED).add_modifier(Modifier::ITALIC),
                            )),
                            Some(i),
                        ));
                    }
                }
                None if sel && p.grab.is_none() => lines.push((
                    Line::from(Span::styled(
                        format!("{pad}{}", lang::l("e: özet yaz", "e: write a summary")),
                        Style::default().fg(FAINT),
                    )),
                    Some(i),
                )),
                None => {}
            }
        }
    }
    if p.grab == Some(n) {
        lines.push((here_line(w), None));
    }
    let rows = area.height as usize;
    let focus = lines
        .iter()
        .position(|(_, o)| *o == Some(p.sel))
        .unwrap_or(0);
    let here = p
        .grab
        .and_then(|_| lines.iter().position(|(_, o)| o.is_none()))
        .unwrap_or(focus);
    let start = here
        .saturating_sub(rows / 2)
        .min(lines.len().saturating_sub(rows));
    let shown: Vec<Line> = lines
        .into_iter()
        .skip(start)
        .take(rows)
        .map(|l| l.0)
        .collect();
    f.render_widget(Paragraph::new(shown).style(theme::base()), area);
}

const CARD_H: u16 = 5;
const CARD_W: u16 = 30;

fn draw_cards(f: &mut Frame, area: Rect, items: &[Item], marks: &BTreeSet<usize>, p: &ScenesPanel) {
    let gs = groups(items);
    let cols = ((area.width + 2) / (CARD_W + 2)).max(1) as usize;
    let cw = ((area.width + 2) / cols as u16).saturating_sub(2).max(16);
    let sel_group = gs
        .iter()
        .position(|(a, s)| *a == Some(p.sel) || s.contains(&p.sel))
        .unwrap_or(0);
    let grab_at = p.grab.map(|u| place_in(&gs, items.len(), u));
    let focus_group = grab_at.map_or(sel_group, |g| g.0);
    let first = (focus_group / cols) * cols;
    // The acts as a strip on top: which are on screen.
    let mut strip: Vec<Span> = Vec::new();
    for (g, (act, _)) in gs.iter().enumerate() {
        let name = act.map_or("·".to_string(), |a| {
            items[a].label.split('.').next().unwrap_or("").to_string()
        });
        let on = (first..first + cols).contains(&g);
        strip.push(Span::styled(
            format!(" {name} "),
            if g == sel_group {
                Style::default().fg(CLARET).add_modifier(Modifier::BOLD)
            } else if on {
                Style::default().fg(BEIGE)
            } else {
                Style::default().fg(FAINT)
            },
        ));
    }
    if gs.len() > cols {
        f.render_widget(
            Paragraph::new(Line::from(strip)),
            Rect { height: 1, ..area },
        );
    }
    let top = area.y + 2;
    let height = area.height.saturating_sub(2);
    for (c, (act, scenes)) in gs.iter().enumerate().skip(first).take(cols) {
        let x = area.x + (c - first) as u16 * (cw + 2);
        let col = Rect {
            x,
            y: top,
            width: cw,
            height,
        };
        // The act's heading.
        let (head, title, act_sel) = match act {
            Some(a) => {
                let it = &items[*a];
                (it.label.to_uppercase(), it.title.clone(), *a == p.sel)
            }
            None => (
                lang::l("PERDEDEN ÖNCE", "BEFORE THE ACTS").into(),
                String::new(),
                false,
            ),
        };
        let mut hs = Style::default().fg(CLARET).add_modifier(Modifier::BOLD);
        if act_sel {
            hs = hs.bg(SELECTION);
        }
        if act.is_some_and(|a| items[a].hidden) {
            hs = hs.fg(FAINT).add_modifier(Modifier::ITALIC);
        }
        if act_sel && p.grab.is_some() {
            hs = hs.fg(WARN);
        }
        let mark_a = act.is_some_and(|a| marks.contains(&items[a].start));
        f.render_widget(
            Paragraph::new(vec![
                Line::from(Span::styled(
                    fit(
                        &format!("{}{head}", if mark_a { "■ " } else { "" }),
                        cw as usize,
                    ),
                    hs,
                )),
                Line::from(Span::styled(
                    fit(title.trim(), cw as usize),
                    Style::default().fg(MUTED),
                )),
            ]),
            Rect { height: 2, ..col },
        );
        let room = height.saturating_sub(3);
        let fits = (room / (CARD_H + 1)).max(1) as usize;
        let here = grab_at.filter(|g| g.0 == c).map(|g| g.1);
        let focus = scenes
            .iter()
            .position(|&s| s == p.sel)
            .or(here)
            .unwrap_or(0);
        let skip = (focus + 1).saturating_sub(fits);
        let mut y = top + 3;
        if skip > 0 {
            f.render_widget(
                Paragraph::new(Span::styled(
                    lang::f("↑ {} sahne daha", "↑ {} more", &[&skip]),
                    Style::default().fg(FAINT),
                )),
                Rect {
                    y: y - 1,
                    height: 1,
                    ..col
                },
            );
        }
        for (k, &i) in scenes.iter().enumerate().skip(skip) {
            if y + CARD_H > top + height {
                let left = scenes.len() - k;
                f.render_widget(
                    Paragraph::new(Span::styled(
                        lang::f("↓ {} sahne daha", "↓ {} more", &[&left]),
                        Style::default().fg(FAINT),
                    )),
                    Rect {
                        y: (top + height).saturating_sub(1).max(y.saturating_sub(1)),
                        height: 1,
                        ..col
                    },
                );
                break;
            }
            if here == Some(k) {
                draw_here(
                    f,
                    Rect {
                        y: y.saturating_sub(1),
                        height: 1,
                        ..col
                    },
                );
            }
            draw_card(
                f,
                Rect {
                    y,
                    height: CARD_H,
                    ..col
                },
                &items[i],
                i == p.sel,
                marks.contains(&items[i].start),
                p.grab.is_some(),
            );
            y += CARD_H + 1;
        }
        if here == Some(scenes.len()) && y <= top + height {
            draw_here(
                f,
                Rect {
                    y: y.saturating_sub(1),
                    height: 1,
                    ..col
                },
            );
        }
    }
}

fn draw_here(f: &mut Frame, r: Rect) {
    let s = lang::l(" buraya ", " here ");
    let side = (r.width as usize).saturating_sub(s.width()) / 2;
    f.render_widget(
        Paragraph::new(Span::styled(
            format!("{}{s}{}", "─".repeat(side), "─".repeat(side)),
            Style::default().fg(CLARET),
        )),
        r,
    );
}

fn draw_card(f: &mut Frame, r: Rect, it: &Item, sel: bool, marked: bool, grabbing: bool) {
    let border = if sel && grabbing {
        WARN
    } else if sel {
        CLARET
    } else if it.hidden {
        FAINT
    } else {
        MUTED
    };
    let w = r.width.saturating_sub(4) as usize;
    let label = it.label.split_whitespace().last().unwrap_or("").to_string();
    let title = fit(
        &format!(
            "{}{label} · {}",
            if marked { "■ " } else { "" },
            if it.title.trim().is_empty() {
                "—"
            } else {
                it.title.trim()
            }
        ),
        w,
    );
    let mut ts = Style::default().fg(if sel { BEIGE } else { FG });
    if sel {
        ts = ts.add_modifier(Modifier::BOLD);
    }
    if it.hidden {
        ts = ts.fg(FAINT).add_modifier(Modifier::ITALIC);
    }
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border))
        .title(Span::styled(format!(" {title} "), ts))
        .style(theme::base());
    let inner = block.inner(r);
    f.render_widget(block, r);
    let room = inner.width.saturating_sub(2) as usize;
    let mut lines = vec![Line::from(Span::styled(
        fit(&it.stats(), room),
        Style::default().fg(FAINT),
    ))];
    lines.push(Line::from(Span::styled(
        fit(&it.cast.join(", "), room),
        Style::default().fg(MUTED),
    )));
    let summary = it.summary.clone().map_or_else(
        || {
            Span::styled(
                if sel {
                    lang::l("e: özet yaz", "e: summary")
                } else {
                    ""
                }
                .to_string(),
                Style::default().fg(FAINT),
            )
        },
        |s| {
            Span::styled(
                fit(&s, room),
                Style::default().fg(FG).add_modifier(Modifier::ITALIC),
            )
        },
    );
    lines.push(Line::from(summary));
    f.render_widget(
        Paragraph::new(lines),
        Rect {
            x: inner.x + 1,
            width: inner.width.saturating_sub(1),
            ..inner
        },
    );
}

/// The strip beside the writing: the tree, the cursor's scene lit and open.
pub fn draw_strip(f: &mut Frame, area: Rect, items: &[Item], cursor: usize) {
    let buf = f.buffer_mut();
    for y in area.y..area.y + area.height {
        buf[(area.x, y)]
            .set_char('│')
            .set_style(Style::default().fg(FAINT).bg(theme::BG));
        for x in area.x + 1..area.x + area.width {
            buf[(x, y)]
                .set_char(' ')
                .set_style(Style::default().bg(theme::BG));
        }
    }
    let inner = Rect {
        x: area.x + 2,
        width: area.width.saturating_sub(3),
        ..area
    };
    let w = inner.width as usize;
    let here = items.iter().rposition(|i| i.start <= cursor);
    let mut lines: Vec<(Line<'static>, bool)> = Vec::new();
    for (i, it) in items.iter().enumerate() {
        let lit = Some(i) == here;
        match it.kind {
            UnitKind::Act => {
                if i > 0 {
                    lines.push((Line::raw(""), false));
                }
                let mut s = Style::default()
                    .fg(if lit { CLARET } else { MUTED })
                    .add_modifier(Modifier::BOLD);
                if it.hidden {
                    s = s.fg(FAINT).add_modifier(Modifier::ITALIC);
                }
                lines.push((Line::from(Span::styled(fit(&it.name(), w), s)), lit));
            }
            UnitKind::Scene => {
                let mut s = Style::default().fg(if lit { CLARET } else { FAINT });
                if lit {
                    s = s.add_modifier(Modifier::BOLD);
                }
                if it.hidden {
                    s = s.add_modifier(Modifier::ITALIC);
                }
                let lead = if lit { "▸ " } else { "  " };
                lines.push((
                    Line::from(Span::styled(fit(&format!("{lead}{}", it.name()), w), s)),
                    lit,
                ));
                if lit {
                    if let Some(sum) = &it.summary {
                        for l in wrap(sum, w.saturating_sub(4)) {
                            lines.push((
                                Line::from(Span::styled(
                                    format!("    {l}"),
                                    Style::default().fg(MUTED).add_modifier(Modifier::ITALIC),
                                )),
                                true,
                            ));
                        }
                    }
                    let mut info = it.stats();
                    if !it.cast.is_empty() {
                        info = format!("{info} · {}", it.cast.join(", "));
                    }
                    for l in wrap(&info, w.saturating_sub(4)) {
                        lines.push((
                            Line::from(Span::styled(
                                format!("    {l}"),
                                Style::default().fg(FAINT),
                            )),
                            true,
                        ));
                    }
                }
            }
        }
    }
    let rows = inner.height.saturating_sub(1) as usize;
    let focus = lines.iter().position(|l| l.1).unwrap_or(0);
    let start = focus
        .saturating_sub(rows / 3)
        .min(lines.len().saturating_sub(rows));
    let shown: Vec<Line> = lines
        .into_iter()
        .skip(start)
        .take(rows)
        .map(|l| l.0)
        .collect();
    f.render_widget(
        Paragraph::new(shown),
        Rect {
            y: inner.y + 1,
            height: inner.height.saturating_sub(1),
            ..inner
        },
    );
}
