//! The document in the import preview: its paragraphs and what each is
//! taken for, its parts to mark (a book's cover, its contents, where the
//! play starts), and the play as it was read. What is drawn is kept ready:
//! a book has thousands of paragraphs and a frame shows a few.

use crate::tui::popup;
use crate::tui::theme::{BEIGE, CLARET, FAINT, FG, MUTED};
use dactile::import::Run;
use dactile::import::pages::Page;
use dactile::import::shape::{Draft, Item};
use dactile::lang;
use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};
use std::cell::Cell;

/// As many of the play's lines as the preview shows.
const SHOWN: usize = 400;
/// From this width the parts have a column of their own.
const TWO_PANES: u16 = 72;

fn page_name(p: Page) -> &'static str {
    match p {
        Page::Play => lang::l("Oyun", "Play"),
        Page::Cover => lang::l("Kapak", "Cover"),
        Page::Imprint => lang::l("Künye", "Imprint"),
        Page::Contents => lang::l("İçindekiler", "Contents"),
        Page::Preface => lang::l("Önsöz", "Preface"),
        Page::Author => lang::l("Yazar", "Author"),
        Page::Cast => lang::l("Kişiler", "Cast"),
        Page::Others => lang::l("Diğer kişiler", "Others in the cast"),
        Page::Notes => lang::l("Notlar", "Notes"),
        Page::Back => lang::l("Kitabın sonu", "End of book"),
        Page::Footnotes => lang::l("Dipnotlar", "Footnotes"),
        Page::Other => lang::l("Diğer", "Other"),
    }
}

/// A run of paragraphs of one label.
struct Part {
    from: usize,
    to: usize,
    page: Page,
    /// The pages printed on it: "s. 3–8".
    pages: String,
}

enum Row {
    Head(usize),
    Para(usize),
}

#[derive(Clone, Copy, PartialEq)]
enum Said {
    Heading,
    Speech,
    Direction,
    Plain,
}

/// A line of the play as it was read.
struct Read {
    kind: Said,
    who: Option<String>,
    text: String,
}

pub(super) struct Marking {
    /// Each paragraph on one line, and the page printed where it is.
    texts: Vec<String>,
    pages: Vec<Option<String>>,
    parts: Vec<Part>,
    rows: Vec<Row>,
    /// The row of each paragraph.
    row_of: Vec<usize>,
    /// The paragraphs chosen, from where it started to where it reached.
    a: usize,
    b: usize,
    /// `Space` pressed: moving widens the choice.
    ranging: bool,
    /// The paragraph kept in sight.
    at: usize,
    /// Marking a part goes on to the next, as replacing does in a search.
    onward: bool,
    /// The picker of labels, open on this entry of `Page::ALL`.
    picker: Option<usize>,
    /// The first row in sight, as the last frame had it.
    top: Cell<usize>,
    play: Vec<Read>,
    /// How many more lines the play has than `play` holds.
    more: usize,
    play_top: usize,
}

fn joined(runs: &[Run]) -> String {
    runs.iter()
        .map(|r| {
            if r.note {
                format!(" [*{}]", r.text)
            } else {
                r.text.clone()
            }
        })
        .collect()
}

fn read_item(d: &Draft, it: &Item) -> Read {
    let line = |kind, who, text| Read { kind, who, text };
    match it {
        Item::Act(t) | Item::Scene(t) => line(Said::Heading, None, t.clone()),
        Item::Preface => line(Said::Heading, None, lang::l("Önsöz", "Preface").into()),
        Item::Notes => line(Said::Heading, None, lang::l("Notlar", "Notes").into()),
        Item::Speech { who, said, raw } => {
            let p = &d.people[d.owner(*who)];
            if p.not_character {
                line(Said::Plain, None, joined(raw))
            } else {
                line(Said::Speech, Some(p.name.clone()), joined(said))
            }
        }
        Item::Direction(r) => line(Said::Direction, None, joined(r)),
        Item::Plain(r) => line(Said::Plain, None, joined(r)),
    }
}

impl Marking {
    pub(super) fn new(d: &Draft) -> Marking {
        let src = &d.source;
        let mut m = Marking {
            texts: src
                .paras
                .iter()
                .map(|p| p.text().replace('\n', " "))
                .collect(),
            pages: src
                .paras
                .iter()
                .map(|p| p.page.and_then(|i| src.pages.get(i).cloned()))
                .collect(),
            parts: Vec::new(),
            rows: Vec::new(),
            row_of: Vec::new(),
            a: 0,
            b: 0,
            ranging: false,
            at: 0,
            onward: true,
            picker: None,
            top: Cell::new(0),
            play: Vec::new(),
            more: 0,
            play_top: 0,
        };
        m.refresh(d);
        m.go_to_play(d);
        m
    }

