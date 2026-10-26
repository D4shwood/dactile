//! Full-screen panels over the text: the cast, the scenes, help.

use super::theme::{self, BEIGE, CLARET, FAINT, FG, MUTED, WARN};
use dactile::cast::{Kind as CastKind, Row};
use dactile::lang;
use dactile::lex::{self, Kind};
use dactile::outline;
use dactile::render::Doc;
use dactile::sidecar::Sidecar;
use dactile::timing;
use ratatui::Frame;
use ratatui::layout::{Position, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

fn panel(title: &str) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(CLARET))
        .title(Span::styled(
            format!(" {title} "),
            Style::default().fg(BEIGE),
        ))
        .style(theme::base())
}

fn big(area: Rect) -> Rect {
    let w = area.width.saturating_sub(4).max(30);
    let h = area.height.saturating_sub(2).max(8);
    Rect {
        x: area.x + (area.width - w) / 2,
        y: area.y + 1,
        width: w,
        height: h,
    }
}

fn inner(r: Rect) -> Rect {
    Rect {
        x: r.x + 1,
        y: r.y + 1,
        width: r.width.saturating_sub(2),
        height: r.height.saturating_sub(2),
    }
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
    let r = big(area);
    f.render_widget(Clear, r);
    let order = if p.by_lines {
        lang::l("en çok konuşan", "most lines")
    } else {
        lang::l("oyundaki sıra", "the play's order")
    };
    f.render_widget(panel(&lang::f("kişiler — {}", "cast — {}", &[&order])), r);
    let inn = inner(r);
    let st = stats(doc, text);
    let list = entries(doc, &st, p.by_lines);
    let rows = inn.height.saturating_sub(3) as usize;
    let start = p.sel.saturating_sub(rows.saturating_sub(1));
    let mut lines: Vec<Line> = Vec::new();
    for (n, e) in list.iter().enumerate().skip(start).take(rows) {
        let sel = n == p.sel;
        let mark = Span::styled(if sel { "▸ " } else { "  " }, Style::default().fg(CLARET));
        match *e {
            Entry::Group(g) => {
                let grp = &doc.cast.groups[g];
                lines.push(Line::from(vec![
                    mark,
                    Span::styled(
                        format!(
                            "┌ {}",
                            if grp.desc.is_empty() {
                                "grup"
                            } else {
                                &grp.desc
                            }
                        ),
                        Style::default().fg(MUTED).add_modifier(Modifier::ITALIC),
                    ),
                ]));
            }
            Entry::Character(i) => {
                let c = &doc.cast.characters[i];
                let indent = if c.group.is_some() && !p.by_lines {
                    "│ "
                } else {
                    ""
                };
                let star = if c.star { "★" } else { " " };
                let kind = match c.kind {
                    CastKind::Hidden => lang::l(" gizli", " hidden"),
                    CastKind::Plain => lang::l(" tabloda yok", " not in the table"),
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
                    ""
                };
                let (lines_n, words) = st[i];
                let desc = c.desc.as_deref().unwrap_or("");
                let full = c
                    .full
                    .as_deref()
                    .map(|f| format!(" ({f})"))
                    .unwrap_or_default();
                lines.push(Line::from(vec![
                    mark,
                    Span::styled(indent.to_string(), Style::default().fg(FAINT)),
                    Span::styled(star, Style::default().fg(BEIGE)),
                    Span::styled(format!(" \\{:<8}", c.alias), Style::default().fg(MUTED)),
                    Span::styled(format!("{:<20}", c.script), name_style),
                    Span::styled(
                        lang::f(
                            "{} replik {} kelime",
                            "{} lines {} words",
                            &[&format!("{lines_n:>4}"), &format!("{words:>6}")],
                        ),
                        Style::default().fg(MUTED),
                    ),
                    Span::styled(format!("{full} {desc}{kind}"), Style::default().fg(FAINT)),
                    Span::styled(card, Style::default().fg(BEIGE)),
                ]));
            }
        }
    }
    f.render_widget(
        Paragraph::new(lines).style(theme::base()),
        Rect {
            height: inn.height.saturating_sub(2),
            ..inn
        },
    );
    let keys = if p.by_lines {
        lang::l(
            "⏎ kart   e düzenle   n yeni   d sil   s sıra   Esc kapat",
            "⏎ card   e edit   n new   d delete   s order   Esc close",
        )
    } else {
        lang::l(
            "⏎ kart   e düzenle   n yeni   d sil   Alt+↑↓ taşı   s sıra   Esc kapat",
            "⏎ card   e edit   n new   d delete   Alt+↑↓ move   s order   Esc close",
        )
    };
    f.render_widget(
        Paragraph::new(keys).style(theme::base().fg(FAINT)),
        Rect {
            y: inn.y + inn.height.saturating_sub(1),
            height: 1,
            ..inn
        },
    );
}

