//! The ribbon along the top, as a word processor has it: tabs (Dosya,
//! Giriş, Ekle, Görünüm, Oyun, Notlar, Yardım), and under the chosen one its
//! buttons in groups. F10 shows a letter on each tab and then on each
//! button; the arrows work too, and the mouse. ^F1 folds it to its tabs.
//! Dosya opens a page of its own (`Popup::Backstage`). The palette (^P)
//! offers the same buttons, so the two never drift apart.

use super::*;
use crate::tui::theme::{FG, SELECTION};
use unicode_width::UnicodeWidthStr;

/// The rows the ribbon takes, open: tabs, two of buttons, the groups'
/// names, a rule.
pub const OPEN_H: u16 = 5;

pub struct Btn {
    pub tr: &'static str,
    pub en: &'static str,
    pub icon: &'static str,
    /// The key that does the same, if any.
    pub key: &'static str,
    pub act: PAct,
}

pub struct Group {
    pub tr: &'static str,
    pub en: &'static str,
    pub btns: Vec<Btn>,
}

pub struct Tab {
    pub tr: &'static str,
    pub en: &'static str,
    /// The letter F10 shows on it, in each language.
    pub tip: (char, char),
    /// None for Dosya, which opens a page instead.
    pub groups: Vec<Group>,
}

fn l(tr: &'static str, en: &'static str) -> &'static str {
    if lang::is_tr() { tr } else { en }
}

impl Btn {
    pub fn name(&self) -> &'static str {
        l(self.tr, self.en)
    }
}
impl Group {
    pub fn name(&self) -> &'static str {
        l(self.tr, self.en)
    }
}
impl Tab {
    pub fn name(&self) -> &'static str {
        l(self.tr, self.en)
    }
    pub fn tip(&self) -> char {
        if lang::is_tr() {
            self.tip.0
        } else {
            self.tip.1
        }
    }
    /// Its buttons in order, group after group.
    pub fn btns(&self) -> impl Iterator<Item = &Btn> {
        self.groups.iter().flat_map(|g| g.btns.iter())
    }
}

/// The tab Dosya: a page, not buttons.
pub const FILE_TAB: usize = 0;