    /// The first paragraph of the play, chosen.
    pub(super) fn go_to_play(&mut self, d: &Draft) {
        let at = d.labels.iter().position(|&p| p == Page::Play);
        self.choose(at.unwrap_or(0));
    }

    /// What the labels and the people came to.
    pub(super) fn refresh(&mut self, d: &Draft) {
        self.parts.clear();
        for (i, &page) in d.labels.iter().enumerate() {
            match self.parts.last_mut() {
                Some(last) if last.page == page => last.to = i,
                _ => self.parts.push(Part {
                    from: i,
                    to: i,
                    page,
                    pages: String::new(),
                }),
            }
        }
        self.rows.clear();
        self.row_of = vec![0; d.labels.len()];
        for n in 0..self.parts.len() {
            let (from, to) = (self.parts[n].from, self.parts[n].to);
            self.parts[n].pages = self.pages_of(from, to);
            self.rows.push(Row::Head(n));
            for i in from..=to {
                self.row_of[i] = self.rows.len();
                self.rows.push(Row::Para(i));
            }
        }
        let items = d.all_items();
        self.more = items.len().saturating_sub(SHOWN);
        self.play = items
            .iter()
            .take(SHOWN)
            .map(|it| read_item(d, it))
            .collect();
        self.play_top = self.play_top.min(self.play.len().saturating_sub(1));
    }

    fn pages_of(&self, from: usize, to: usize) -> String {
        let part = &self.pages[from..=to];
        let first = part.iter().find_map(|p| p.as_deref());
        let last = part.iter().rev().find_map(|p| p.as_deref());
        match (first, last) {
            (Some(a), Some(b)) if a != b => lang::f("s. {}–{}", "pp. {}–{}", &[&a, &b]),
            (Some(a), _) => lang::f("s. {}", "p. {}", &[&a]),
            _ => String::new(),
        }
    }

    fn lo(&self) -> usize {
        self.a.min(self.b)
    }

    fn hi(&self) -> usize {
        self.a.max(self.b)
    }

    fn last(&self) -> usize {
        self.row_of.len().saturating_sub(1)
    }

    fn choose(&mut self, i: usize) {
        self.a = i;
        self.b = i;
        self.at = i;
        self.ranging = false;
    }

    fn move_to(&mut self, i: usize) {
        let i = i.min(self.last());
        if self.ranging {
            self.b = i;
            self.at = i;
        } else {
            self.choose(i);
        }
    }

    fn choose_part(&mut self, from: usize, to: usize) {
        self.a = from;
        self.b = to;
        self.at = from;
        self.ranging = false;
    }

    /// The part after (or before) the paragraphs chosen.
    pub(super) fn step(&mut self, on: bool) {
        let (lo, hi) = (self.lo(), self.hi());
        let at = if on {
            self.parts.iter().find(|p| p.from > hi)
        } else {
            self.parts.iter().rev().find(|p| p.to < lo)
        };
        if let Some((from, to)) = at.map(|p| (p.from, p.to)) {
            self.choose_part(from, to);
        }
    }

    /// A draft read again comes back with a book's title in a volume: this
    /// play's stays.
    fn keeping(d: &mut Draft, edit: impl FnOnce(&mut Draft)) {
        let (play, title, author) = (d.play, d.title.clone(), d.author.clone());
        edit(d);
        if play.is_some() {
            d.play = play;
            d.title = title;
            d.author = author;
        }
    }

    fn mark(&mut self, d: &mut Draft, page: Page) {
        let (lo, hi) = (self.lo(), self.hi());
        Self::keeping(d, |d| d.mark(lo, hi, page));
        self.refresh(d);
        let next = self.parts.iter().find(|p| p.from > hi);
        match next.map(|p| (p.from, p.to)) {
            Some((from, to)) if self.onward => self.choose_part(from, to),
            _ => self.ranging = false,
        }
    }

