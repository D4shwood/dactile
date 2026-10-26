//! Small windows over the text: the suggestion menu, the verb list, the
//! search window and the forms.

use super::theme::{self, BEIGE, CLARET, FAINT, FG, MUTED, SELECTION};
use dactile::complete::{self, Item};
use dactile::lang;
use ratatui::Frame;
use ratatui::layout::{Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};
use unicode_width::UnicodeWidthStr;

/// The background of a field being typed into.
const FIELD: Color = Color::Rgb(0x22, 0x22, 0x22);

/// A list to choose from, filtered by what is typed.
pub struct Menu {
    pub all: Vec<Item>,
    pub shown: Vec<Item>,
    pub sel: usize,
    /// Where the typed query starts in the text (after the backslash).
    pub from: usize,
    /// Headings after a `#`, not macros after a backslash.
    pub hash: bool,
}

impl Menu {
    pub fn new(all: Vec<Item>, from: usize) -> Menu {
        let shown = all.clone();
        Menu {
            all,
            shown,
            sel: 0,
            from,
            hash: false,
        }
    }

    pub fn filter(&mut self, query: &str) {
        self.shown = complete::filter(&self.all, query);
        self.sel = 0;
    }

    pub fn up(&mut self) {
        self.sel = self.sel.saturating_sub(1);
    }

    pub fn down(&mut self) {
        if self.sel + 1 < self.shown.len() {
            self.sel += 1;
        }
    }

    pub fn chosen(&self) -> Option<&Item> {
        self.shown.get(self.sel)
    }
}

/// The verbs offered after `\xdo `.
pub struct Verbs {
    pub all: Vec<String>,
    pub shown: Vec<String>,
    pub sel: usize,
    /// Where the direction's text starts.
    pub from: usize,
}

impl Verbs {
    pub fn new(all: Vec<String>, from: usize) -> Verbs {
        Verbs {
            shown: all.clone(),
            all,
            sel: 0,
            from,
        }
    }

    pub fn filter(&mut self, query: &str) {
        let q = dactile::cast::fold_ascii(query);
        self.shown = self
            .all
            .iter()
            .filter(|v| dactile::cast::fold_ascii(v).starts_with(&q))
            .cloned()
            .collect();
        self.sel = 0;
    }
}

/// A form of a few fields.
pub struct Form {
    pub kind: FormKind,
    pub title: String,
    pub fields: Vec<Field>,
    pub focus: usize,
    /// Where the result goes in the text.
    pub at: std::ops::Range<usize>,
    pub error: Option<String>,
    /// A change asked to be confirmed with a second ⏎ (renaming an alias).
    pub confirmed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormKind {
    Scene,
    Act,
    Character,
    /// Changing a character, by its alias before the change.
    EditCharacter(String),
    /// A character's card, by alias.
    Card(String),
    /// The cover details and draft/final.
    Info,
    Find,
    /// Home screen: a new play, opening a file, the settings.
    NewPlay,
    OpenPath,
    Settings,
    /// A copy of this play under a new title.
    Copy(std::path::PathBuf),
    /// A new draft of this play, next to it.
    Fork(std::path::PathBuf),
}

pub struct Field {
    pub label: String,
    pub value: String,
    /// Values to offer with Tab (earlier places, times…).
    pub options: Vec<String>,
    /// A yes/no field, toggled with Space.
    pub toggle: bool,
    /// Changed by hand: an automatic value (the alias) stops following.
    pub touched: bool,
}

impl Field {
    pub fn text(label: &str, value: &str, options: Vec<String>) -> Field {
        Field {
            label: label.into(),
            value: value.into(),
            options,
            toggle: false,
            touched: false,
        }
    }
    pub fn flag(label: &str, on: bool) -> Field {
        Field {
            label: label.into(),
            value: if on { "evet" } else { "hayır" }.into(),
            options: Vec::new(),
            toggle: true,
            touched: false,
        }
    }
    pub fn on(&self) -> bool {
        self.value == "evet"
    }
}

impl Form {
    pub fn get(&self, label: &str) -> &str {
        self.fields
            .iter()
            .find(|f| f.label == label)
            .map_or("", |f| f.value.trim())
    }

    pub fn field_mut(&mut self, label: &str) -> Option<&mut Field> {
        self.fields.iter_mut().find(|f| f.label == label)
    }