/// The tabs, left to right.
pub fn tabs() -> Vec<Tab> {
    use KeyCode::{Char, F};
    let (c, a, n) = (KeyModifiers::CONTROL, KeyModifiers::ALT, KeyModifiers::NONE);
    let k = |code, mods| PAct::Key(code, mods);
    let b = |tr, en, icon, key, act| Btn {
        tr,
        en,
        icon,
        key,
        act,
    };
    let g = |tr, en, btns| Group { tr, en, btns };
    vec![
        Tab {
            tr: "Dosya",
            en: "File",
            tip: ('D', 'F'),
            groups: Vec::new(),
        },
        Tab {
            tr: "Giriş",
            en: "Home",
            tip: ('G', 'H'),
            groups: vec![
                g(
                    "Kayıt",
                    "Save",
                    vec![
                        b("Kaydet", "Save", "▣", "^S", k(Char('s'), c)),
                        b("PDF derle", "Build PDF", "⎙", "F5", k(F(5), n)),
                    ],
                ),
                g(
                    "Geri al",
                    "Undo",
                    vec![
                        b("Geri al", "Undo", "↶", "^Z", k(Char('z'), c)),
                        b("Yinele", "Redo", "↷", "^Y", k(Char('y'), c)),
                    ],
                ),
                g(
                    "Pano",
                    "Clipboard",
                    vec![
                        b("Yapıştır", "Paste", "⎗", "^V", k(Char('v'), c)),
                        b("Hepsini seç", "Select all", "▤", "^A", k(Char('a'), c)),
                        b("Kes", "Cut", "✂", "^X", k(Char('x'), c)),
                        b("Kopyala", "Copy", "⎘", "^C", k(Char('c'), c)),
                    ],
                ),
                g(
                    "Düzenleme",
                    "Editing",
                    vec![
                        b("Bul", "Find", "⌕", "^F", k(Char('f'), c)),
                        b("Yazım", "Spelling", "✓", "Alt+S", k(Char('s'), a)),
                        b("Ara", "Search", "⌘", "^K", k(Char('k'), c)),
                        b("Komutlar", "Commands", "≡", "^P", k(Char('p'), c)),
                    ],
                ),
            ],
        },
        Tab {
            tr: "Ekle",
            en: "Insert",
            tip: ('E', 'N'),
            groups: vec![
                g(
                    "Yapı",
                    "Structure",
                    vec![
                        b("Perde", "Act", "▬", "#perde", PAct::NewAct),
                        b("Sahne", "Scene", "§", "#sahne", PAct::NewScene),
                        b(
                            "Önsahne",
                            "Prologue",
                            "◇",
                            "#önsahne",
                            PAct::Insert("#önsahne ‸"),
                        ),
                    ],
                ),
                g(
                    "Kişi",
                    "People",
                    vec![
                        b("Karakter", "Character", "☺", "", PAct::NewCharacter),
                        b("Kişiler", "Cast", "⁂", "^O", k(Char('o'), c)),
                    ],
                ),
                g(
                    "Yönerge",
                    "Directions",
                    vec![
                        b("Eylem", "Action", "⁋", "", PAct::Insert("\\action{‸}")),
                        b(
                            "Sahne yönergesi",
                            "Stage direction",
                            "⌖",
                            "",
                            PAct::Insert("\\stagedir{‸}"),
                        ),
                        b(
                            "Replik içi",
                            "Inside a line",
                            "❨",
                            "((",
                            PAct::InsertInline("\\ic{‸}"),
                        ),
                        b("Es", "Pause", "⋯", "", PAct::Insert("\\beat‸")),
                        b(
                            "Dipnot",
                            "Footnote",
                            "†",
                            "",
                            PAct::InsertInline("\\dipnot{‸}"),
                        ),
                    ],
                ),
                g(
                    "Bölüm",
                    "Sections",
                    vec![
                        b(
                            "Önsöz",
                            "Preface",
                            "¶",
                            "#önsöz",
                            PAct::Insert("#önsöz\n\n‸"),
                        ),
                        b(
                            "Rejisöre notlar",
                            "Director's notes",
                            "✎",
                            "#notlar",
                            PAct::Insert("#notlar\n\n‸"),
                        ),
                    ],
                ),
            ],
        },
        Tab {
            tr: "Görünüm",
            en: "View",
            tip: ('R', 'W'),
            groups: vec![
                g(
                    "Görünümler",
                    "Views",
                    vec![
                        b("Sayfa", "Pages", "▯", "F7", k(F(7), n)),
                        b("Kaynak", "Source", "⟨⟩", "F2", k(F(2), n)),
                        b("PDF", "PDF", "⎙", "F12", k(F(12), n)),
                        b(
                            "Kişi tanımları",
                            "Cast lines",
                            "⊞",
                            "Alt+P",
                            k(Char('p'), a),
                        ),
                    ],
                ),
                g(
                    "Odak",
                    "Focus",
                    vec![
                        b("Daktilo", "Typewriter", "⌨", "F3", k(F(3), n)),
                        b("Sahne odağı", "Scene focus", "◉", "F4", k(F(4), n)),
                        b("Soldur", "Dim", "◐", "F8", k(F(8), n)),
                        b("Rol", "Role", "◑", "Shift+F8", k(F(8), KeyModifiers::SHIFT)),
                    ],
                ),
                g(
                    "Yanda",
                    "Beside",
                    vec![
                        b("Sahne şeridi", "Scene strip", "▥", "^G", k(Char('g'), c)),
                        b("Notlar", "Notes", "✎", "^N", k(Char('n'), c)),
                    ],
                ),
                g(
                    "Görünüş",
                    "Look",
                    vec![
                        b("Tema", "Theme", "◈", "F6", k(F(6), n)),
                        b("Animasyon", "Animations", "≈", "", PAct::Animations),
                        b("Şeridi katla", "Fold ribbon", "⌃", "^F1", PAct::FoldRibbon),
                    ],
                ),
            ],
        },
        Tab {
            tr: "Oyun",
            en: "Play",
            tip: ('O', 'P'),
            groups: vec![
                g(
                    "Kişiler",
                    "Cast",
                    vec![
                        b("Kişiler", "Cast", "⁂", "^O", k(Char('o'), c)),
                        b(
                            "Yeni karakter",
                            "New character",
                            "☺",
                            "",
                            PAct::NewCharacter,
                        ),
                    ],
                ),
                g(
                    "Sahneler",
                    "Scenes",
                    vec![
                        b("Pano", "Board", "▦", "^G ^G", PAct::Board),
                        b("Özet", "Summary", "≣", "Alt+E", k(Char('e'), a)),
                        b("Böl", "Split", "⫶", "Alt+B", k(Char('b'), a)),
                        b("Birleştir", "Join", "⋈", "Alt+J", k(Char('j'), a)),
                    ],
                ),
                g(
                    "Prova",
                    "Rehearsal",
                    vec![
                        b("Dağıtım", "Casting", "☍", "F9", k(F(9), n)),
                        b("Ezber", "Memorize", "◌", "", PAct::Memorize),
                    ],
                ),
                g(
                    "Kapak",
                    "Cover",
                    vec![b(
                        "Oyun bilgisi",
                        "Cover details",
                        "ⓘ",
                        "Alt+İ",
                        k(Char('i'), a),
                    )],
                ),
            ],
        },
        Tab {
            tr: "Notlar",
            en: "Notes",
            tip: ('N', 'O'),
            groups: vec![
                g(
                    "Notlar",
                    "Notes",
                    vec![
                        b("Panel", "Panel", "✎", "^N", k(Char('n'), c)),
                        b("Buraya not", "Note here", "✚", "Alt+N", k(Char('n'), a)),
                    ],
                ),
                g(
                    "Al",
                    "Take",
                    vec![
                        b("Panodan", "Clipboard", "⎗", "", PAct::NotesPaste),
                        b("Dosyadan", "From a file", "⇲", "", PAct::NotesFile),
                    ],
                ),
                g(
                    "Çıktı",
                    "Output",
                    vec![b("Notlu PDF", "PDF with notes", "⎙", "", PAct::NotesPdf)],
                ),
            ],
        },
        Tab {
            tr: "Yardım",
            en: "Help",
            tip: ('Y', 'Y'),
            groups: vec![g(
                "Yardım",
                "Help",
                vec![
                    b("Tuşlar", "Keys", "⌨", "F1", k(F(1), n)),
                    b("Komutlar", "Commands", "≡", "^P", k(Char('p'), c)),
                    b("Sürüm", "Version", "ⓘ", "", PAct::About),
                ],
            )],
        },
    ]
}