    /// A key in the document: whether the people may have changed.
    pub(super) fn doc_key(&mut self, k: KeyEvent, d: &mut Draft) -> bool {
        if let Some(i) = self.picker {
            match k.code {
                KeyCode::Up => self.picker = Some(i.saturating_sub(1)),
                KeyCode::Down => self.picker = Some((i + 1).min(Page::ALL.len() - 1)),
                KeyCode::Esc => self.picker = None,
                KeyCode::Enter => {
                    self.picker = None;
                    self.mark(d, Page::ALL[i]);
                    return true;
                }
                _ => {}
            }
            return false;
        }
        if self.row_of.is_empty() {
            return false;
        }
        match k.code {
            KeyCode::Up => self.move_to(self.at.saturating_sub(1)),
            KeyCode::Down => self.move_to(self.at + 1),
            KeyCode::PageUp => self.move_to(self.at.saturating_sub(10)),
            KeyCode::PageDown => self.move_to(self.at + 10),
            KeyCode::Home => self.move_to(0),
            KeyCode::End => self.move_to(self.last()),
            KeyCode::Char(' ') => {
                let on = !self.ranging;
                self.choose(self.at);
                self.ranging = on;
            }
            KeyCode::Char('n') | KeyCode::F(3)
                if !k
                    .modifiers
                    .contains(ratatui::crossterm::event::KeyModifiers::SHIFT) =>
            {
                self.step(true)
            }
            KeyCode::Char('p') | KeyCode::F(3) => self.step(false),
            KeyCode::Char('g') => self.onward = !self.onward,
            KeyCode::Char('m') => {
                let now = d.labels.get(self.lo()).copied().unwrap_or(Page::Play);
                self.picker = Page::ALL.iter().position(|&p| p == now);
            }
            KeyCode::Char('s') => {
                let lo = self.lo();
                Self::keeping(d, |d| d.play_from(lo));
                self.refresh(d);
                self.ranging = false;
                return true;
            }
            KeyCode::Char('e') => {
                let hi = self.hi();
                Self::keeping(d, |d| d.play_to(hi));
                self.refresh(d);
                self.ranging = false;
                return true;
            }
            _ => {}
        }
        false
    }

    /// The first paragraph chosen.
    pub(super) fn at(&self) -> usize {
        self.lo()
    }

    /// The last paragraph chosen.
    pub(super) fn end(&self) -> usize {
        self.hi()
    }

    pub(super) fn ranging(&self) -> bool {
        self.ranging
    }

    /// Esc: the choice goes back to one paragraph.
    pub(super) fn stop_ranging(&mut self) {
        self.choose(self.at);
    }

    pub(super) fn picking(&self) -> bool {
        self.picker.is_some()
    }

    pub(super) fn play_key(&mut self, k: KeyEvent) {
        let last = self.play.len().saturating_sub(1);
        self.play_top = match k.code {
            KeyCode::Up => self.play_top.saturating_sub(1),
            KeyCode::Down => (self.play_top + 1).min(last),
            KeyCode::PageUp => self.play_top.saturating_sub(10),
            KeyCode::PageDown => (self.play_top + 10).min(last),
            KeyCode::Home => 0,
            KeyCode::End => last,
            _ => self.play_top,
        };
    }

    pub(super) fn help(&self) -> String {
        let chosen = self.hi() - self.lo() + 1;
        let on = if self.onward {
            lang::l("açık", "on")
        } else {
            lang::l("kapalı", "off")
        };
        lang::f(
            "{} paragraf seçili · ↑↓ · Space aralık · n/p bölüm · m işaretle · s oyun burada başlar · e burada biter · g sonrakine geç: {}",
            "{} paragraphs chosen · ↑↓ · Space range · n/p part · m mark · s the play starts here · e ends here · g go on to the next: {}",
            &[&chosen, &on],
        )
    }

    pub(super) fn draw_doc(&self, f: &mut Frame, area: Rect) {
        let mut doc = area;
        if area.width >= TWO_PANES {
            let w = (area.width / 3).clamp(24, 38);
            self.draw_parts(f, Rect { width: w, ..area });
            doc = Rect {
                x: area.x + w + 1,
                width: area.width - w - 1,
                ..area
            };
        }
        self.draw_rows(f, doc);
    }

    fn draw_parts(&self, f: &mut Frame, area: Rect) {
        let room = (area.height as usize).saturating_sub(1) / 2;
        let on = self.parts.iter().position(|p| p.to >= self.at).unwrap_or(0);
        let first = (on + 1).saturating_sub(room.max(1));
        let w = area.width as usize;
        let mut lines = vec![Line::from(Span::styled(
            lang::l("BÖLÜMLER", "PARTS"),
            Style::default().fg(FAINT),
        ))];
        for (n, p) in self.parts.iter().enumerate().skip(first).take(room) {
            let sel = p.from <= self.hi() && p.to >= self.lo();
            let head = format!("{} {} ({})", page_name(p.page), p.pages, p.to - p.from + 1);
            let name = Style::default().fg(BEIGE).add_modifier(Modifier::BOLD);
            lines.push(popup::row(
                vec![
                    popup::mark(n == on),
                    Span::styled(popup::fit(&head, w.saturating_sub(2)), name),
                ],
                sel,
                area.width,
            ));
            let text = self.texts.get(p.from).map_or("", String::as_str);
            lines.push(popup::row(
                vec![Span::styled(
                    format!("  {}", popup::fit(text, w.saturating_sub(2))),
                    Style::default().fg(MUTED),
                )],
                sel,
                area.width,
            ));
        }
        f.render_widget(Paragraph::new(lines), area);
    }