    /// Tab on a field with options: the next option that starts with what
    /// is typed (or the next one).
    pub fn complete(&mut self) -> bool {
        let f = &mut self.fields[self.focus];
        if f.options.is_empty() {
            return false;
        }
        let cur = f.value.clone();
        let pos = f.options.iter().position(|o| *o == cur);
        let next = match pos {
            Some(i) => f.options[(i + 1) % f.options.len()].clone(),
            None => f
                .options
                .iter()
                .find(|o| {
                    dactile::cast::fold_ascii(o).starts_with(&dactile::cast::fold_ascii(&cur))
                })
                .unwrap_or(&f.options[0])
                .clone(),
        };
        f.value = next;
        true
    }
}

pub fn panel(title: &str) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(CLARET))
        .title(Span::styled(
            format!(" {title} "),
            Style::default().fg(BEIGE),
        ))
        .style(theme::base())
}

/// Clears `r` for a window.
pub fn float(f: &mut Frame, r: Rect) {
    f.render_widget(Clear, r);
}

/// `s` cut to `w` columns, with an ellipsis when it doesn't fit.
pub fn fit(s: &str, w: usize) -> String {
    if s.width() <= w {
        return s.to_string();
    }
    let mut out = String::new();
    for c in s.chars() {
        if out.width() + unicode_width::UnicodeWidthChar::width(c).unwrap_or(0) + 1 > w {
            break;
        }
        out.push(c);
    }
    out.push('…');
    out
}

/// A row of a list `width` wide: the chosen one on a band of colour.
pub fn row(mut spans: Vec<Span<'static>>, sel: bool, width: u16) -> Line<'static> {
    if sel {
        let used: usize = spans.iter().map(|s| s.content.width()).sum();
        spans.push(Span::raw(" ".repeat((width as usize).saturating_sub(used))));
        for s in &mut spans {
            s.style = s.style.bg(SELECTION);
        }
    }
    Line::from(spans)
}

pub fn mark(sel: bool) -> Span<'static> {
    Span::styled(if sel { "▸ " } else { "  " }, Style::default().fg(CLARET))
}

/// Where a popup of `w`×`h` goes near the cursor, inside `area`.
fn near(area: Rect, at: Position, w: u16, h: u16) -> Rect {
    let w = w.min(area.width.saturating_sub(1));
    let h = h.min(area.height);
    let x =
        at.x.saturating_sub(2)
            .min(area.x + area.width - w - 1)
            .max(area.x);
    let below = at.y + 1;
    let y = if below + h < area.y + area.height {
        below
    } else {
        at.y.saturating_sub(h).max(area.y)
    };
    Rect {
        x,
        y,
        width: w,
        height: h,
    }
}

pub fn draw_menu(f: &mut Frame, area: Rect, at: Position, m: &Menu, query: &str) {
    let rows = m.shown.len().min(8) as u16;
    if rows == 0 {
        return;
    }
    let key_w = m
        .shown
        .iter()
        .map(|i| i.key.width())
        .max()
        .unwrap_or(4)
        .max(5)
        + 2;
    let label_w = m.shown.iter().map(|i| i.label.width()).max().unwrap_or(10);
    let w = (2 + key_w + label_w + 5).clamp(28, 74) as u16;
    let r = near(area, at, w, rows + 2);
    float(f, r);
    let title = format!("{}{query}", if m.hash { '#' } else { '\\' });
    let count = Line::from(Span::styled(
        format!(" {}/{} ", m.sel + 1, m.shown.len()),
        Style::default().fg(FAINT),
    ))
    .right_aligned();
    f.render_widget(panel(&title).title_bottom(count), r);
    let inner = inner(r);
    let room = (inner.width as usize).saturating_sub(2 + key_w);
    let start = m.sel.saturating_sub(rows as usize - 1);
    let lines: Vec<Line> = m
        .shown
        .iter()
        .enumerate()
        .skip(start)
        .take(rows as usize)
        .map(|(i, it)| {
            let sel = i == m.sel;
            let key = Style::default()
                .fg(if sel { BEIGE } else { FG })
                .add_modifier(if sel {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                });
            row(
                vec![
                    mark(sel),
                    Span::styled(format!("{:<key_w$}", it.key), key),
                    Span::styled(fit(&it.label, room), Style::default().fg(MUTED)),
                ],
                sel,
                inner.width,
            )
        })
        .collect();
    f.render_widget(Paragraph::new(lines).style(theme::base()), inner);
}

