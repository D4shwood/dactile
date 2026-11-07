//! F1: the keys, drawn as a keyboard. Pressing a key lights it and says what
//! it does; the bound keys are coloured by what they are for.

use super::popup::{fit, float, inner, panel};
use super::theme::{self, BEIGE, CLARET, FAINT, FG, MUTED, WARN};
use dactile::cast;
use dactile::keys::{self, BINDS, Bind, Cat, FKEYS, Only, rows};
use dactile::lang;
use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use unicode_width::UnicodeWidthStr;

fn color(c: Cat) -> Color {
    match c {
        Cat::File => CLARET,
        Cat::Edit => BEIGE,
        Cat::Panel => Color::Rgb(0x6f, 0x93, 0xb8),
        Cat::View => Color::Rgb(0x86, 0xa8, 0x6e),
        Cat::Write => WARN,
    }
}

/// A key pressed on the help screen.
#[derive(Debug, Clone, PartialEq)]
pub struct Pressed {
    pub cap: String,
    pub ctrl: bool,
    pub alt: bool,
}

/// What the help screen shows: the key pressed or pointed at, or a
/// category pointed at in the legend; and where its caps are, for the mouse.
#[derive(Debug, Clone, Default)]
pub struct Help {
    pub pressed: Option<Pressed>,
    pub cat: Option<Cat>,
    hits: Vec<(Rect, Hit)>,
}

#[derive(Debug, Clone, Copy)]
enum Hit {
    Cap(&'static str),
    Cat(Cat),
}

impl Help {
    /// A key pressed on the help screen.
    pub fn key(&mut self, k: KeyEvent, layout: &str) {
        if let Some(p) = pressed(k, layout) {
            self.pressed = Some(p);
            self.cat = None;
        }
    }

    /// The mouse over (or clicking) a cap or a legend entry: it lights up as
    /// if pressed, with the modifiers its first binding needs.
    pub fn point(&mut self, x: u16, y: u16, layout: &str) {
        let Some(&(_, hit)) = self
            .hits
            .iter()
            .find(|(r, _)| x >= r.x && x < r.x + r.width && y >= r.y && y < r.y + r.height)
        else {
            return;
        };
        match hit {
            Hit::Cap(cap) => {
                let first = binds_on(cap, layout).next();
                self.pressed = Some(Pressed {
                    cap: cap.to_string(),
                    ctrl: first.is_some_and(|b| b.ctrl),
                    alt: first.is_some_and(|b| b.alt),
                });
                self.cat = None;
            }
            Hit::Cat(c) => {
                self.cat = Some(c);
                self.pressed = None;
            }
        }
    }
}

/// The cap a key event lands on, in `layout`'s letters.
pub fn pressed(k: KeyEvent, layout: &str) -> Option<Pressed> {
    let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
    let alt = k.modifiers.contains(KeyModifiers::ALT);
    let cap = match k.code {
        KeyCode::F(n) => format!("F{n}"),
        KeyCode::Tab | KeyCode::BackTab => "Tab".into(),
        KeyCode::Backspace => "⌫".into(),
        KeyCode::Enter => "⏎".into(),
        KeyCode::Char(c) if c.is_alphabetic() => {
            if layout == "us" {
                c.to_uppercase().to_string()
            } else {
                cast::upper_tr(&c.to_string())
            }
        }
        KeyCode::Char(c) => c.to_string(),
        KeyCode::Left => "←".into(),
        KeyCode::Right => "→".into(),
        KeyCode::Up => "↑".into(),
        KeyCode::Down => "↓".into(),
        _ => return None,
    };
    Some(Pressed { cap, ctrl, alt })
}

/// The letter rows of a layout: Turkish Q, Turkish F, or US.
fn binds_on<'a>(cap: &'a str, layout: &'a str) -> impl Iterator<Item = &'static Bind> + 'a {
    BINDS
        .iter()
        .filter(move |b| b.only.fits(Only::Light) && b.cap(layout) == cap)
}

