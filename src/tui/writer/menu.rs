//! F10: the menus along the top, as a word processor has them. The palette
//! (^P) offers the same entries, so the two never drift apart.

use super::*;
use crate::tui::theme::{FG, SELECTION};
use unicode_width::UnicodeWidthStr;

/// An entry of a menu: its name, the key that does the same, what it does.
pub struct MenuItem {
    pub tr: &'static str,
    pub en: &'static str,
    pub key: &'static str,
    pub act: PAct,
}

/// A menu: its name and its entries, `None` for a rule between them.
pub struct MenuDef {
    pub tr: &'static str,
    pub en: &'static str,
    /// Which of the name's letters opens it, in each language.
    pub hot: (usize, usize),
    pub items: Vec<Option<MenuItem>>,
}

impl MenuDef {
    pub fn name(&self) -> &'static str {
        if lang::is_tr() { self.tr } else { self.en }
    }

    /// The letter that opens it, and where it is in the name.
    pub fn hot(&self) -> (usize, char) {
        let at = if lang::is_tr() {
            self.hot.0
        } else {
            self.hot.1
        };
        let c = self.name().chars().nth(at).unwrap_or(' ');
        (at, c.to_lowercase().next().unwrap_or(c))
    }
}

impl MenuItem {
    pub fn name(&self) -> &'static str {
        if lang::is_tr() { self.tr } else { self.en }
    }
}

/// The six menus, left to right.
pub fn menus() -> Vec<MenuDef> {
    use KeyCode::{Char, F};
    let (c, a, n) = (KeyModifiers::CONTROL, KeyModifiers::ALT, KeyModifiers::NONE);
    let k = |code, mods| PAct::Key(code, mods);
    let it = |tr, en, key, act| Some(MenuItem { tr, en, key, act });
    vec![
        MenuDef {
            tr: "Dosya",
            en: "File",
            hot: (0, 0),
            items: vec![
                it("Kaydet", "Save", "^S", k(Char('s'), c)),
                None,
                it("PDF derle", "Build the PDF", "F5", k(F(5), n)),
                it("PDF'i aç", "Open the PDF", "Alt+O", k(Char('o'), a)),
                it("Derleme logu", "The build log", "Alt+L", k(Char('l'), a)),
                None,
                it(
                    "Oyun bilgisi, taslak/final",
                    "Cover details, draft/final",
                    "Alt+İ",
                    k(Char('i'), a),
                ),
                it(
                    "Dağıtım ve oyuncu kopyaları",
                    "Casting and actors' copies",
                    "F9",
                    k(F(9), n),
                ),
                it(
                    "Notlu PDF kopyası",
                    "A PDF copy with notes",
                    "",
                    PAct::NotesPdf,
                ),
                it(
                    "Şablon olarak kaydet",
                    "Save as a template",
                    "",
                    PAct::SaveTemplate,
                ),
                None,
                it("Çık", "Quit", "^Q", k(Char('q'), c)),
            ],
        },
        MenuDef {
            tr: "Düzen",
            en: "Edit",
            hot: (1, 0),
            items: vec![
                it("Geri al", "Undo", "^Z", k(Char('z'), c)),
                it("Yinele", "Redo", "^Y", k(Char('y'), c)),
                None,
                it("Kes", "Cut", "^X", k(Char('x'), c)),
                it("Kopyala", "Copy", "^C", k(Char('c'), c)),
                it("Yapıştır", "Paste", "^V", k(Char('v'), c)),
                it("Hepsini seç", "Select all", "^A", k(Char('a'), c)),
                None,
                it("Bul ve değiştir", "Find and replace", "^F", k(Char('f'), c)),
                it(
                    "Karakter ya da komut ara",
                    "Search characters and commands",
                    "^K",
                    k(Char('k'), c),
                ),
                it(
                    "Yazım önerileri",
                    "Spelling suggestions",
                    "Alt+S",
                    k(Char('s'), a),
                ),
            ],
        },
        MenuDef {
            tr: "Görünüm",
            en: "View",
            hot: (0, 0),
            items: vec![
                it("Sayfa görünümü", "Page view", "F7", k(F(7), n)),
                it("Sonraki tema", "Next theme", "F6", k(F(6), n)),
                None,
                it("Daktilo modu", "Typewriter mode", "F3", k(F(3), n)),
                it("Sahne odağı", "Scene focus", "F4", k(F(4), n)),
                it("İmleç dışını soldur", "Dim the rest", "F8", k(F(8), n)),
                it(
                    "Rol görünümü",
                    "Role view",
                    "Shift+F8",
                    k(F(8), KeyModifiers::SHIFT),
                ),
                None,
                it(
                    "Bütün oyun kaynak olarak",
                    "The whole play as source",
                    "F2",
                    k(F(2), n),
                ),
                it(
                    "Kişi tanımları kaynakta",
                    "The cast lines as source",
                    "Alt+P",
                    k(Char('p'), a),
                ),
                it("Sahne şeridi", "The scene strip", "^G", k(Char('g'), c)),
                it("Animasyonlar", "Animations", "", PAct::Animations),
            ],
        },
        MenuDef {
            tr: "Oyun",
            en: "Play",
            hot: (0, 0),
            items: vec![
                it("Yeni sahne", "New scene", "#sahne", PAct::NewScene),
                it("Yeni perde", "New act", "#perde", PAct::NewAct),
                it("Yeni karakter", "New character", "", PAct::NewCharacter),
                None,
                it(
                    "Kişiler: düzenle, birleştir, böl",
                    "Cast: edit, merge, split",
                    "^O",
                    k(Char('o'), c),
                ),
                it(
                    "Pano: sahneler, kartlar",
                    "The board: scenes, cards",
                    "^G ^G",
                    PAct::Board,
                ),
                it(
                    "Dağıtım: oyuncular, tablo, sayılar",
                    "Casting: actors, table, numbers",
                    "F9",
                    k(F(9), n),
                ),
                None,
                it(
                    "Sahne özeti",
                    "The scene's summary",
                    "Alt+E",
                    k(Char('e'), a),
                ),
                it(
                    "Sahneyi imleçten böl",
                    "Split the scene at the cursor",
                    "Alt+B",
                    k(Char('b'), a),
                ),
                it(
                    "Sonraki sahneyle birleştir",
                    "Join with the next scene",
                    "Alt+J",
                    k(Char('j'), a),
                ),
                None,
                it(
                    "Ezber: rolün replikleri gizli",
                    "Memorize: the role's lines hidden",
                    "",
                    PAct::Memorize,
                ),
            ],
        },
        MenuDef {
            tr: "Notlar",
            en: "Notes",
            hot: (0, 0),
            items: vec![
                it("Notlar paneli", "The notes panel", "^N", k(Char('n'), c)),
                it("Buraya not", "A note here", "Alt+N", k(Char('n'), a)),
                None,
                it(
                    "Panodaki notları al",
                    "Take notes from the clipboard",
                    "",
                    PAct::NotesPaste,
                ),
                it(
                    "Dosyadan not al",
                    "Take notes from a file",
                    "",
                    PAct::NotesFile,
                ),
                None,
                it(
                    "Notlu PDF kopyası",
                    "A PDF copy with notes",
                    "",
                    PAct::NotesPdf,
                ),
            ],
        },
        MenuDef {
            tr: "Yardım",
            en: "Help",
            hot: (0, 0),
            items: vec![
                it("Tuşlar", "Keys", "F1", k(F(1), n)),
                it("Komut paleti", "Command palette", "^P", k(Char('p'), c)),
                None,
                it("Sürüm", "Version", "", PAct::About),
            ],
        },
    ]
}

