//! F1: the keys, drawn as a keyboard. Pressing a key lights it and says what
//! it does; the bound keys are coloured by what they are for.

use super::popup::{fit, float, inner, panel};
use super::theme::{self, BEIGE, CLARET, FAINT, FG, MUTED, WARN};
use dactile::cast;
use dactile::lang;
use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use unicode_width::UnicodeWidthStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cat {
    File,
    Edit,
    Panel,
    View,
    Write,
}

impl Cat {
    const ALL: [Cat; 5] = [Cat::Write, Cat::Panel, Cat::View, Cat::Edit, Cat::File];

    fn color(self) -> Color {
        match self {
            Cat::File => CLARET,
            Cat::Edit => BEIGE,
            Cat::Panel => Color::Rgb(0x6f, 0x93, 0xb8),
            Cat::View => Color::Rgb(0x86, 0xa8, 0x6e),
            Cat::Write => WARN,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Cat::File => lang::l("dosya", "file"),
            Cat::Edit => lang::l("düzenleme", "editing"),
            Cat::Panel => lang::l("pencereler", "panels"),
            Cat::View => lang::l("görünüm", "view"),
            Cat::Write => lang::l("yazarken", "writing"),
        }
    }
}

/// A key and what it does.
pub struct Bind {
    /// The key cap: "O", "F3", "\\", "Tab".
    pub cap: &'static str,
    pub ctrl: bool,
    pub alt: bool,
    pub cat: Cat,
    pub tr: &'static str,
    pub en: &'static str,
}

const fn b(
    cap: &'static str,
    ctrl: bool,
    alt: bool,
    cat: Cat,
    tr: &'static str,
    en: &'static str,
) -> Bind {
    Bind {
        cap,
        ctrl,
        alt,
        cat,
        tr,
        en,
    }
}

/// Every key of the writer, the one list the keyboard and the short list
/// both read.
pub const BINDS: &[Bind] = &[
    b(
        "\\",
        false,
        false,
        Cat::Write,
        "öneri menüsü: karakterler, komutlar",
        "suggestions: characters, commands",
    ),
    b(
        "#",
        false,
        false,
        Cat::Write,
        "satır başında: #sahne, #perde, #önsöz, #notlar",
        "at a line's start: #sahne, #perde, #önsöz, #notlar",
    ),
    b(
        ":",
        false,
        false,
        Cat::Write,
        "SELİN: yazınca SELİN'in repliği olur",
        "typing SELİN: makes it SELİN's line",
    ),
    b(
        "(",
        false,
        false,
        Cat::Write,
        "(( replik içinde yönerge açar: (güler)",
        "(( opens a direction inside a line: (laughs)",
    ),
    b(
        "Tab",
        false,
        false,
        Cat::Write,
        "boş satırda sıradaki konuşanı yazar",
        "on an empty line, writes who speaks next",
    ),
    b(
        "⌫",
        false,
        false,
        Cat::Write,
        "bir makronun içine girince satırın kaynağı açılır",
        "into a macro: the line shows its source",
    ),
    b(
        "P",
        true,
        false,
        Cat::Panel,
        "komut paleti: her şeyi adıyla bul ve yap",
        "command palette: find anything by name and do it",
    ),
    b(
        "O",
        true,
        false,
        Cat::Panel,
        "kişiler: düzenle, kart, sıra, yeni, sil",
        "the cast: edit, card, order, new, delete",
    ),
    b(
        "G",
        true,
        false,
        Cat::Panel,
        "sahne şeridi yazının yanında; iki kez: pano (liste, kartlar, taşı)",
        "the scene strip beside the writing; twice: the board (list, cards, move)",
    ),
    b(
        "E",
        false,
        true,
        Cat::Panel,
        "imlecin sahnesinin özeti (PDF'e girmez)",
        "the cursor's scene's summary (never printed)",
    ),
    b(
        "B",
        false,
        true,
        Cat::Edit,
        "sahneyi imlecin satırından ikiye böl",
        "split the scene in two at the cursor's line",
    ),
    b(
        "J",
        false,
        true,
        Cat::Edit,
        "sahneyi sonrakiyle birleştir",
        "join the scene with the next one",
    ),
    b(
        "K",
        true,
        false,
        Cat::Panel,
        "karakter ya da komut ara",
        "search characters and commands",
    ),
    b(
        "F",
        true,
        false,
        Cat::Panel,
        "bul ve değiştir",
        "find and replace",
    ),
    // Alt with the key that types "i": İ on a Turkish keyboard, I on a US one.
    b(
        "İ",
        false,
        true,
        Cat::Panel,
        "oyun bilgisi: başlık, yazar, taslak/final",
        "cover details: title, author, draft/final",
    ),
    b(
        "S",
        false,
        true,
        Cat::Panel,
        "imleçteki kelime için yazım önerileri",
        "spelling suggestions for the word at the cursor",
    ),
    b(
        "P",
        false,
        true,
        Cat::Panel,
        "kişi tanımlarını ve oyun bilgisini kaynakta aç",
        "show the cast and cover lines of the source",
    ),
    b("F1", false, false, Cat::View, "bu ekran", "this screen"),
    b(
        "F2",
        false,
        false,
        Cat::View,
        "bütün oyun kaynak olarak",
        "the whole play as source",
    ),
    b(
        "F3",
        false,
        false,
        Cat::View,
        "daktilo: imleç satırı ortada",
        "typewriter: the cursor's line in the middle",
    ),
    b(
        "F4",
        false,
        false,
        Cat::View,
        "sahne odağı: diğer sahneler kapanır",
        "scene focus: the other scenes fold",
    ),
    b(
        "F5",
        false,
        false,
        Cat::File,
        "PDF'i derle (arka planda, tek geçiş)",
        "build the PDF (in the background, one pass)",
    ),
    b(
        "O",
        false,
        true,
        Cat::File,
        "PDF'i görüntüleyicide aç",
        "open the PDF in the viewer",
    ),
    b(
        "L",
        false,
        true,
        Cat::File,
        "son derlemenin logunu aç",
        "open the last build's log",
    ),
    b(
        "F8",
        false,
        false,
        Cat::View,
        "imleç dışını soldur",
        "dim all but the cursor's line",
    ),
    b(
        "F6",
        false,
        false,
        Cat::View,
        "tema: kâğıt, sepya, gece, yüksek kontrast, koyu",
        "theme: paper, sepia, night, high contrast, dark",
    ),
    b(
        "F7",
        false,
        false,
        Cat::View,
        "sayfa görünümü: A4 yapraklar, sayfa numaraları",
        "page view: A4 sheets, page numbers",
    ),
    b("Z", true, false, Cat::Edit, "geri al", "undo"),
    b("Y", true, false, Cat::Edit, "yinele", "redo"),
    b("C", true, false, Cat::Edit, "kopyala", "copy"),
    b("X", true, false, Cat::Edit, "kes", "cut"),
    b(
        "V",
        true,
        false,
        Cat::Edit,
        "yapıştır; satır başında 'AD: …' satırları replik olur",
        "paste; at a line's start 'NAME: …' lines become lines",
    ),
    b(
        "A",
        true,
        false,
        Cat::Edit,
        "oyunun hepsini seç",
        "select the whole play",
    ),
    b(
        "S",
        true,
        false,
        Cat::File,
        "kaydet (zaten kendiliğinden kaydeder)",
        "save (it saves by itself anyway)",
    ),
    b("Q", true, false, Cat::File, "çık", "quit"),
];