/// Rows the keyboard needs: five rows of caps three high, a legend, two
/// lines about the key, the border.
const KEYBOARD_HEIGHT: u16 = 5 * 3 + 6;
const KEYBOARD_WIDTH: u16 = 76;

pub fn draw_help(f: &mut Frame, area: Rect, layout: &str, help: &mut Help) {
    help.hits.clear();
    if area.width < KEYBOARD_WIDTH + 4 || area.height < KEYBOARD_HEIGHT + 2 {
        return draw_list(f, area);
    }
    let pressed = help.pressed.as_ref();
    let cat = help.cat;
    let r = Rect {
        x: area.x + (area.width - KEYBOARD_WIDTH) / 2,
        y: area.y + (area.height - KEYBOARD_HEIGHT) / 2,
        width: KEYBOARD_WIDTH,
        height: KEYBOARD_HEIGHT,
    };
    float(f, r);
    f.render_widget(
        panel(lang::l(
            "tuşlar — bir tuşa bas ya da fareyle üstüne gel; Esc kapatır",
            "keys — press one or point at it; Esc closes",
        )),
        r,
    );
    let inn = inner(r);
    let lit = |cap: &str| -> bool {
        pressed
            .is_some_and(|p| p.cap == cap || (cap == "Ctrl" && p.ctrl) || (cap == "Alt" && p.alt))
            || cat.is_some_and(|c| binds_on(cap, layout).any(|b| b.cat == c))
    };
    let mut y = inn.y;
    let letters = rows(layout);
    let special = keys::special(Only::Light);
    let lines: [(&[&str], u16); 5] = [
        (&FKEYS, 0),
        (letters[0], 1),
        (letters[1], 2),
        (letters[2], 4),
        (&special, 0),
    ];
    for (caps, indent) in lines {
        let mut x = inn.x + indent;
        for &cap in caps {
            let w = (cap.width() as u16 + 3).max(5);
            if x + w > inn.x + inn.width {
                break;
            }
            let r = Rect {
                x,
                y,
                width: w,
                height: 3,
            };
            draw_cap(f, r, cap, layout, lit(cap));
            help.hits.push((r, Hit::Cap(cap)));
            x += w;
        }
        y += 3;
    }
    // The colours; pointing at one lights its keys.
    let mut legend = Vec::new();
    let mut lx = inn.x;
    for c in Cat::ALL {
        let label = format!("■ {}   ", c.name());
        let w = label.width() as u16;
        help.hits.push((
            Rect {
                x: lx,
                y,
                width: w,
                height: 1,
            },
            Hit::Cat(c),
        ));
        lx += w;
        let on = cat == Some(c);
        legend.push(Span::styled("■ ", Style::default().fg(color(c))));
        legend.push(Span::styled(
            format!("{}   ", c.name()),
            Style::default()
                .fg(if on { color(c) } else { MUTED })
                .add_modifier(if on {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                }),
        ));
    }
    f.render_widget(
        Paragraph::new(Line::from(legend)).style(theme::base()),
        Rect {
            y,
            height: 1,
            ..inn
        },
    );
    // What the pressed key does.
    let about: Vec<Line> = match pressed {
        None if cat.is_some() => {
            let c = cat.unwrap_or(Cat::Write);
            let combos: Vec<String> = BINDS
                .iter()
                .filter(|b| b.cat == c && b.only.fits(Only::Light))
                .map(|b| b.combo(layout))
                .collect();
            vec![
                Line::from(vec![
                    Span::styled(
                        format!("{}  ", c.name()),
                        Style::default().fg(color(c)).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        fit(&combos.join("   "), inn.width as usize - 14),
                        Style::default().fg(FG),
                    ),
                ]),
                Line::from(Span::styled(
                    lang::l(
                        "Birinin üstüne gel: ne yaptığı burada yazar.",
                        "Point at one: what it does shows here.",
                    ),
                    Style::default().fg(FAINT),
                )),
            ]
        }
        None => vec![Line::from(Span::styled(
            lang::l(
                "Bir tuşa bas ya da fareyle üstüne gel: ne yaptığı burada yazar.",
                "Press a key or point at it: what it does shows here.",
            ),
            Style::default().fg(FAINT),
        ))],
        Some(p) => {
            let exact = binds_on(&p.cap, layout).find(|b| b.ctrl == p.ctrl && b.alt == p.alt);
            let mut out = Vec::new();
            match exact {
                Some(b) => out.push(Line::from(vec![
                    Span::styled(
                        format!("{}  ", b.combo(layout)),
                        Style::default()
                            .fg(color(b.cat))
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        fit(b.what(), inn.width as usize - 12),
                        Style::default().fg(FG),
                    ),
                ])),
                None => out.push(Line::from(Span::styled(
                    lang::l(
                        "Bu tuşun burada bir işi yok.",
                        "This key does nothing special here.",
                    ),
                    Style::default().fg(MUTED),
                ))),
            }
            // The same cap with other modifiers.
            let others: Vec<String> = binds_on(&p.cap, layout)
                .filter(|b| !exact.is_some_and(|e| std::ptr::eq(*b, e)))
                .map(|b| format!("{} {}", b.combo(layout), b.what()))
                .collect();
            let note = match p.cap.as_str() {
                "Tab" => Some(lang::l(
                    "Terminal Ctrl+I'yi de Tab olarak gönderir.",
                    "The terminal sends Ctrl+I as Tab too.",
                )),
                "⌫" => Some(lang::l(
                    "Terminal Ctrl+H'yi de ⌫ olarak gönderir.",
                    "The terminal sends Ctrl+H as ⌫ too.",
                )),
                "⏎" => Some(lang::l(
                    "Terminal Ctrl+M'yi de ⏎ olarak gönderir.",
                    "The terminal sends Ctrl+M as ⏎ too.",
                )),
                _ => None,
            };
            let second = if !others.is_empty() {
                others.join("  ·  ")
            } else {
                note.unwrap_or("").to_string()
            };
            out.push(Line::from(Span::styled(
                fit(&second, inn.width as usize),
                Style::default().fg(MUTED),
            )));
            out
        }
    };
    f.render_widget(
        Paragraph::new(about).style(theme::base()),
        Rect {
            y: y + 2,
            height: 2,
            ..inn
        },
    );
}