/// F10 open: which menu, and which of its entries.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bar {
    pub menu: usize,
    pub sel: usize,
}

/// The first entry of a menu at or after `from`, going `step` (±1), over
/// the rules.
pub fn next_item(items: &[Option<MenuItem>], from: usize, step: isize) -> usize {
    let n = items.len() as isize;
    let mut i = from as isize;
    for _ in 0..n {
        if (0..n).contains(&i) && items[i as usize].is_some() {
            return i as usize;
        }
        i = (i + step).rem_euclid(n);
    }
    0
}

/// Where the menus' names sit on the header row: `(x, width)` each, after
/// a space. Their gaps are two columns.
pub fn title_spans(defs: &[MenuDef]) -> Vec<(u16, u16)> {
    let mut x = 1u16;
    defs.iter()
        .map(|d| {
            let w = d.name().width() as u16 + 2;
            let at = (x, w);
            x += w;
            at
        })
        .collect()
}

impl Writer {
    /// Whether an entry is on, for those that are on or off.
    pub(super) fn checked(&self, act: PAct) -> Option<bool> {
        use KeyCode::{Char, F};
        let alt = KeyModifiers::ALT;
        Some(match act {
            PAct::Animations => self.animations,
            PAct::Key(F(2), _) => self.raw_all,
            PAct::Key(F(3), _) => self.typewriter,
            PAct::Key(F(4), _) => self.focus,
            PAct::Key(F(7), _) => self.pages,
            PAct::Key(F(8), m) if m.contains(KeyModifiers::SHIFT) => self.role_view.is_some(),
            PAct::Key(F(8), _) => self.dim,
            PAct::Key(Char('g'), m) if m.contains(KeyModifiers::CONTROL) => self.strip,
            PAct::Key(Char('n'), m) if m.contains(KeyModifiers::CONTROL) => self.notes_view.open,
            PAct::Key(Char('p'), m) if m == alt => self.preamble_open,
            _ => return None,
        })
    }

