//! Small windows over the text: the suggestion menu, the verb list, the
//! search window and the forms.

use super::theme::{self, BEIGE, CLARET, FAINT, FG, MUTED};
use dactile::complete::{self, Item};
use ratatui::Frame;
use ratatui::layout::{Position, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

/// A list to choose from, filtered by what is typed.
pub struct Menu {
    pub all: Vec<Item>,
    pub shown: Vec<Item>,
    pub sel: usize,
    /// Where the typed query starts in the text (after the backslash).
    pub from: usize,
}

impl Menu {
    pub fn new(all: Vec<Item>, from: usize) -> Menu {
        let shown = all.clone();
        Menu {
            all,
            shown,
            sel: 0,
            from,
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormKind {
    Scene,
    Act,
    Character,
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

/// Where a popup of `w`×`h` goes near the cursor, inside `area`.
fn near(area: Rect, at: Position, w: u16, h: u16) -> Rect {
    let w = w.min(area.width);
    let h = h.min(area.height);
    let x = at.x.min(area.x + area.width - w);
    let below = at.y + 1;
    let y = if below + h <= area.y + area.height {
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
    let w = m
        .shown
        .iter()
        .take(8)
        .map(|i| i.key.chars().count() + i.label.chars().count() + 5)
        .max()
        .unwrap_or(20) as u16;
    let r = near(area, at, w.clamp(24, 64), rows + 2);
    f.render_widget(Clear, r);
    let title = if query.is_empty() {
        "\\".to_string()
    } else {
        format!("\\{query}")
    };
    f.render_widget(panel(&title), r);
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
            Line::from(vec![
                Span::styled(if sel { "▸ " } else { "  " }, Style::default().fg(CLARET)),
                Span::styled(format!("{:<9}", it.key), key),
                Span::styled(it.label.clone(), Style::default().fg(MUTED)),
            ])
        })
        .collect();
    f.render_widget(Paragraph::new(lines).style(theme::base()), inner(r));
}

pub fn draw_verbs(f: &mut Frame, area: Rect, at: Position, v: &Verbs) {
    let rows = v.shown.len().min(6) as u16;
    if rows == 0 {
        return;
    }
    let w = v
        .shown
        .iter()
        .take(6)
        .map(|s| s.chars().count() + 4)
        .max()
        .unwrap_or(10) as u16;
    let r = near(area, at, w.clamp(16, 40), rows + 2);
    f.render_widget(Clear, r);
    f.render_widget(panel("fiil"), r);
    let start = v.sel.saturating_sub(rows as usize - 1);
    let lines: Vec<Line> = v
        .shown
        .iter()
        .enumerate()
        .skip(start)
        .take(rows as usize)
        .map(|(i, s)| {
            let sel = i == v.sel;
            Line::from(vec![
                Span::styled(if sel { "▸ " } else { "  " }, Style::default().fg(CLARET)),
                Span::styled(
                    s.clone(),
                    Style::default().fg(if sel { BEIGE } else { MUTED }),
                ),
            ])
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
    f.render_widget(Clear, r);
    f.render_widget(panel("ara — karakter, komut"), r);
    let inner = inner(r);
    let q = Line::from(vec![
        Span::styled("› ", Style::default().fg(CLARET)),
        Span::styled(query.to_string(), Style::default().fg(FG)),
    ]);
    f.render_widget(
        Paragraph::new(q).style(theme::base()),
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
            Line::from(vec![
                Span::styled(if sel { "▸ " } else { "  " }, Style::default().fg(CLARET)),
                Span::styled(
                    format!("\\{:<14} ", it.key),
                    Style::default().fg(if sel { BEIGE } else { FG }),
                ),
                Span::styled(it.label.clone(), Style::default().fg(MUTED)),
            ])
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
        .map(|x| x.label.chars().count())
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
    f.render_widget(Clear, r);
    f.render_widget(panel(&form.title), r);
    let inner = inner(r);
    let mut cursor = None;
    for (i, fl) in form.fields.iter().enumerate() {
        let y = inner.y + i as u16;
        let focused = i == form.focus;
        let label = Span::styled(
            format!("{:<w$}", fl.label, w = label_w as usize),
            Style::default().fg(if focused { BEIGE } else { MUTED }),
        );
        let value = if fl.toggle {
            Span::styled(
                if fl.on() { "[x] evet" } else { "[ ] hayır" }.to_string(),
                Style::default().fg(FG),
            )
        } else {
            Span::styled(fl.value.clone(), Style::default().fg(FG))
        };
        let mut spans = vec![label, value];
        if focused && !fl.toggle && !fl.options.is_empty() && fl.value.is_empty() {
            spans.push(Span::styled(
                format!("  Tab: {}", fl.options[0]),
                Style::default().fg(FAINT),
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
    let hint = "⏎ tamam   Tab tamamla   ↑↓ alan   Esc vazgeç";
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

fn inner(r: Rect) -> Rect {
    Rect {
        x: r.x + 1,
        y: r.y + 1,
        width: r.width.saturating_sub(2),
        height: r.height.saturating_sub(2),
    }
}