    fn draw_rows(&self, f: &mut Frame, area: Rect) {
        let room = (area.height as usize).max(1);
        let focus = self.row_of.get(self.at).copied().unwrap_or(0);
        let mut top = self.top.get().min(self.rows.len().saturating_sub(1));
        if focus < top + 1 {
            // The part's heading comes with its first paragraph.
            top = focus.saturating_sub(1);
        } else if focus >= top + room {
            top = focus + 1 - room;
        }
        self.top.set(top);
        let label_w = if area.width < 60 { 7 } else { 13 };
        let w = area.width as usize;
        let fixed = 2 + label_w + 1 + 5 + 1;
        let lines: Vec<Line> = self
            .rows
            .iter()
            .skip(top)
            .take(room)
            .map(|r| match r {
                Row::Head(n) => self.head_row(*n, w),
                Row::Para(i) => self.para_row(*i, label_w, w.saturating_sub(fixed), area.width),
            })
            .collect();
        f.render_widget(Paragraph::new(lines), area);
        if let Some(i) = self.picker {
            self.draw_picker(f, area, i);
        }
    }

    fn head_row(&self, n: usize, w: usize) -> Line<'static> {
        let p = &self.parts[n];
        let s = lang::f(
            "── {} {} · {} paragraf",
            "── {} {} · {} paragraphs",
            &[&page_name(p.page), &p.pages, &(p.to - p.from + 1)],
        );
        Line::from(Span::styled(
            popup::fit(&s, w),
            Style::default().fg(CLARET).add_modifier(Modifier::BOLD),
        ))
    }

    fn para_row(&self, i: usize, label_w: usize, text_w: usize, width: u16) -> Line<'static> {
        let sel = (self.lo()..=self.hi()).contains(&i);
        let part = self.parts.iter().find(|p| p.to >= i);
        let page = part.map_or(Page::Other, |p| p.page);
        let printed = self.pages[i]
            .as_deref()
            .filter(|_| i == 0 || self.pages[i] != self.pages[i - 1])
            .unwrap_or("");
        let text = if page == Page::Play { FG } else { MUTED };
        popup::row(
            vec![
                popup::mark(i == self.at),
                Span::styled(
                    format!("{:<label_w$} ", popup::fit(page_name(page), label_w),),
                    Style::default().fg(FAINT),
                ),
                Span::styled(format!("{printed:>5} "), Style::default().fg(FAINT)),
                Span::styled(
                    popup::fit(&self.texts[i], text_w),
                    Style::default().fg(text),
                ),
            ],
            sel,
            width,
        )
    }

    fn draw_picker(&self, f: &mut Frame, area: Rect, on: usize) {
        let w = 26.min(area.width);
        let h = (Page::ALL.len() as u16 + 2).min(area.height);
        let r = Rect {
            x: area.x + (area.width - w) / 2,
            y: area.y + (area.height - h) / 2,
            width: w,
            height: h,
        };
        popup::float(f, r);
        f.render_widget(popup::panel(lang::l("bu bölüm", "this part")), r);
        let inner = popup::inner(r);
        let room = (inner.height as usize).max(1);
        let first = (on + 1).saturating_sub(room);
        let lines: Vec<Line> = Page::ALL
            .iter()
            .enumerate()
            .skip(first)
            .take(room)
            .map(|(i, &p)| {
                popup::row(
                    vec![
                        popup::mark(i == on),
                        Span::styled(page_name(p), Style::default().fg(FG)),
                    ],
                    i == on,
                    inner.width,
                )
            })
            .collect();
        f.render_widget(Paragraph::new(lines), inner);
    }

    pub(super) fn draw_play(&self, f: &mut Frame, area: Rect) {
        let mut lines: Vec<Line> = self
            .play
            .iter()
            .skip(self.play_top)
            .take(area.height as usize)
            .map(|l| {
                let text = l.text.clone();
                match l.kind {
                    Said::Heading => Line::from(Span::styled(
                        text,
                        Style::default().fg(BEIGE).add_modifier(Modifier::BOLD),
                    )),
                    Said::Direction => Line::from(Span::styled(
                        text,
                        Style::default().fg(MUTED).add_modifier(Modifier::ITALIC),
                    )),
                    Said::Speech | Said::Plain => {
                        let who = l.who.as_ref().map(|w| {
                            Span::styled(
                                format!("{w}  "),
                                Style::default().fg(BEIGE).add_modifier(Modifier::BOLD),
                            )
                        });
                        let said = Span::styled(text, Style::default().fg(FG));
                        Line::from(who.into_iter().chain([said]).collect::<Vec<_>>())
                    }
                }
            })
            .collect();
        if self.more > 0 && lines.len() < area.height as usize {
            lines.push(Line::from(Span::styled(
                lang::f("… ve {} satır daha", "… and {} more", &[&self.more]),
                Style::default().fg(MUTED),
            )));
        }
        f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
    }
}