    /// F10 or a click on a name: that menu, on its first entry.
    pub(super) fn open_bar(&mut self, menu: usize) {
        let defs = menus();
        let menu = menu.min(defs.len() - 1);
        self.popup = Popup::Bar(Bar {
            menu,
            sel: next_item(&defs[menu].items, 0, 1),
        });
    }

    /// The keys while a menu is open.
    pub(super) fn bar_key(&mut self, k: KeyEvent) {
        let Popup::Bar(bar) = &self.popup else { return };
        let bar = *bar;
        let defs = menus();
        let items = &defs[bar.menu].items;
        let m = defs.len();
        match k.code {
            KeyCode::Esc | KeyCode::F(10) => self.popup = Popup::None,
            KeyCode::Left => self.open_bar((bar.menu + m - 1) % m),
            KeyCode::Right | KeyCode::Tab => self.open_bar((bar.menu + 1) % m),
            KeyCode::Up => {
                let sel = next_item(items, (bar.sel + items.len() - 1) % items.len(), -1);
                self.popup = Popup::Bar(Bar { sel, ..bar });
            }
            KeyCode::Down => {
                let sel = next_item(items, (bar.sel + 1) % items.len(), 1);
                self.popup = Popup::Bar(Bar { sel, ..bar });
            }
            KeyCode::Home => {
                self.popup = Popup::Bar(Bar {
                    sel: next_item(items, 0, 1),
                    ..bar
                })
            }
            KeyCode::End => {
                self.popup = Popup::Bar(Bar {
                    sel: next_item(items, items.len() - 1, -1),
                    ..bar
                })
            }
            KeyCode::Enter | KeyCode::Char(' ') => self.choose_bar(bar.menu, bar.sel),
            // A menu's first letter opens it.
            KeyCode::Char(ch) => {
                let ch = ch.to_lowercase().next().unwrap_or(ch);
                if let Some(i) = defs.iter().position(|d| d.hot().1 == ch) {
                    self.open_bar(i);
                }
            }
            _ => {}
        }
    }

    /// Runs entry `sel` of menu `menu`, the menu closed first.
    pub(super) fn choose_bar(&mut self, menu: usize, sel: usize) {
        let act = menus()
            .get(menu)
            .and_then(|d| d.items.get(sel))
            .and_then(|i| i.as_ref())
            .map(|i| i.act);
        self.popup = Popup::None;
        if let Some(act) = act {
            self.run(act);
        }
    }