/// The page Dosya opens: its entries.
pub fn backstage() -> Vec<(&'static str, &'static str, &'static str, PAct)> {
    use KeyCode::{Char, F};
    let (c, a, n) = (KeyModifiers::CONTROL, KeyModifiers::ALT, KeyModifiers::NONE);
    let k = |code, mods| PAct::Key(code, mods);
    vec![
        ("Bilgi", "Info", "", PAct::Info),
        ("Kaydet", "Save", "^S", k(Char('s'), c)),
        ("PDF derle", "Build the PDF", "F5", k(F(5), n)),
        ("PDF'i aç", "Open the PDF", "Alt+O", k(Char('o'), a)),
        ("Derleme logu", "The build log", "Alt+L", k(Char('l'), a)),
        (
            "Dağıtım ve oyuncu kopyaları",
            "Casting and actors' copies",
            "F9",
            k(F(9), n),
        ),
        (
            "Notlu PDF kopyası",
            "A PDF copy with notes",
            "",
            PAct::NotesPdf,
        ),
        (
            "Şablon olarak kaydet",
            "Save as a template",
            "",
            PAct::SaveTemplate,
        ),
        ("Oyun bilgisi", "Cover details", "Alt+İ", k(Char('i'), a)),
        ("Oyun ayarları", "Play settings", "", PAct::PlaySettings),
        ("Kapat", "Close", "^Q", k(Char('q'), c)),
    ]
}

/// Each button's F10 letter on a tab: the first of its name's letters no
/// other button took, else a digit.
pub fn tips(tab: &Tab) -> Vec<char> {
    let mut used: Vec<char> = Vec::new();
    let mut out = Vec::new();
    for b in tab.btns() {
        let pick = b
            .name()
            .chars()
            .filter(|c| c.is_alphanumeric())
            .flat_map(char::to_lowercase)
            .find(|c| !used.contains(c))
            .or_else(|| ('1'..='9').find(|c| !used.contains(c)))
            .unwrap_or('0');
        used.push(pick);
        out.push(pick);
    }
    out
}

/// F10, the keys going to the ribbon: first the tabs' letters, then the
/// buttons' of the chosen tab; `sel` is a button chosen with the arrows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Nav {
    pub on_btns: bool,
    pub sel: Option<usize>,
    /// Opened over the text by a click while the ribbon is folded: no
    /// letters shown.
    pub by_mouse: bool,
}

/// What a click on the ribbon hit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Hit {
    Tab(usize),
    Btn(usize),
    Act(PAct),
    Fold,
}

/// A button laid out: where, and its index on the tab.
struct Placed {
    x: u16,
    y: u16,
    w: u16,
    i: usize,
}

/// A group laid out: its buttons, its span and the name under it.
struct GroupLay {
    x: u16,
    w: u16,
    name: String,
    btns: Vec<Placed>,
    /// Icons only.
    compact: bool,
}

/// The buttons in columns of two, groups side by side from `x0`, in
/// `width`; groups from the right lose their names when it is tight.
fn lay_out(tab: &Tab, x0: u16, y: u16, width: u16) -> Vec<GroupLay> {
    let cell = |b: &Btn, compact: bool| {
        if compact {
            b.icon.width() as u16
        } else {
            (b.icon.width() + 1 + b.name().width()) as u16
        }
    };
    let group_w = |g: &Group, compact: bool| -> u16 {
        let cols: u16 = g
            .btns
            .chunks(2)
            .map(|col| col.iter().map(|b| cell(b, compact)).max().unwrap_or(0))
            .sum();
        let gaps = (g.btns.len().div_ceil(2) as u16).saturating_sub(1) * 2;
        (cols + gaps).max(g.name().width() as u16) + 2
    };
    let mut compact = vec![false; tab.groups.len()];
    let total = |compact: &[bool]| -> u16 {
        tab.groups
            .iter()
            .zip(compact)
            .map(|(g, &c)| group_w(g, c) + 1)
            .sum()
    };
    for i in (0..tab.groups.len()).rev() {
        if total(&compact) <= width {
            break;
        }
        compact[i] = true;
    }
    let mut out = Vec::new();
    let mut x = x0;
    let mut i = 0;
    for (g, &c) in tab.groups.iter().zip(&compact) {
        let w = group_w(g, c);
        if x + w > x0 + width {
            break;
        }
        let mut bx = x + 1;
        let mut btns = Vec::new();
        for col in g.btns.chunks(2) {
            let cw = col.iter().map(|b| cell(b, c)).max().unwrap_or(0);
            for (r, _) in col.iter().enumerate() {
                btns.push(Placed {
                    x: bx,
                    y: y + r as u16,
                    w: cw,
                    i,
                });
                i += 1;
            }
            bx += cw + 2;
        }
        out.push(GroupLay {
            x,
            w,
            name: g.name().to_string(),
            btns,
            compact: c,
        });
        x += w + 1;
    }
    out
}

impl Writer {
    /// The rows above the text: the ribbon, or its tabs alone.
    pub(super) fn top_h(&self) -> u16 {
        if self.ribbon_folded { 1 } else { OPEN_H }
    }

