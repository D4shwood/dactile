//! Learning a role by heart, full screen: the cues before the role's line,
//! the line itself hidden; Space shows its first letters, then all of it,
//! then goes on to the next.

use super::popup::fit;
use super::theme::{self, BEIGE, CLARET, FAINT, FG, MUTED};
use dactile::lang;
use dactile::memorize::{Cue, initials};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph};
use unicode_width::UnicodeWidthStr;

pub struct Memo {
    /// The role, an alias, and its script name.
    pub role: String,
    pub name: String,
    /// The line being asked: an index into the cues.
    pub at: usize,
    /// 0 hidden, 1 first letters, 2 all of it.
    pub shown: u8,
    /// Opened from F9: Esc goes back there.
    pub back: bool,
}

/// The role's lines, as indexes into `cues`.
pub fn ours(cues: &[Cue]) -> Vec<usize> {
    (0..cues.len()).filter(|&i| cues[i].ours).collect()
}

/// Lines of `s` at most `w` wide, broken between words.
fn wrap(s: &str, w: usize) -> Vec<String> {
    let mut out = vec![String::new()];
    for word in s.split(' ') {
        let last = out.last_mut().expect("one line at least");
        if !last.is_empty() && last.width() + 1 + word.width() > w {
            out.push(word.to_string());
        } else {
            if !last.is_empty() {
                last.push(' ');
            }
            last.push_str(word);
        }
    }
    out
}

pub fn draw(f: &mut Frame, area: Rect, cues: &[Cue], m: &Memo, actor: Option<&str>) {
    f.render_widget(Clear, area);
    f.render_widget(Block::default().style(theme::base()), area);
    let all = ours(cues);
    let n = all.iter().position(|&i| i == m.at).map_or(0, |k| k + 1);
    let who = match actor {
        Some(a) => format!("{} ({a})", m.name),
        None => m.name.clone(),
    };
    let head = Line::from(vec![
        Span::styled(
            lang::l(" ezber", " memorize"),
            Style::default().fg(CLARET).add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!("   {who}   "), Style::default().fg(BEIGE)),
        Span::styled(format!("{n}/{}", all.len()), Style::default().fg(MUTED)),
    ]);
    f.render_widget(Paragraph::new(head), Rect { height: 1, ..area });
    let keys = match m.shown {
        0 => lang::l(
            "␣ ilk harfler   ⌫ önceki   ↓ atla   Esc kapat",
            "␣ first letters   ⌫ back   ↓ skip   Esc close",
        ),
        1 => lang::l(
            "␣ hepsini göster   ⌫ önceki   ↓ atla   Esc kapat",
            "␣ show it all   ⌫ back   ↓ skip   Esc close",
        ),
        _ => lang::l(
            "␣ sıradaki   ⌫ önceki   Esc kapat",
            "␣ next   ⌫ back   Esc close",
        ),
    };
    f.render_widget(
        Paragraph::new(Span::styled(keys, Style::default().fg(FAINT))),
        Rect {
            x: area.x + 1,
            y: area.y + area.height.saturating_sub(1),
            width: area.width.saturating_sub(2),
            height: 1,
        },
    );
    if all.is_empty() {
        f.render_widget(
            Paragraph::new(Span::styled(
                lang::l("Bu rolün repliği yok.", "This role has no lines."),
                Style::default().fg(MUTED),
            )),
            Rect {
                x: area.x + 2,
                y: area.y + 2,
                width: area.width.saturating_sub(4),
                height: 1,
            },
        );
        return;
    }
    let width = (area.width as usize).saturating_sub(4).min(76);
    let name_w = cues
        .iter()
        .filter_map(|c| c.who.as_ref().map(|w| w.width()))
        .max()
        .unwrap_or(0)
        .min(16)
        + 2;
    let x = area.x + (area.width.saturating_sub(width as u16)) / 2;
    let room = area.height.saturating_sub(4) as usize;
    // Lines from the bottom up: the asked line last, the cues above it.
    let mut lines: Vec<Line> = Vec::new();
    for i in (0..=m.at.min(cues.len() - 1)).rev() {
        let c = &cues[i];
        let asked = i == m.at;
        let text = if asked {
            match m.shown {
                0 => "…".to_string(),
                1 => initials(&c.text),
                _ => c.text.clone(),
            }
        } else {
            c.text.clone()
        };
        let style = if c.heading {
            Style::default().fg(CLARET).add_modifier(Modifier::BOLD)
        } else if asked {
            Style::default().fg(FG).add_modifier(Modifier::BOLD)
        } else if c.ours {
            Style::default().fg(MUTED)
        } else if c.who.is_none() {
            Style::default().fg(MUTED).add_modifier(Modifier::ITALIC)
        } else {
            Style::default().fg(FG)
        };
        let mut block: Vec<Line> = Vec::new();
        if c.heading {
            block.push(Line::from(Span::styled(fit(&text, width), style)).centered());
        } else {
            let (lead, w) = match &c.who {
                Some(who) => (who.clone(), width.saturating_sub(name_w)),
                None => (String::new(), width),
            };
            let pad = if c.who.is_some() { name_w } else { 0 };
            for (k, part) in wrap(&text, w.max(10)).into_iter().enumerate() {
                let name = if k == 0 { lead.clone() } else { String::new() };
                let name_style = if c.ours {
                    Style::default().fg(CLARET).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(BEIGE)
                };
                block.push(Line::from(vec![
                    Span::styled(
                        format!("{:<pad$}", fit(&name, pad.saturating_sub(1))),
                        name_style,
                    ),
                    Span::styled(part, style),
                ]));
            }
        }
        block.push(Line::raw(""));
        if lines.len() + block.len() > room {
            break;
        }
        // Stop at the scene's heading: what came before is another scene.
        let heading = c.heading;
        block.extend(lines);
        lines = block;
        if heading && i != m.at {
            break;
        }
    }
    let top = area.y + 2 + (room.saturating_sub(lines.len())) as u16;
    f.render_widget(
        Paragraph::new(lines),
        Rect {
            x,
            y: top,
            width: width as u16,
            height: area.height.saturating_sub(top - area.y + 1),
        },
    );
}