// ------------------------------------------------------------------ scenes

pub fn draw_scenes(f: &mut Frame, area: Rect, doc: &Doc, wpm: u32, sel: usize) {
    let r = big(area);
    f.render_widget(Clear, r);
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
    let inn = inner(r);
    let rows = inn.height.saturating_sub(2) as usize;
    let mut lines: Vec<Line> = Vec::new();
    let mut act = None;
    let mut sel_line = 0;
    for (i, s) in doc.outline.scenes.iter().enumerate() {
        if s.act != act {
            act = s.act;
            if let Some(a) = act {
                let a = &doc.outline.acts[a];
                let title = if a.title.trim().is_empty() {
                    String::new()
                } else {
                    format!(" · {}", a.title.trim())
                };
                lines.push(Line::from(Span::styled(
                    format!(
                        "{}. {}{title}",
                        outline::roman(a.number),
                        lang::l("PERDE", "ACT")
                    ),
                    Style::default().fg(CLARET).add_modifier(Modifier::BOLD),
                )));
            }
        }
        let is_sel = i == sel;
        if is_sel {
            sel_line = lines.len();
        }
        let place = if s.place.trim().is_empty() {
            String::new()
        } else {
            format!(" — {}", s.place.trim())
        };
        let names: Vec<String> = s
            .cast
            .iter()
            .map(|a| doc.cast.get(a).map_or(a.clone(), |c| c.script.clone()))
            .collect();
        lines.push(Line::from(vec![
            Span::styled(
                if is_sel { "▸ " } else { "  " },
                Style::default().fg(CLARET),
            ),
            Span::styled(
                format!("{} {}{place}", lang::l("Sahne", "Scene"), s.label),
                Style::default().fg(if is_sel { BEIGE } else { FG }),
            ),
            Span::styled(
                lang::f(
                    "   {} replik · {} kelime · ≈ {}",
                    "   {} lines · {} words · ≈ {}",
                    &[
                        &s.lines,
                        &s.words,
                        &timing::format(timing::scene_secs(s, wpm)),
                    ],
                ),
                Style::default().fg(MUTED),
            ),
        ]));
        lines.push(Line::from(Span::styled(
            format!("    {}", names.join(", ")),
            Style::default().fg(FAINT),
        )));
    }
    if doc.outline.scenes.is_empty() {
        lines.push(Line::from(Span::styled(
            lang::l(
                "Henüz sahne yok: \\sahne ile başla.",
                "No scenes yet: start one with \\sahne.",
            ),
            Style::default().fg(MUTED),
        )));
    }
    let start = sel_line
        .saturating_sub(rows / 2)
        .min(lines.len().saturating_sub(rows));
    let shown: Vec<Line> = lines.into_iter().skip(start).take(rows).collect();
    f.render_widget(
        Paragraph::new(shown).style(theme::base()),
        Rect {
            height: inn.height.saturating_sub(1),
            ..inn
        },
    );
    f.render_widget(
        Paragraph::new(lang::l(
            "⏎ git   ↑↓ seç   Esc kapat",
            "⏎ go   ↑↓ choose   Esc close",
        ))
        .style(theme::base().fg(FAINT)),
        Rect {
            y: inn.y + inn.height.saturating_sub(1),
            height: 1,
            ..inn
        },
    );
}

// ------------------------------------------------------------------ help