pub fn draw_verbs(f: &mut Frame, area: Rect, at: Position, v: &Verbs) {
    let rows = v.shown.len().min(6) as u16;
    if rows == 0 {
        return;
    }
    let w = v.shown.iter().map(|s| s.width() + 5).max().unwrap_or(10) as u16;
    let r = near(area, at, (w + 2).clamp(20, 42), rows + 2);
    float(f, r);
    f.render_widget(panel(lang::l("fiil", "verb")), r);
    let width = inner(r).width;
    let start = v.sel.saturating_sub(rows as usize - 1);
    let lines: Vec<Line> = v
        .shown
        .iter()
        .enumerate()
        .skip(start)
        .take(rows as usize)
        .map(|(i, s)| {
            let sel = i == v.sel;
            row(
                vec![
                    mark(sel),
                    Span::styled(
                        s.clone(),
                        Style::default().fg(if sel { BEIGE } else { MUTED }),
                    ),
                ],
                sel,
                width,
            )
        })
        .collect();
    f.render_widget(Paragraph::new(lines).style(theme::base()), inner(r));
}

/// The search window (^K): the same list, full height, with its query line.
pub fn draw_search(f: &mut Frame, area: Rect, m: &Menu, query: &str) {
    let w = area.width.saturating_sub(8).clamp(30, 80);
    let h = area.height.saturating_sub(4).max(6);
    let r = Rect {
        x: area.x + (area.width - w) / 2,
        y: area.y + 2,
        width: w,
        height: h,
    };
    float(f, r);
    f.render_widget(
        panel(lang::l(
            "ara — karakter, komut",
            "search — characters, commands",
        )),
        r,
    );
    let inner = inner(r);
    let q = Line::from(vec![
        Span::styled("› ", Style::default().fg(CLARET)),
        Span::styled(query.to_string(), Style::default().fg(FG)),
    ]);
    f.render_widget(
        Paragraph::new(q).style(theme::base().bg(FIELD)),
        Rect { height: 1, ..inner },
    );
    let rows = inner.height.saturating_sub(2) as usize;
    let start = m.sel.saturating_sub(rows.saturating_sub(1));
    let lines: Vec<Line> = m
        .shown
        .iter()
        .enumerate()
        .skip(start)
        .take(rows)
        .map(|(i, it)| {
            let sel = i == m.sel;
            let room = (inner.width as usize).saturating_sub(18);
            row(
                vec![
                    mark(sel),
                    Span::styled(
                        format!("\\{:<14} ", it.key),
                        Style::default().fg(if sel { BEIGE } else { FG }),
                    ),
                    Span::styled(fit(&it.label, room), Style::default().fg(MUTED)),
                ],
                sel,
                inner.width,
            )
        })
        .collect();
    f.render_widget(
        Paragraph::new(lines).style(theme::base()),
        Rect {
            y: inner.y + 2,
            height: inner.height.saturating_sub(2),
            ..inner
        },
    );
    f.set_cursor_position(Position::new(
        inner.x + 2 + query.chars().count() as u16,
        inner.y,
    ));
}

pub fn draw_form(f: &mut Frame, area: Rect, form: &Form) {
    let label_w = form
        .fields
        .iter()
        .map(|x| label_of(&x.label).chars().count())
        .max()
        .unwrap_or(4) as u16
        + 2;
    let w = area.width.saturating_sub(8).clamp(36, 70);
    let h = form.fields.len() as u16 + 4 + u16::from(form.error.is_some());
    let r = Rect {
        x: area.x + (area.width - w) / 2,
        y: area.y + area.height.saturating_sub(h) / 3,
        width: w,
        height: h,
    };
    float(f, r);
    f.render_widget(panel(&form.title), r);
    let inner = inner(r);
    let mut cursor = None;
    for (i, fl) in form.fields.iter().enumerate() {
        let y = inner.y + i as u16;
        let focused = i == form.focus;
        let label = Span::styled(
            format!("{:<w$}", label_of(&fl.label), w = label_w as usize),
            Style::default().fg(if focused { BEIGE } else { MUTED }),
        );
        let value = if fl.toggle {
            Span::styled(
                if fl.on() {
                    lang::l("[x] evet", "[x] yes")
                } else {
                    lang::l("[ ] hayır", "[ ] no")
                }
                .to_string(),
                Style::default().fg(FG),
            )
        } else {
            // A typing field: a band to type on, as wide as the window.
            let room = (inner.width.saturating_sub(label_w + 1)) as usize;
            Span::styled(
                format!("{:<room$}", fl.value),
                Style::default().fg(FG).bg(FIELD),
            )
        };
        let mut spans = vec![label, value];
        if focused && !fl.toggle && !fl.options.is_empty() && fl.value.is_empty() {
            // The hint sits on the empty band.
            spans.pop();
            let hint = format!("Tab: {}", fl.options[0]);
            let room = (inner.width.saturating_sub(label_w + 1)) as usize;
            spans.push(Span::styled(
                format!("{:<room$}", fit(&hint, room)),
                Style::default().fg(FAINT).bg(FIELD),
            ));
        }
        f.render_widget(
            Paragraph::new(Line::from(spans)).style(theme::base()),
            Rect {
                y,
                height: 1,
                ..inner
            },
        );
        if focused && !fl.toggle {
            cursor = Some(Position::new(
                inner.x + label_w + fl.value.chars().count() as u16,
                y,
            ));
        }
    }
    let mut y = inner.y + form.fields.len() as u16 + 1;
    if let Some(e) = &form.error {
        f.render_widget(
            Paragraph::new(e.as_str()).style(theme::base().fg(theme::WARN)),
            Rect {
                y,
                height: 1,
                ..inner
            },
        );
        y += 1;
    }
    let hint = if form.kind == FormKind::Find {
        lang::l(
            "⏎ sonraki   ^R değiştir   Alt+R hepsini   ↑↓ alan   Esc kapat",
            "⏎ next   ^R replace   Alt+R all   ↑↓ field   Esc close",
        )
    } else {
        lang::l(
            "⏎ tamam   Tab tamamla   ↑↓ alan   Esc vazgeç",
            "⏎ done   Tab complete   ↑↓ field   Esc cancel",
        )
    };
    f.render_widget(
        Paragraph::new(hint).style(theme::base().fg(FAINT)),
        Rect {
            y,
            height: 1,
            ..inner
        },
    );
    if let Some(c) = cursor {
        f.set_cursor_position(c);
    }
}