    /// Whether a button is on, for those that are on or off.
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

    /// F10: the tabs' letters.
    pub(super) fn open_ribbon(&mut self) {
        self.popup = Popup::Ribbon(Nav {
            on_btns: false,
            sel: None,
            by_mouse: false,
        });
    }

    /// A tab chosen: Dosya opens its page, the others show their buttons.
    pub(super) fn pick_tab(&mut self, i: usize, nav: Option<Nav>) {
        if i == FILE_TAB {
            self.popup = Popup::Backstage(1);
            return;
        }
        self.ribbon_tab = i;
        self.popup = match nav {
            Some(n) => Popup::Ribbon(n),
            None => Popup::None,
        };
    }

    /// Runs a button, the ribbon's keys given back to the text first.
    pub(super) fn press(&mut self, act: PAct) {
        self.popup = Popup::None;
        self.run(act);
    }

    pub(super) fn ribbon_key(&mut self, k: KeyEvent) {
        let Popup::Ribbon(nav) = &self.popup else {
            return;
        };
        let nav = *nav;
        let tabs = tabs();
        let tab = &tabs[self.ribbon_tab];
        let n = tab.btns().count();
        let letter = match k.code {
            KeyCode::Char(c) => c.to_lowercase().next(),
            _ => None,
        };
        if !nav.on_btns {
            match k.code {
                KeyCode::Esc | KeyCode::F(10) => self.popup = Popup::None,
                KeyCode::Left => {
                    self.ribbon_tab = (self.ribbon_tab + tabs.len() - 1) % tabs.len();
                    if self.ribbon_tab == FILE_TAB {
                        self.ribbon_tab = tabs.len() - 1;
                    }
                }
                KeyCode::Right => {
                    self.ribbon_tab = (self.ribbon_tab + 1) % tabs.len();
                    if self.ribbon_tab == FILE_TAB {
                        self.ribbon_tab = 1;
                    }
                }
                KeyCode::Down | KeyCode::Enter => {
                    self.popup = Popup::Ribbon(Nav {
                        on_btns: true,
                        sel: Some(0),
                        ..nav
                    });
                    self.tell_btn(0);
                }
                _ => {
                    if let Some(i) = tabs
                        .iter()
                        .position(|t| t.tip().to_lowercase().next() == letter)
                    {
                        self.pick_tab(
                            i,
                            Some(Nav {
                                on_btns: true,
                                ..nav
                            }),
                        );
                    }
                }
            }
            return;
        }
        let sel = nav.sel.unwrap_or(0).min(n.saturating_sub(1));
        let go = |s: usize| {
            Popup::Ribbon(Nav {
                sel: Some(s),
                ..nav
            })
        };
        match k.code {
            KeyCode::Esc => {
                self.popup = if nav.by_mouse {
                    Popup::None
                } else {
                    Popup::Ribbon(Nav {
                        on_btns: false,
                        sel: None,
                        ..nav
                    })
                }
            }
            KeyCode::F(10) => self.popup = Popup::None,
            KeyCode::Up => {
                self.popup = go(sel.saturating_sub(1));
                self.tell_btn(sel.saturating_sub(1));
            }
            KeyCode::Down => {
                let s = (sel + 1).min(n - 1);
                self.popup = go(s);
                self.tell_btn(s);
            }
            KeyCode::Left => {
                let s = sel.saturating_sub(2);
                self.popup = go(s);
                self.tell_btn(s);
            }
            KeyCode::Right => {
                let s = (sel + 2).min(n - 1);
                self.popup = go(s);
                self.tell_btn(s);
            }
            KeyCode::Enter => {
                if let Some(b) = tab.btns().nth(sel) {
                    self.press(b.act);
                }
            }
            _ => {
                let tips = tips(tab);
                if let Some(i) = tips.iter().position(|t| Some(*t) == letter)
                    && let Some(b) = tab.btns().nth(i)
                {
                    self.press(b.act);
                }
            }
        }
    }

    /// The chosen button's name and key on the status line.
    fn tell_btn(&mut self, i: usize) {
        let tabs = tabs();
        if let Some(b) = tabs[self.ribbon_tab].btns().nth(i) {
            let msg = if b.key.is_empty() {
                b.name().to_string()
            } else {
                format!("{} — {}", b.name(), b.key)
            };
            self.say(&msg, false);
        }
    }

    /// Folds the ribbon to its tabs, or opens it again.
    pub(super) fn fold_ribbon(&mut self) {
        self.ribbon_folded = !self.ribbon_folded;
        self.popup = Popup::None;
        self.relayout();
        self.keep_visible();
    }