/// Help > Commands: the commands down the right side; the chosen one's
/// explanation and example at the foot.
pub fn draw_guide(f: &mut Frame, area: Rect, at: usize) {
    use dactile::guide::{ENTRIES, Group};
    let w = 46.min(area.width);
    let r = Rect {
        x: area.x + area.width - w,
        width: w,
        ..area
    };
    float(f, r);
    f.render_widget(
        panel(lang::l(
            "komutlar — ↑↓ seç, Esc kapat",
            "commands — ↑↓ choose, Esc closes",
        )),
        r,
    );
    let inn = inner(r);
    let width = inn.width as usize;
    let chosen = &ENTRIES[at];
    let mut foot = vec![Line::from(Span::styled(
        chosen.name,
        Style::default().fg(CLARET).add_modifier(Modifier::BOLD),
    ))];
    for l in super::board::wrap(chosen.what(), width) {
        foot.push(Line::from(Span::styled(l, Style::default().fg(FG))));
    }
    if !chosen.example.is_empty() {
        foot.push(Line::from(Span::styled(
            fit(chosen.example, width),
            Style::default().fg(BEIGE),
        )));
    }
    let foot_h = (foot.len() as u16).min(inn.height / 2);
    let list_h = inn.height.saturating_sub(foot_h + 1) as usize;
    let mut lines = Vec::new();
    let mut here = 0;
    for g in Group::ALL {
        lines.push(Line::from(Span::styled(
            g.name(),
            Style::default().fg(MUTED).add_modifier(Modifier::BOLD),
        )));
        for (i, e) in ENTRIES.iter().enumerate().filter(|(_, e)| e.group == g) {
            if i == at {
                here = lines.len();
            }
            let style = if i == at {
                Style::default().fg(CLARET).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(FG)
            };
            let lead = if i == at { "▸ " } else { "  " };
            lines.push(Line::from(Span::styled(format!("{lead}{}", e.name), style)));
        }
    }
    let top = here
        .saturating_sub(list_h / 2)
        .min(lines.len().saturating_sub(list_h));
    f.render_widget(
        Paragraph::new(lines.split_off(top)).style(theme::base()),
        Rect {
            height: list_h as u16,
            ..inn
        },
    );
    f.render_widget(
        Paragraph::new(foot).style(theme::base()),
        Rect {
            y: inn.y + inn.height - foot_h,
            height: foot_h,
            ..inn
        },
    );
}