/// A field's label on screen: the Turkish label is also its key in code.
pub fn label_of(key: &str) -> &str {
    if lang::is_tr() {
        return key;
    }
    match key {
        "Yer" => "Place",
        "Zaman" => "Time",
        "Başlık" => "Title",
        "Ad" => "Name",
        "Kısaltma" => "Alias",
        "Tam ad" => "Full name",
        "Açıklama" => "Description",
        "Kişiler tablosunda" => "In the cast table",
        "Yıldızlı" => "Starred",
        "Gizli (HERKES gibi)" => "Hidden (like ALL)",
        "Yaş" => "Age",
        "İlişkiler" => "Relations",
        "Notlar" => "Notes",
        "Ara" => "Find",
        "Değiştir" => "Replace",
        "Yazar" => "Author",
        "Taslak no" => "Draft no",
        "Telif" => "Copyright",
        "Adres" => "Address",
        "E-posta" => "E-mail",
        "Telefon" => "Phone",
        "Final (baskı)" => "Final (print)",
        "Dil" => "Language",
        "Oyunlar klasörü" => "Plays folder",
        "Adın" => "Your name",
        "Öneri menüsü" => "Suggestion menu",
        "Dakikada kelime" => "Words a minute",
        "Klavye" => "Keyboard",
        "Dosya" => "File",
        other => other,
    }
}

/// What a key did to a form.
pub enum Outcome {
    Stay,
    Submit,
    Cancel,
}

/// Field editing shared by every form: typing, Backspace, ↑↓, Tab to
/// complete or move on, Space on a yes/no field.
pub fn edit(form: &mut Form, k: ratatui::crossterm::event::KeyEvent) -> Outcome {
    use ratatui::crossterm::event::KeyCode;
    form.error = None;
    let n = form.fields.len();
    match k.code {
        KeyCode::Esc => return Outcome::Cancel,
        KeyCode::Enter => return Outcome::Submit,
        KeyCode::Up | KeyCode::BackTab => form.focus = (form.focus + n - 1) % n,
        KeyCode::Down => form.focus = (form.focus + 1) % n,
        KeyCode::Tab => {
            if !form.complete() {
                form.focus = (form.focus + 1) % n;
            }
        }
        KeyCode::Backspace => {
            let f = &mut form.fields[form.focus];
            if !f.toggle {
                f.value.pop();
                f.touched = true;
            }
        }
        KeyCode::Char(' ') if form.fields[form.focus].toggle => {
            let f = &mut form.fields[form.focus];
            f.value = if f.on() { "hayır" } else { "evet" }.into();
        }
        KeyCode::Char(c) => {
            let f = &mut form.fields[form.focus];
            if !f.toggle {
                f.value.push(c);
                f.touched = true;
            }
        }
        _ => {}
    }
    Outcome::Stay
}

/// Inside a window's border, with a column of air on each side.
pub fn inner(r: Rect) -> Rect {
    Rect {
        x: r.x + 2,
        y: r.y + 1,
        width: r.width.saturating_sub(4),
        height: r.height.saturating_sub(2),
    }
}