    /// The tabs' row, and under it the chosen tab's buttons unless folded
    /// (or over the text, `over`, when opened from folded).
    pub(super) fn draw_ribbon(&mut self, f: &mut Frame, area: Rect, over: bool) {
        self.ribbon_hits.clear();
        let nav = match &self.popup {
            Popup::Ribbon(n) => Some(*n),
            _ => None,
        };
        let tips_on = nav.is_some_and(|n| !n.by_mouse);
        let tabs = tabs();
        let faint = Style::default().fg(FAINT);
        let muted = Style::default().fg(MUTED);
        let row0 = Rect { height: 1, ..area };
        if !over {
            f.render_widget(
                ratatui::widgets::Block::default().style(theme::base()),
                Rect {
                    height: if self.ribbon_folded { 1 } else { OPEN_H },
                    ..area
                },
            );
            // Quick access: save, undo, redo.
            let mut x = area.x + 1;
            let mut spans: Vec<Span> = vec![Span::raw(" ")];
            let (c, ctrl) = (KeyCode::Char, KeyModifiers::CONTROL);
            for (icon, act) in [
                ("▣", PAct::Key(c('s'), ctrl)),
                ("↶", PAct::Key(c('z'), ctrl)),
                ("↷", PAct::Key(c('y'), ctrl)),
            ] {
                spans.push(Span::styled(format!("{icon} "), muted));
                self.ribbon_hits.push((
                    Rect {
                        x,
                        y: area.y,
                        width: 2,
                        height: 1,
                    },
                    Hit::Act(act),
                ));
                x += 2;
            }
            spans.push(Span::styled("│ ", faint));
            x += 2;
            for (i, t) in tabs.iter().enumerate() {
                let name = t.name();
                let w = name.width() as u16 + 2;
                let on = i == self.ribbon_tab;
                let style = if i == FILE_TAB {
                    Style::default().fg(CLARET).add_modifier(Modifier::BOLD)
                } else if on {
                    Style::default()
                        .fg(BEIGE)
                        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
                } else {
                    muted
                };
                spans.push(Span::raw(" "));
                spans.push(Span::styled(name.to_string(), style));
                spans.push(Span::raw(" "));
                self.ribbon_hits.push((
                    Rect {
                        x,
                        y: area.y,
                        width: w,
                        height: 1,
                    },
                    Hit::Tab(i),
                ));
                x += w;
            }
            // Right: the play's title and state, then the command search.
            let state = self.state_span();
            let search = lang::l(" ⌕ Komut ara  ^P ", " ⌕ Commands  ^P ");
            let search_w = search.width() as u16;
            let right_w = state.content.width() as u16 + 2;
            let used = x - area.x;
            let room = area.width.saturating_sub(used + right_w + search_w + 3);
            let title = self.play_title();
            if room > 6 {
                let t = popup::fit(&title, room as usize);
                let pad = room as usize - t.width();
                spans.push(Span::raw(" ".repeat(pad / 2 + 1)));
                spans.push(Span::styled(
                    t,
                    Style::default().fg(BEIGE).add_modifier(Modifier::BOLD),
                ));
                spans.push(Span::raw(" ".repeat(pad - pad / 2 + 1)));
            } else {
                spans.push(Span::raw(" ".repeat(room as usize + 2)));
            }
            if area.width > used + right_w + search_w {
                spans.push(state);
                spans.push(Span::raw("  "));
                let sx = area.x + area.width - search_w - 1;
                spans.push(Span::styled(
                    search,
                    Style::default().fg(MUTED).bg(SELECTION),
                ));
                self.ribbon_hits.push((
                    Rect {
                        x: sx,
                        y: area.y,
                        width: search_w,
                        height: 1,
                    },
                    Hit::Act(PAct::Key(KeyCode::Char('p'), ctrl)),
                ));
            }
            f.render_widget(Paragraph::new(Line::from(spans)).style(theme::base()), row0);
            // F10: a letter on each tab.
            if tips_on && !nav.is_some_and(|n| n.on_btns) {
                for (r, h) in self.ribbon_hits.clone() {
                    if let Hit::Tab(i) = h {
                        badge(f, r.x, r.y, tabs[i].tip());
                    }
                }
            }
            if self.ribbon_folded {
                return;
            }
        }
        let tab = &tabs[self.ribbon_tab];
        let body = Rect {
            y: area.y + 1,
            height: OPEN_H - 1,
            ..area
        };
        if over {
            popup::float(f, body);
            f.render_widget(
                ratatui::widgets::Block::default().style(theme::base()),
                body,
            );
        }
        let groups = lay_out(tab, area.x + 1, body.y, area.width.saturating_sub(4));
        let btns: Vec<&Btn> = tab.btns().collect();
        let tips = tips(tab);
        let sel = nav.filter(|n| n.on_btns).and_then(|n| n.sel);
        let show_tips = tips_on && nav.is_some_and(|n| n.on_btns);
        let buf = f.buffer_mut();
        for g in &groups {
            for p in &g.btns {
                let b = btns[p.i];
                let on = self.checked(b.act) == Some(true);
                let chosen = sel == Some(p.i);
                let mut style = Style::default().fg(if on { BEIGE } else { FG });
                if chosen {
                    style = style.bg(SELECTION);
                }
                let icon_style =
                    Style::default()
                        .fg(if on { CLARET } else { MUTED })
                        .bg(if chosen {
                            SELECTION
                        } else {
                            theme::base().bg.unwrap_or_default()
                        });
                let label = if g.compact {
                    String::new()
                } else {
                    format!(" {}", b.name())
                };
                let pad = (p.w as usize).saturating_sub(b.icon.width() + label.width());
                buf.set_string(p.x, p.y, b.icon, icon_style);
                buf.set_string(
                    p.x + b.icon.width() as u16,
                    p.y,
                    format!("{label}{}", " ".repeat(pad)),
                    style.add_modifier(if on {
                        Modifier::BOLD
                    } else {
                        Modifier::empty()
                    }),
                );
                if show_tips {
                    let t: String = tips[p.i].to_uppercase().collect();
                    buf.set_string(
                        p.x,
                        p.y,
                        format!("{t:1}"),
                        Style::default()
                            .fg(theme::base().bg.unwrap_or_default())
                            .bg(CLARET)
                            .add_modifier(Modifier::BOLD),
                    );
                }
                self.ribbon_hits.push((
                    Rect {
                        x: p.x,
                        y: p.y,
                        width: p.w,
                        height: 1,
                    },
                    Hit::Btn(p.i),
                ));
            }
            // The group's name under it, and a rule after it.
            let name = popup::fit(&g.name, g.w.saturating_sub(2) as usize);
            let nx = g.x + (g.w.saturating_sub(name.width() as u16)) / 2;
            buf.set_string(nx, body.y + 2, &name, faint);
            for dy in 0..3 {
                buf.set_string(g.x + g.w, body.y + dy, "│", faint);
            }
        }
        // The rule under it all, and the fold mark at its right end.
        let rule_y = body.y + 3;
        buf.set_string(area.x, rule_y, "─".repeat(area.width as usize), faint);
        let fold = if over { " ⌄ " } else { " ⌃ " };
        let fx = area.x + area.width.saturating_sub(4);
        buf.set_string(fx, rule_y, fold, muted);
        self.ribbon_hits.push((
            Rect {
                x: fx,
                y: rule_y,
                width: 3,
                height: 1,
            },
            Hit::Fold,
        ));
    }