/// One key cap: coloured by what it does, lit when pressed.
fn draw_cap(f: &mut Frame, r: Rect, cap: &str, layout: &str, lit: bool) {
    let cat = binds_on(cap, layout).next().map(|b| b.cat);
    let modifier = matches!(cap, "Ctrl" | "Alt" | "Esc");
    let color = match cat {
        Some(c) => color(c),
        None if modifier => MUTED,
        None => FAINT,
    };
    let (border, label) = if lit {
        (
            Style::default().fg(theme::BG).bg(color),
            Style::default()
                .fg(theme::BG)
                .bg(color)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        (
            Style::default().fg(color),
            Style::default()
                .fg(if cat.is_some() || modifier { FG } else { FAINT })
                .add_modifier(if cat.is_some() {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                }),
        )
    };
    let inner_w = (r.width - 2) as usize;
    let pad = inner_w.saturating_sub(cap.width());
    let lines = vec![
        Line::from(Span::styled(format!("╭{}╮", "─".repeat(inner_w)), border)),
        Line::from(vec![
            Span::styled("│", border),
            Span::styled(
                format!("{}{cap}{}", " ".repeat(pad / 2), " ".repeat(pad - pad / 2)),
                label,
            ),
            Span::styled("│", border),
        ]),
        Line::from(Span::styled(format!("╰{}╯", "─".repeat(inner_w)), border)),
    ];
    f.render_widget(Paragraph::new(lines).style(theme::base()), r);
}

/// Too small for the keyboard: the keys as a list, by what they are for.
fn draw_list(f: &mut Frame, area: Rect) {
    let mut lines: Vec<Line> = Vec::new();
    for c in Cat::ALL {
        lines.push(Line::from(Span::styled(
            format!(" {}", c.name()),
            Style::default().fg(color(c)).add_modifier(Modifier::BOLD),
        )));
        for b in BINDS
            .iter()
            .filter(|b| b.cat == c && b.only.fits(Only::Light))
        {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  {:<10}", b.combo("q")),
                    Style::default().fg(BEIGE),
                ),
                Span::styled(b.what().to_string(), Style::default().fg(MUTED)),
            ]));
        }
    }
    let w = area.width.saturating_sub(4);
    let h = (lines.len() as u16 + 2).min(area.height);
    let r = Rect {
        x: area.x + (area.width - w) / 2,
        y: area.y + (area.height - h) / 2,
        width: w,
        height: h,
    };
    float(f, r);
    f.render_widget(
        panel(lang::l("tuşlar — Esc kapatır", "keys — Esc closes")),
        r,
    );
    f.render_widget(Paragraph::new(lines).style(theme::base()), inner(r));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_land_on_caps() {
        let k = |code, m| KeyEvent::new(code, m);
        assert_eq!(
            pressed(k(KeyCode::Char('o'), KeyModifiers::CONTROL), "q"),
            Some(Pressed {
                cap: "O".into(),
                ctrl: true,
                alt: false
            })
        );
        // Turkish i is İ on a Turkish keyboard, I on a US one.
        assert_eq!(
            pressed(k(KeyCode::Char('i'), KeyModifiers::ALT), "q")
                .unwrap()
                .cap,
            "İ"
        );
        assert_eq!(
            pressed(k(KeyCode::Char('i'), KeyModifiers::ALT), "us")
                .unwrap()
                .cap,
            "I"
        );
        assert_eq!(
            pressed(k(KeyCode::F(4), KeyModifiers::NONE), "q")
                .unwrap()
                .cap,
            "F4"
        );
    }
}