pub const HELP: &[(&str, &str, &str)] = &[
    ("", "Yazarken", "Writing"),
    (
        "\\",
        "öneri menüsü: karakterler, komutlar (↑↓ seç, Tab/⏎ kabul, Esc kapat)",
        "suggestions: characters, commands (↑↓ choose, Tab/⏎ take, Esc close)",
    ),
    (
        "\\se + boşluk",
        "SELİN konuşur; satır başında tanımsız kısaltma yeni karakter sorar",
        "SELİN speaks; an unknown alias at a line's start asks for a new character",
    ),
    (
        "\\sedo + boşluk",
        "yönerge: (Selin …), fiil listesi açılır",
        "a direction: (Selin …), with a list of verbs",
    ),
    (
        "((",
        "replik içinde yönerge: (güler)",
        "a direction inside a line: (laughs)",
    ),
    (
        "Backspace",
        "bir makronun içine girince satır ham açılır; Esc kapatır",
        "into a macro: the line shows its source; Esc closes it",
    ),
    (
        "^K",
        "karakter ya da komut ara",
        "search characters and commands",
    ),
    ("", "Pencereler", "Panels"),
    (
        "^O",
        "kişiler: düzenle, kart, sıra, yeni, sil",
        "cast: edit, card, order, new, delete",
    ),
    ("^G", "sahneler ve süreler", "scenes and running times"),
    (
        "Alt+I",
        "oyun bilgisi: başlık, yazar, taslak/final",
        "cover details: title, author, draft/final",
    ),
    ("^F", "bul ve değiştir", "find and replace"),
    (
        "Alt+S",
        "imleçteki kelime için yazım önerileri",
        "spelling suggestions for the word at the cursor",
    ),
    ("", "Görünüm", "View"),
    ("F2", "tüm metin ham", "everything as source"),
    (
        "F3",
        "daktilo: imleç satırı ortada",
        "typewriter: the cursor's line in the middle",
    ),
    (
        "F4",
        "sahne odağı: diğer sahneler kapanır",
        "scene focus: the other scenes fold",
    ),
    ("F5", "imleç dışını soldur", "dim all but the cursor's line"),
    (
        "Alt+P",
        "kişi tanımları ve oyun bilgisi satırlarını aç",
        "show the cast and cover lines",
    ),
    ("", "Dosya", "File"),
    (
        "^S",
        "kaydet (zaten kendiliğinden kaydeder)",
        "save (it saves by itself anyway)",
    ),
    ("^Z / ^Y", "geri al / yinele", "undo / redo"),
    ("^C ^X ^V", "kopyala / kes / yapıştır", "copy / cut / paste"),
    ("^Q", "çık", "quit"),
];

pub fn draw_help(f: &mut Frame, area: Rect) {
    let r = big(area);
    f.render_widget(Clear, r);
    f.render_widget(
        panel(lang::l(
            "yardım — F1 ya da Esc kapatır",
            "help — F1 or Esc closes",
        )),
        r,
    );
    let lines: Vec<Line> = HELP
        .iter()
        .map(|(k, tr, en)| {
            let what = if lang::is_tr() { tr } else { en };
            let k = if lang::is_tr() {
                k.to_string()
            } else {
                k.replace("boşluk", "space")
            };
            if k.is_empty() {
                Line::from(Span::styled(
                    what.to_string(),
                    Style::default().fg(CLARET).add_modifier(Modifier::BOLD),
                ))
            } else {
                Line::from(vec![
                    Span::styled(format!("  {k:<16}"), Style::default().fg(BEIGE)),
                    Span::styled(what.to_string(), Style::default().fg(MUTED)),
                ])
            }
        })
        .collect();
    f.render_widget(Paragraph::new(lines).style(theme::base()), inner(r));
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
    f.render_widget(Clear, r);
    f.render_widget(panel(word), r);
    let lines: Vec<Line> = items
        .iter()
        .take(9)
        .enumerate()
        .map(|(i, s)| {
            let is_sel = i == sel;
            let add = s.starts_with('+');
            Line::from(vec![
                Span::styled(
                    if is_sel { "▸ " } else { "  " },
                    Style::default().fg(CLARET),
                ),
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
            ])
        })
        .collect();
    f.render_widget(Paragraph::new(lines).style(theme::base()), inner(r));
}