    /// The header row's menu names, with the open one lit.
    pub(super) fn bar_titles(&self) -> Vec<Span<'static>> {
        let open = match self.popup {
            Popup::Bar(b) => Some(b.menu),
            _ => None,
        };
        let mut out = vec![Span::raw(" ")];
        for (i, d) in menus().iter().enumerate() {
            let name = d.name();
            let at = d.hot().0;
            let before: String = name.chars().take(at).collect();
            let head: String = name.chars().skip(at).take(1).collect();
            let rest: String = name.chars().skip(at + 1).collect();
            let style = if open == Some(i) {
                Style::default().fg(BEIGE).bg(SELECTION)
            } else {
                Style::default().fg(MUTED)
            };
            out.push(Span::styled(format!(" {before}"), style));
            out.push(Span::styled(head, style.add_modifier(Modifier::UNDERLINED)));
            out.push(Span::styled(format!("{rest} "), style));
        }
        out
    }

    /// The open menu, dropped from its name.
    pub(super) fn draw_bar(&mut self, f: &mut Frame, area: Rect, bar: Bar) {
        let defs = menus();
        let def = &defs[bar.menu];
        let (x, _) = title_spans(&defs)[bar.menu];
        let name_w = def
            .items
            .iter()
            .flatten()
            .map(|i| i.name().width())
            .max()
            .unwrap_or(0);
        let key_w = def
            .items
            .iter()
            .flatten()
            .map(|i| i.key.width())
            .max()
            .unwrap_or(0);
        // ✓, the name, a gap, the key, inside the borders.
        let w = (2 + name_w + 3 + key_w + 2 + 2) as u16;
        let w = w.min(area.width);
        let h = (def.items.len() as u16 + 2).min(area.height.saturating_sub(1));
        let r = Rect {
            x: (area.x + x).min(area.x + area.width - w),
            y: area.y + 1,
            width: w,
            height: h,
        };
        popup::float(f, r);
        f.render_widget(
            ratatui::widgets::Block::default()
                .borders(ratatui::widgets::Borders::ALL)
                .border_type(ratatui::widgets::BorderType::Rounded)
                .border_style(Style::default().fg(CLARET))
                .style(theme::base()),
            r,
        );
        let inn = popup::inner(r);
        self.bar_hits.clear();
        let lines: Vec<Line> = def
            .items
            .iter()
            .enumerate()
            .take(inn.height as usize)
            .map(|(n, item)| {
                let Some(item) = item else {
                    return Line::from(Span::styled(
                        "─".repeat(inn.width as usize),
                        Style::default().fg(FAINT),
                    ));
                };
                self.bar_hits.push((inn.y + n as u16, n));
                let is_sel = n == bar.sel;
                let tick = match self.checked(item.act) {
                    Some(true) => "✓ ",
                    _ => "  ",
                };
                let room = (inn.width as usize).saturating_sub(2 + key_w + 2);
                let name = popup::fit(item.name(), room);
                let gap =
                    (inn.width as usize).saturating_sub(2 + name.width() + item.key.width() + 1);
                popup::row(
                    vec![
                        Span::styled(tick, Style::default().fg(CLARET)),
                        Span::styled(name, Style::default().fg(if is_sel { BEIGE } else { FG })),
                        Span::raw(" ".repeat(gap)),
                        Span::styled(item.key.to_string(), Style::default().fg(MUTED)),
                        Span::raw(" "),
                    ],
                    is_sel,
                    inn.width,
                )
            })
            .collect();
        self.bar_rect = r;
        f.render_widget(Paragraph::new(lines).style(theme::base()), inn);
    }

    /// A click on the header row or in the open menu; true when it was one.
    pub(super) fn bar_mouse(&mut self, m: MouseEvent) -> bool {
        let MouseEventKind::Down(MouseButton::Left) = m.kind else {
            return matches!(self.popup, Popup::Bar(_)) && !matches!(m.kind, MouseEventKind::Moved);
        };
        if m.row == 0 {
            let defs = menus();
            let hit = title_spans(&defs)
                .iter()
                .position(|&(x, w)| m.column >= x && m.column < x + w);
            let open = match self.popup {
                Popup::Bar(b) => Some(b.menu),
                _ => None,
            };
            match (hit, open) {
                (Some(i), Some(o)) if o == i => self.popup = Popup::None,
                (Some(i), _) => self.open_bar(i),
                (None, Some(_)) => self.popup = Popup::None,
                (None, None) => return false,
            }
            return true;
        }
        let Popup::Bar(bar) = &self.popup else {
            return false;
        };
        let bar = *bar;
        let r = self.bar_rect;
        let inside =
            m.column >= r.x && m.column < r.x + r.width && m.row >= r.y && m.row < r.y + r.height;
        match self.bar_hits.iter().find(|(y, _)| *y == m.row) {
            Some(&(_, sel)) if inside => self.choose_bar(bar.menu, sel),
            _ if inside => {}
            _ => self.popup = Popup::None,
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_menu_has_entries_and_its_own_letter() {
        for tr in [true, false] {
            lang::set(if tr { lang::Lang::Tr } else { lang::Lang::En });
            let defs = menus();
            assert_eq!(defs.len(), 6);
            let mut firsts: Vec<char> = defs.iter().map(|d| d.hot().1).collect();
            firsts.sort();
            firsts.dedup();
            assert_eq!(firsts.len(), 6, "two menus share a letter");
            for d in &defs {
                assert!(d.items.first().is_some_and(|i| i.is_some()), "{}", d.name());
                assert!(d.items.last().is_some_and(|i| i.is_some()), "{}", d.name());
                // Keys the writer has are named.
                for i in d.items.iter().flatten() {
                    if matches!(i.act, PAct::Key(..)) {
                        assert!(!i.key.is_empty(), "{}", i.name());
                    }
                }
            }
        }
    }

    #[test]
    fn the_palette_has_all_the_menus_have() {
        let items = palette_items("");
        for d in menus() {
            for i in d.items.iter().flatten() {
                assert!(items.iter().any(|p| p.act == i.act), "{}", i.name());
            }
        }
    }

    #[test]
    fn moving_skips_the_rules() {
        let items = &menus()[0].items;
        assert!(items[1].is_none());
        assert_eq!(next_item(items, 1, 1), 2);
        assert_eq!(next_item(items, 1, -1), 0);
    }
}