impl Bind {
    /// The cap on `layout`'s keyboard.
    fn cap(&self, layout: &str) -> &'static str {
        if layout == "us" && self.cap == "İ" {
            "I"
        } else {
            self.cap
        }
    }

    fn what(&self) -> &'static str {
        if lang::is_tr() { self.tr } else { self.en }
    }

    fn combo(&self, layout: &str) -> String {
        let mut s = String::new();
        if self.ctrl {
            s.push_str("Ctrl+");
        }
        if self.alt {
            s.push_str("Alt+");
        }
        s.push_str(self.cap(layout));
        s
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
fn rows(layout: &str) -> [&'static [&'static str]; 3] {
    match layout {
        "f" => [
            &["F", "G", "Ğ", "I", "O", "D", "R", "N", "H", "P", "Q", "W"],
            &["U", "İ", "E", "A", "Ü", "T", "K", "M", "L", "Y", "Ş", "X"],
            &["J", "Ö", "V", "C", "Ç", "Z", "S", "B"],
        ],
        "us" => [
            &["Q", "W", "E", "R", "T", "Y", "U", "I", "O", "P"],
            &["A", "S", "D", "F", "G", "H", "J", "K", "L"],
            &["Z", "X", "C", "V", "B", "N", "M"],
        ],
        _ => [
            &["Q", "W", "E", "R", "T", "Y", "U", "I", "O", "P", "Ğ", "Ü"],
            &["A", "S", "D", "F", "G", "H", "J", "K", "L", "Ş", "İ"],
            &["Z", "X", "C", "V", "B", "N", "M", "Ö", "Ç"],
        ],
    }
}

const FKEYS: [&str; 12] = [
    "F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9", "F10", "F11", "F12",
];
const SPECIAL: [&str; 10] = ["Ctrl", "Alt", "Tab", "\\", "#", "(", ":", "⌫", "⏎", "Esc"];

fn binds_on<'a>(cap: &'a str, layout: &'a str) -> impl Iterator<Item = &'static Bind> + 'a {
    BINDS.iter().filter(move |b| b.cap(layout) == cap)
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
    let lines: [(&[&str], u16); 5] = [
        (&FKEYS, 0),
        (letters[0], 1),
        (letters[1], 2),
        (letters[2], 4),
        (&SPECIAL, 0),
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
        legend.push(Span::styled("■ ", Style::default().fg(c.color())));
        legend.push(Span::styled(
            format!("{}   ", c.name()),
            Style::default()
                .fg(if on { c.color() } else { MUTED })
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
                .filter(|b| b.cat == c)
                .map(|b| b.combo(layout))
                .collect();
            vec![
                Line::from(vec![
                    Span::styled(
                        format!("{}  ", c.name()),
                        Style::default().fg(c.color()).add_modifier(Modifier::BOLD),
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
                            .fg(b.cat.color())
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

/// One key cap: coloured by what it does, lit when pressed.
fn draw_cap(f: &mut Frame, r: Rect, cap: &str, layout: &str, lit: bool) {
    let cat = binds_on(cap, layout).next().map(|b| b.cat);
    let modifier = matches!(cap, "Ctrl" | "Alt" | "Esc");
    let color = match cat {
        Some(c) => c.color(),
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
            Style::default().fg(c.color()).add_modifier(Modifier::BOLD),
        )));
        for b in BINDS.iter().filter(|b| b.cat == c) {
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
    fn every_bound_cap_is_on_the_keyboard() {
        for layout in ["q", "f", "us"] {
            let letters = rows(layout);
            for b in BINDS {
                let cap = b.cap(layout);
                let on = FKEYS.contains(&cap)
                    || SPECIAL.contains(&cap)
                    || letters.iter().any(|r| r.contains(&cap));
                assert!(on, "{cap} not on the {layout} keyboard");
            }
        }
    }

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