    fn play_title(&self) -> String {
        self.doc.title(&self.buf.text).unwrap_or_else(|| {
            self.path
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default()
        })
    }

    fn state_span(&self) -> Span<'static> {
        if self.read_only {
            Span::styled(
                lang::l("salt okunur", "read-only"),
                Style::default().fg(WARN),
            )
        } else if self.conflict {
            Span::styled(
                lang::l("dışarıda değişti", "changed elsewhere"),
                Style::default().fg(WARN),
            )
        } else if self.buf.dirty {
            Span::styled("•", Style::default().fg(MUTED))
        } else {
            Span::styled(lang::l("kaydedildi", "saved"), Style::default().fg(FAINT))
        }
    }

    /// A click on the ribbon; true when it was one.
    pub(super) fn ribbon_mouse(&mut self, m: MouseEvent) -> bool {
        let MouseEventKind::Down(MouseButton::Left) = m.kind else {
            return false;
        };
        let hit = self
            .ribbon_hits
            .iter()
            .find(|(r, _)| {
                m.column >= r.x
                    && m.column < r.x + r.width
                    && m.row >= r.y
                    && m.row < r.y + r.height
            })
            .map(|(_, h)| *h);
        let over = matches!(self.popup, Popup::Ribbon(_));
        match hit {
            Some(Hit::Tab(i)) => {
                if i == FILE_TAB {
                    self.pick_tab(i, None);
                } else if self.ribbon_folded {
                    // Folded: the buttons drop over the text, once.
                    self.ribbon_tab = i;
                    self.popup = Popup::Ribbon(Nav {
                        on_btns: true,
                        sel: None,
                        by_mouse: true,
                    });
                } else if i == self.ribbon_tab {
                    // The chosen tab again folds it, as a double click does.
                    self.fold_ribbon();
                } else {
                    self.pick_tab(i, None);
                }
                true
            }
            Some(Hit::Btn(i)) => {
                let tabs = tabs();
                if let Some(b) = tabs[self.ribbon_tab].btns().nth(i) {
                    self.press(b.act);
                }
                true
            }
            Some(Hit::Act(a)) => {
                self.press(a);
                true
            }
            Some(Hit::Fold) => {
                self.fold_ribbon();
                true
            }
            None if over => {
                self.popup = Popup::None;
                true
            }
            None => m.row < self.text_top,
        }
    }

    /// What Dosya opens: the entries on the left, the chosen one's
    /// details on the right.
    pub(super) fn draw_backstage(&mut self, f: &mut Frame, area: Rect, sel: usize) {
        popup::float(f, area);
        f.render_widget(
            ratatui::widgets::Block::default().style(theme::base()),
            area,
        );
        let items = backstage();
        let col_w = 34u16.min(area.width / 2);
        let col = Rect {
            width: col_w,
            ..area
        };
        let band = Style::default().bg(SELECTION);
        f.render_widget(ratatui::widgets::Block::default().style(band), col);
        let mut lines = vec![
            Line::from(Span::styled(
                lang::l("  ← Esc  geri", "  ← Esc  back"),
                Style::default().fg(MUTED).bg(SELECTION),
            )),
            Line::from(""),
        ];
        self.ribbon_hits.clear();
        for (i, (tr, en, key, _)) in items.iter().enumerate() {
            let on = i == sel;
            let name = if lang::is_tr() { *tr } else { *en };
            let w = col_w as usize - 4;
            let gap = w.saturating_sub(name.width() + key.width());
            let style = if on {
                Style::default()
                    .fg(BEIGE)
                    .bg(CLARET)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(FG).bg(SELECTION)
            };
            lines.push(Line::from(vec![
                Span::styled("  ", style),
                Span::styled(name.to_string(), style),
                Span::styled(" ".repeat(gap), style),
                Span::styled(
                    key.to_string(),
                    if on {
                        style
                    } else {
                        Style::default().fg(MUTED).bg(SELECTION)
                    },
                ),
                Span::styled("  ", style),
            ]));
            self.ribbon_hits.push((
                Rect {
                    x: col.x,
                    y: area.y + 2 + i as u16,
                    width: col_w,
                    height: 1,
                },
                Hit::Btn(i),
            ));
        }
        f.render_widget(
            Paragraph::new(lines),
            Rect {
                y: area.y + 1,
                ..col
            },
        );
        // The right: what the chosen one is about.
        let right = Rect {
            x: area.x + col_w + 4,
            y: area.y + 2,
            width: area.width.saturating_sub(col_w + 8),
            height: area.height.saturating_sub(4),
        };
        let (tr, en, _, act) = items[sel.min(items.len() - 1)];
        let head = Line::from(Span::styled(
            (if lang::is_tr() { tr } else { en }).to_string(),
            Style::default().fg(BEIGE).add_modifier(Modifier::BOLD),
        ));
        let mut body = vec![head, Line::from("")];
        let row = |k: &str, v: String| {
            Line::from(vec![
                Span::styled(format!("{k:<18}"), Style::default().fg(MUTED)),
                Span::styled(v, Style::default().fg(FG)),
            ])
        };
        match act {
            PAct::Info => {
                let text = &self.buf.text;
                let o = &self.doc.outline;
                let wpm = self.wpm();
                let secs: u32 = o
                    .scenes
                    .iter()
                    .map(|s| dactile::timing::scene_secs(s, wpm))
                    .sum();
                let words: usize = o.scenes.iter().map(|s| s.words).sum();
                let get = |n: &str| dactile::info::get(text, n).unwrap_or_default();
                body.push(row(lang::l("Başlık", "Title"), self.play_title()));
                body.push(row(lang::l("Yazar", "Author"), get("author")));
                body.push(row(lang::l("Taslak", "Draft"), get("draft")));
                body.push(row(
                    lang::l("Baskı", "Mode"),
                    if dactile::info::is_final(text) {
                        "final".into()
                    } else {
                        lang::l("taslak", "draft").into()
                    },
                ));
                body.push(Line::from(""));
                body.push(row(
                    lang::l("Perde / sahne", "Acts / scenes"),
                    format!("{} / {}", o.acts.len(), o.scenes.len()),
                ));
                body.push(row(
                    lang::l("Kişi", "Characters"),
                    self.doc.cast.characters.len().to_string(),
                ));
                body.push(row(lang::l("Kelime", "Words"), words.to_string()));
                body.push(row(
                    lang::l("Süre", "Running time"),
                    format!("≈ {}", dactile::timing::format(secs)),
                ));
                body.push(Line::from(""));
                body.push(row(
                    lang::l("Dosya", "File"),
                    dactile::config::tilde(&self.path),
                ));
                let pdf = build::pdf_path(&self.path);
                let when = |p: &Path| {
                    dactile::file::modified(p).map_or_else(
                        || lang::l("yok", "none").to_string(),
                        |t| {
                            chrono::DateTime::<chrono::Local>::from(t)
                                .format("%d.%m.%Y %H:%M")
                                .to_string()
                        },
                    )
                };
                body.push(row(lang::l("Değişti", "Changed"), when(&self.path)));
                body.push(row("PDF", when(&pdf)));
            }
            _ => {
                let about = match act {
                    PAct::Key(KeyCode::Char('s'), _) => lang::l(
                        "Oyunu diske yazar. Dactile Light her değişiklikten sonra kendisi de kaydeder.",
                        "Writes the play to disk. Dactile Light also saves after each change.",
                    ),
                    PAct::Key(KeyCode::F(5), _) => lang::l(
                        "XeLaTeX ile PDF'i derler; bitince alt satırda söyler.",
                        "Builds the PDF with XeLaTeX; the status line says when it is done.",
                    ),
                    PAct::Key(KeyCode::F(9), _) => lang::l(
                        "Kim kimi oynuyor, sahne/rol tablosu; oradan her oyuncuya kendi kopyası.",
                        "Who plays whom, the scene/role table; from there each actor's own copy.",
                    ),
                    PAct::NotesPdf => lang::l(
                        "Açık notlar kenarda, hepsinin listesi sonda, ayrı bir PDF.",
                        "The open notes in the margin, a list of all at the end, a PDF of its own.",
                    ),
                    PAct::SaveTemplate => lang::l(
                        "Kapak, kişiler, perde ve sahneler kalır, replikler kalmaz; ana ekranda Yeni'de çıkar.",
                        "Keeps the cover, cast, acts and scenes, not the lines; it shows under New on the home screen.",
                    ),
                    PAct::PlaySettings => lang::l(
                        "Bu oyunun konuşma hızı, taslağı olduğu oyun ve sözlüğü.",
                        "This play's pace, the play it is a draft of, and its words.",
                    ),
                    PAct::Key(KeyCode::Char('q'), _) => lang::l(
                        "Kaydeder ve ana ekrana döner.",
                        "Saves and goes back to the home screen.",
                    ),
                    _ => "",
                };
                body.push(Line::from(Span::styled(about, Style::default().fg(FG))));
                body.push(Line::from(""));
                body.push(Line::from(Span::styled(
                    lang::l("⏎ yap", "⏎ do it"),
                    Style::default().fg(MUTED),
                )));
            }
        }
        f.render_widget(
            Paragraph::new(body)
                .wrap(ratatui::widgets::Wrap { trim: false })
                .style(theme::base()),
            right,
        );
    }

    pub(super) fn backstage_key(&mut self, k: KeyEvent) {
        let Popup::Backstage(sel) = self.popup else {
            return;
        };
        let n = backstage().len();
        match k.code {
            KeyCode::Esc | KeyCode::F(10) | KeyCode::Left => self.popup = Popup::None,
            KeyCode::Up => self.popup = Popup::Backstage(sel.saturating_sub(1)),
            KeyCode::Down => self.popup = Popup::Backstage((sel + 1).min(n - 1)),
            KeyCode::Enter => self.backstage_run(sel),
            _ => {}
        }
    }

    fn backstage_run(&mut self, sel: usize) {
        let act = backstage()[sel].3;
        if act != PAct::Info {
            self.press(act);
        }
    }

    pub(super) fn backstage_mouse(&mut self, m: MouseEvent) {
        let MouseEventKind::Down(MouseButton::Left) = m.kind else {
            return;
        };
        if m.row == 1 && m.column < 34 {
            self.popup = Popup::None;
            return;
        }
        let hit = self
            .ribbon_hits
            .iter()
            .find(|(r, _)| m.row == r.y && m.column >= r.x && m.column < r.x + r.width)
            .map(|(_, h)| *h);
        if let Some(Hit::Btn(i)) = hit {
            if matches!(self.popup, Popup::Backstage(s) if s == i) {
                self.backstage_run(i);
            } else {
                self.popup = Popup::Backstage(i);
            }
        }
    }

    /// A snippet put in: `‸` is where the cursor goes. A block snippet on
    /// a line of its own after the cursor's line, an inline one at the
    /// cursor.
    pub(super) fn insert_snippet(&mut self, s: &str, block: bool) {
        if !self.editable() {
            return;
        }
        let nl = source::line_ending(&self.buf.text);
        let (at, text) = if block {
            let end = self.buf.text[self.buf.cursor..]
                .find('\n')
                .map_or(self.buf.text.len(), |i| self.buf.cursor + i);
            let line_start = self.buf.text[..self.buf.cursor]
                .rfind('\n')
                .map_or(0, |i| i + 1);
            let empty = self.buf.text[line_start..end].trim().is_empty();
            if empty {
                (line_start, s.replace('\n', nl))
            } else {
                (end, format!("{nl}{nl}{}", s.replace('\n', nl)))
            }
        } else {
            (self.buf.cursor, s.to_string())
        };
        let mark = text.find('‸').unwrap_or(text.len());
        let text = text.replace('‸', "");
        let at = if block && self.buf.text[..at].trim_end().len() < self.doc.parts.body.start {
            // Never in the preamble: at the play's start.
            self.doc.parts.body.start
        } else {
            at
        };
        self.buf
            .apply(&[dactile::tex::Edit::new(at..at, text)], at + mark);
        self.edited();
    }
}

/// A letter on a dark band, as Word shows its key tips.
fn badge(f: &mut Frame, x: u16, y: u16, c: char) {
    let t: String = c.to_uppercase().collect();
    f.buffer_mut().set_string(
        x,
        y,
        t,
        Style::default()
            .fg(theme::base().bg.unwrap_or_default())
            .bg(CLARET)
            .add_modifier(Modifier::BOLD),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tabs_and_letters_are_their_own() {
        for tr in [true, false] {
            lang::set(if tr { lang::Lang::Tr } else { lang::Lang::En });
            let tabs = tabs();
            assert_eq!(tabs.len(), 7);
            let mut t: Vec<char> = tabs.iter().map(|t| t.tip()).collect();
            t.sort();
            t.dedup();
            assert_eq!(t.len(), 7, "two tabs share a letter");
            for tab in tabs.iter().skip(1) {
                assert!(!tab.groups.is_empty(), "{}", tab.name());
                let mut tips = tips(tab);
                let n = tips.len();
                tips.sort();
                tips.dedup();
                assert_eq!(
                    tips.len(),
                    n,
                    "two buttons share a letter on {}",
                    tab.name()
                );
                for b in tab.btns() {
                    if matches!(b.act, PAct::Key(..)) {
                        assert!(!b.key.is_empty(), "{}", b.name());
                    }
                }
            }
        }
    }

    #[test]
    fn the_palette_has_all_the_ribbon_has() {
        let items = palette_items("");
        for t in tabs() {
            for b in t.btns() {
                assert!(items.iter().any(|p| p.act == b.act), "{}", b.name());
            }
        }
        for (tr, _, _, act) in backstage() {
            assert!(
                act == PAct::Info || items.iter().any(|p| p.act == act),
                "{tr}"
            );
        }
    }

    #[test]
    fn a_tight_ribbon_keeps_its_icons() {
        lang::set(lang::Lang::Tr);
        let tabs = tabs();
        let wide = lay_out(&tabs[2], 0, 1, 200);
        assert!(wide.iter().all(|g| !g.compact));
        let tight = lay_out(&tabs[2], 0, 1, 60);
        assert!(tight.iter().any(|g| g.compact));
        let last = tight.last().unwrap();
        assert!(last.x + last.w <= 60);
    }
}
