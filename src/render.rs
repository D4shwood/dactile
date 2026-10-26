//! The live view: a play's text laid out the way the PDF reads, with every
//! place the cursor can stand mapped back to a byte of the source.
//!
//! A macro shows as what it means: `\se Amca` is "SELİN  Amca", `\ic{güler}`
//! is "(güler)". Such a piece is a chip: the cursor steps over it as one unit,
//! and editing it opens the block as raw source (`Options::raw_block`).
//! Blocks the app doesn't understand are always shown raw.

use crate::cast::{self, Cast};
use crate::info;
use crate::lang;
use crate::lex::{self, Block, Inline, Kind};
use crate::outline::{self, Outline};
use crate::source::{self, Parts};
use crate::tex;
use std::ops::Range;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// The play model of one version of the text.
pub struct Doc {
    pub parts: Parts,
    pub cast: Cast,
    /// Preamble, body and tail blocks, in order.
    pub blocks: Vec<Block>,
    /// Which of `blocks` are the body.
    pub body: Range<usize>,
    /// Built on the body blocks: its block indexes are relative to `body.start`.
    pub outline: Outline,
}

impl Doc {
    pub fn new(text: &str) -> Doc {
        let parts = source::split(text);
        let cast = Cast::parse(text, parts.preamble.clone());
        let none = Cast::default();
        let mut blocks = if parts.preamble.is_empty() {
            Vec::new()
        } else {
            lex::blocks(text, parts.preamble.clone(), &none)
        };
        let body_start = blocks.len();
        let body = lex::blocks(text, parts.body.clone(), &cast);
        let outline = Outline::of(text, &body, &cast);
        blocks.extend(body);
        let body_end = blocks.len();
        if !parts.tail.is_empty() {
            blocks.extend(lex::blocks(text, parts.tail.clone(), &none));
        }
        Doc {
            parts,
            cast,
            blocks,
            body: body_start..body_end,
            outline,
        }
    }

    /// The block holding byte `at`.
    pub fn block_at(&self, at: usize) -> Option<usize> {
        self.blocks.iter().rposition(|b| b.span.start <= at)
    }

    /// The scene a block is in, as an index into `outline.scenes`.
    pub fn scene_of(&self, block: usize) -> Option<usize> {
        self.body_index(block)
            .and_then(|b| self.outline.scene_at(b))
    }

    pub fn act_of(&self, block: usize) -> Option<usize> {
        self.body_index(block).and_then(|b| self.outline.act_at(b))
    }

    fn body_index(&self, block: usize) -> Option<usize> {
        self.body.contains(&block).then(|| block - self.body.start)
    }

    /// Whether the block is a line of dialogue or a `\xdo` direction, where
    /// `_word_` underlines.
    pub fn is_dialogue(&self, block: usize) -> bool {
        matches!(
            self.blocks.get(block).map(|b| &b.kind),
            Some(Kind::Line { .. } | Kind::Do { .. })
        )
    }

    pub fn in_preamble(&self, block: usize) -> bool {
        block < self.body.start
    }

    /// The play's title from `\title{…}`, its first line.
    pub fn title(&self, text: &str) -> Option<String> {
        let c = tex::commands(text, self.parts.preamble.clone())
            .into_iter()
            .find(|c| c.name == "title")?;
        let g = tex::group(text, c.span.end)?;
        let first = g.inner.split("\\\\").next().unwrap_or("").trim();
        Some(first.to_string()).filter(|t| !t.is_empty())
    }

    /// Who speaks in a block.
    pub fn speaker(&self, block: usize) -> Option<&str> {
        match &self.blocks.get(block)?.kind {
            Kind::Line { alias, .. } => Some(alias),
            Kind::Do { aliases, .. } => aliases.first().map(String::as_str),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Style {
    Text,
    Name,
    NameStar,
    NameMuted,
    Direction,
    Ref,
    Under,
    Heading,
    /// The play's title on the cover preview.
    Title,
    SceneCast,
    Faint,
    Raw,
    RawCmd,
    Warn,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Cell {
    pub col: u16,
    pub ch: char,
    pub style: Style,
    /// The source byte of this character when it is a character of the
    /// source: what a selection covering it covers.
    pub byte: Option<usize>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Row {
    /// The block the row shows; None for the folded preamble line.
    pub block: Option<usize>,
    pub cells: Vec<Cell>,
    /// The draft line number shown in the margin, on a block's first row.
    pub number: Option<usize>,
    /// Between two pages: drawn as the desk, not as paper.
    pub gap: bool,
}

/// A place the cursor can stand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stop {
    pub byte: usize,
    pub row: usize,
    pub col: u16,
}

#[derive(Debug, Clone, Default)]
pub struct Layout {
    pub rows: Vec<Row>,
    /// Sorted by byte, one per byte.
    pub stops: Vec<Stop>,
    /// The rows of each block (empty for blocks folded away).
    pub block_rows: Vec<Range<usize>>,
    /// Width of the name column, the gap included.
    pub name_width: u16,
}

impl Layout {
    /// The stop for a byte, or the nearest one before it.
    pub fn stop_at(&self, byte: usize) -> Option<Stop> {
        let i = self.stops.partition_point(|s| s.byte <= byte);
        self.stops
            .get(i.checked_sub(1)?)
            .copied()
            .or_else(|| self.stops.first().copied())
    }

    pub fn is_stop(&self, byte: usize) -> bool {
        self.stops.binary_search_by_key(&byte, |s| s.byte).is_ok()
    }

    pub fn prev_stop(&self, byte: usize) -> Option<Stop> {
        let i = self.stops.partition_point(|s| s.byte < byte);
        i.checked_sub(1).map(|i| self.stops[i])
    }

    pub fn next_stop(&self, byte: usize) -> Option<Stop> {
        let i = self.stops.partition_point(|s| s.byte <= byte);
        self.stops.get(i).copied()
    }

    /// The stop on `row` nearest to `col`.
    pub fn stop_near(&self, row: usize, col: u16) -> Option<Stop> {
        self.stops
            .iter()
            .filter(|s| s.row == row)
            .min_by_key(|s| (s.col as i32 - col as i32).unsigned_abs())
            .copied()
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Options {
    pub width: u16,
    /// The block shown as raw source because it is being edited as such.
    pub raw_block: Option<usize>,
    /// Everything as raw source.
    pub raw_all: bool,
    /// Show the preamble's source instead of one summary line.
    pub preamble_open: bool,
    /// Show only this scene (an index into `outline.scenes`); the others
    /// fold to one line each.
    pub focus_scene: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Align {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone)]
struct Glyph {
    ch: char,
    style: Style,
    /// The cursor stop just before this character.
    stop: Option<usize>,
    /// For a character of the source: its byte.
    byte: Option<usize>,
}

impl Glyph {
    fn plain(ch: char, style: Style, at: usize) -> Glyph {
        Glyph {
            ch,
            style,
            stop: Some(at),
            byte: Some(at),
        }
    }
    fn deco(ch: char, style: Style, stop: Option<usize>) -> Glyph {
        Glyph {
            ch,
            style,
            stop,
            byte: None,
        }
    }
}

/// `s` as one atomic chip: a stop before its first character only.
fn chip(s: &str, style: Style, at: usize) -> Vec<Glyph> {
    s.chars()
        .enumerate()
        .map(|(i, c)| Glyph::deco(c, style, (i == 0).then_some(at)))
        .collect()
}

fn cw(c: char) -> u16 {
    c.width().unwrap_or(0).max(1) as u16
}

/// Lays the whole play out for a screen `opts.width` columns wide.
pub fn layout(text: &str, doc: &Doc, opts: Options) -> Layout {
    let mut b = Builder {
        text,
        doc,
        out: Layout::default(),
        width: opts.width.max(20),
    };
    b.out.name_width = name_column(doc, b.width);
    // Where the PDF starts a new page; never in the all-source view.
    let pages = !opts.raw_all;
    let mut break_next = false;
    let mut seen_act = false;
    for i in 0..doc.blocks.len() {
        if pages && doc.body.contains(&i) {
            let kind = &doc.blocks[i].kind;
            let cmd = text[doc.blocks[i].span.clone()].trim();
            if matches!(kind, Kind::Act { .. }) {
                // The curtain falls on the act before.
                if seen_act {
                    b.curtain(false);
                }
                seen_act = true;
            }
            let before =
                matches!(kind, Kind::Act { .. } | Kind::Section { .. }) || cmd == "\\kisiler";
            if (before || break_next) && !matches!(kind, Kind::Blank) {
                b.gap();
                break_next = false;
            }
            if cmd == "\\maketitle" || is_page_break(cmd) {
                break_next = true;
            }
        }
        let start = b.out.rows.len();
        let raw = opts.raw_all || opts.raw_block == Some(i);
        if doc.in_preamble(i) && !opts.preamble_open && !opts.raw_all {
            // One summary row for the whole preamble.
            if i == 0 {
                let n = doc.cast.characters.len();
                let s = lang::f(
                    "▸ Kişiler ({}) · oyun bilgisi",
                    "▸ Cast ({}) · cover details",
                    &[&n],
                );
                let glyphs = chip(&s, Style::Faint, doc.parts.preamble.start);
                let width = b.width;
                b.flow(0, &glyphs, 0, width, Align::Left, None, start);
                for r in &mut b.out.rows[start..] {
                    r.block = None;
                }
            }
            b.out.block_rows.push(start..b.out.rows.len());
            continue;
        }
        // Scene focus: other scenes are one summary line each.
        if let Some(f) = opts.focus_scene
            && let Some(sc) = doc.scene_of(i)
            && sc != f
        {
            let scene = &doc.outline.scenes[sc];
            if scene.block + doc.body.start == i {
                let place = if scene.place.trim().is_empty() {
                    String::new()
                } else {
                    format!(" — {}", scene.place.trim())
                };
                let s = lang::f(
                    "▸ Sahne {}{} · {} replik",
                    "▸ Scene {}{} · {} lines",
                    &[&scene.label, &place, &scene.lines],
                );
                let glyphs = chip(&s, Style::Faint, doc.blocks[i].span.start);
                let width = b.width;
                b.flow(i, &glyphs, 0, width, Align::Center, None, start);
            }
            b.out.block_rows.push(start..b.out.rows.len());
            continue;
        }
        if raw || !b.rendered(i) {
            // Plain prose reads as text; other LaTeX stays in the background.
            let prose = matches!(doc.blocks[i].kind, Kind::Other)
                && !text[doc.blocks[i].span.clone()]
                    .trim_start()
                    .starts_with('\\');
            let base = if raw || prose {
                Style::Raw
            } else {
                Style::Faint
            };
            b.raw(i, base);
        }
        if let Some(n) = i
            .checked_sub(doc.body.start)
            .and_then(|k| doc.outline.numbers.get(k).copied().flatten())
            && start < b.out.rows.len()
        {
            b.out.rows[start].number = Some(n);
        }
        b.out.block_rows.push(start..b.out.rows.len());
        if pages && i + 1 == doc.body.end && !doc.body.is_empty() {
            b.curtain(true);
        }
    }
    b.out.stops.sort_by_key(|s| s.byte);
    // One stop per byte: the first placed wins (a block's own start over the
    // end of the one before).
    b.out.stops.dedup_by_key(|s| s.byte);
    b.out
}

/// `\\clearpage` and its kin, alone on a line.
fn is_page_break(cmd: &str) -> bool {
    matches!(
        cmd,
        "\\clearpage" | "\\newpage" | "\\pagebreak" | "\\cleardoublepage"
    )
}

/// `\\section*{X}`, `\\chapter{X}`…: the heading text, when the block is only that.
fn latex_heading(text: &str, span: Range<usize>) -> Option<String> {
    let c = tex::commands(text, span.clone()).into_iter().next()?;
    if !matches!(c.name, "section" | "chapter" | "subsection")
        || !text[span.start..c.span.start].trim().is_empty()
    {
        return None;
    }
    let mut at = c.span.end;
    if let Some(st) = tex::star(text, at) {
        at = st.end;
    }
    while let Some(o) = tex::optional(text, at) {
        at = o.span.end;
    }
    let g = tex::group(text, at)?;
    text[g.span.end..span.end]
        .trim()
        .is_empty()
        .then(|| g.inner.trim().to_string())
}

/// "PERDE" → "P E R D E"; words three spaces apart.
fn spaced(s: &str) -> String {
    s.split_whitespace()
        .map(|w| w.chars().map(String::from).collect::<Vec<_>>().join(" "))
        .collect::<Vec<_>>()
        .join("   ")
}

/// The name column: as wide as the widest speaker's name and a gap, at most
/// a third of the screen.
fn name_column(doc: &Doc, width: u16) -> u16 {
    let widest = doc
        .cast
        .characters
        .iter()
        .map(|c| c.script.width())
        .max()
        .unwrap_or(6) as u16;
    (widest + 2).min(width / 3).max(6)
}

struct Builder<'a> {
    text: &'a str,
    doc: &'a Doc,
    out: Layout,
    width: u16,
}

impl Builder<'_> {
    fn ensure_row(&mut self, row: usize, block: usize) {
        while self.out.rows.len() <= row {
            self.out.rows.push(Row {
                block: Some(block),
                cells: Vec::new(),
                number: None,
                gap: false,
            });
        }
    }

    /// The end of a page and the start of the next.
    fn gap(&mut self) {
        if self.out.rows.is_empty() || self.out.rows.last().is_some_and(|r| r.gap) {
            return;
        }
        for _ in 0..2 {
            self.out.rows.push(Row {
                gap: true,
                ..Row::default()
            });
        }
        // The new page's top margin.
        self.out.rows.push(Row::default());
    }

    /// "PERDE SONU" where an act ends, and "SON" after the last one, as the
    /// class prints them.
    fn curtain(&mut self, last: bool) {
        let mut words = vec![lang::l("PERDE SONU", "CURTAIN")];
        if last {
            words.push(lang::l("SON", "THE END"));
        }
        for w in words {
            let row = self.out.rows.len();
            self.out.rows.push(Row::default());
            let glyphs: Vec<Glyph> = w
                .chars()
                .map(|c| Glyph::deco(c, Style::SceneCast, None))
                .collect();
            let width = self.width;
            self.flow(0, &glyphs, 0, width, Align::Center, None, row);
            self.out.rows[row].block = None;
        }
    }

    fn stop(&mut self, byte: usize, row: usize, col: u16) {
        self.out.stops.push(Stop { byte, row, col });
    }

    /// Lays glyphs out from `indent`, wrapping before column `right`, with a stop
    /// `end` after the last one. Returns how many rows it used.
    #[allow(clippy::too_many_arguments)]
    fn flow(
        &mut self,
        block: usize,
        glyphs: &[Glyph],
        indent: u16,
        right: u16,
        align: Align,
        end: Option<usize>,
        first_row: usize,
    ) -> usize {
        let avail = right.saturating_sub(indent).max(4);
        let mut lines: Vec<Vec<usize>> = vec![Vec::new()];
        let mut line_w = 0u16;
        let mut i = 0;
        while i < glyphs.len() {
            if glyphs[i].ch == '\n' {
                // A forced break: its stop stays at the end of the line it ends,
                // and spaces after it don't start the next line.
                lines.last_mut().unwrap().push(i);
                i += 1;
                while i < glyphs.len() && glyphs[i].ch == ' ' {
                    lines.last_mut().unwrap().push(i);
                    i += 1;
                }
                lines.push(Vec::new());
                line_w = 0;
                continue;
            }
            let mut j = i;
            while j < glyphs.len() && glyphs[j].ch != ' ' && glyphs[j].ch != '\n' {
                j += 1;
            }
            let mut k = j;
            while k < glyphs.len() && glyphs[k].ch == ' ' {
                k += 1;
            }
            let word_w: u16 = glyphs[i..j].iter().map(|g| cw(g.ch)).sum();
            if line_w > 0 && line_w + word_w > avail {
                lines.push(Vec::new());
                line_w = 0;
            }
            for (n, g) in glyphs[i..j].iter().enumerate() {
                if line_w > 0 && line_w + cw(g.ch) > avail {
                    lines.push(Vec::new());
                    line_w = 0;
                }
                lines.last_mut().unwrap().push(i + n);
                line_w += cw(g.ch);
            }
            // Spaces stay at the end of their line, even past the edge.
            for n in j..k {
                lines.last_mut().unwrap().push(n);
                line_w += 1;
            }
            i = k;
        }
        let mut last = (first_row, indent);
        for (n, line) in lines.iter().enumerate() {
            let row = first_row + n;
            self.ensure_row(row, block);
            let trailing = line
                .iter()
                .rev()
                .take_while(|&&g| glyphs[g].ch == ' ')
                .count() as u16;
            let content: u16 = line.iter().map(|&g| cw(glyphs[g].ch)).sum::<u16>() - trailing;
            let mut col = match align {
                Align::Left => indent,
                Align::Center => indent + avail.saturating_sub(content) / 2,
                Align::Right => indent + avail.saturating_sub(content),
            };
            for &g in line {
                let gl = &glyphs[g];
                if let Some(at) = gl.stop {
                    self.stop(at, row, col);
                }
                if gl.ch == '\n' {
                    continue;
                }
                self.out.rows[row].cells.push(Cell {
                    col,
                    ch: gl.ch,
                    style: gl.style,
                    byte: gl.byte,
                });
                col += cw(gl.ch);
            }
            last = (row, col);
        }
        if let Some(at) = end {
            self.stop(at, last.0, last.1);
        }
        lines.len()
    }

    /// The source of a range, commands and comments highlighted.
    fn raw_glyphs(&self, range: Range<usize>, base: Style) -> Vec<Glyph> {
        let mut out = Vec::new();
        let mut in_cmd = false;
        let mut in_comment = false;
        let mut prev = '\0';
        for (i, c) in self.text[range.clone()].char_indices() {
            let at = range.start + i;
            if c == '%' && !tex::is_escaped(self.text, at) {
                in_comment = true;
            }
            if c == '\\' && !in_comment && prev != '\\' {
                in_cmd = true;
            } else if in_cmd && prev != '\\' && !tex::is_letter(c) {
                in_cmd = false;
            }
            let style = if in_comment {
                Style::Faint
            } else if in_cmd || matches!(c, '{' | '}' | '[' | ']') {
                Style::RawCmd
            } else {
                base
            };
            out.push(Glyph::plain(c, style, at));
            prev = if prev == '\\' && c == '\\' { '\0' } else { c };
        }
        out
    }

    /// A block as its source: each source line wrapped on its own rows.
    fn raw(&mut self, b: usize, base: Style) {
        let span = self.doc.blocks[b].span.clone();
        let mut row = self.out.rows.len();
        let lines = tex::lines(&self.text[span.clone()]);
        if lines.is_empty() {
            self.ensure_row(row, b);
            self.stop(span.start, row, 0);
            return;
        }
        for line in lines {
            let line = line.start + span.start..line.end + span.start;
            let glyphs = self.raw_glyphs(line.clone(), base);
            row += self.flow(b, &glyphs, 0, self.width, Align::Left, Some(line.end), row);
        }
    }

    /// Glyphs for the pieces of a line's text.
    fn inline_glyphs(&self, range: Range<usize>, base: Style, dialogue: bool) -> Vec<Glyph> {
        let mut out = Vec::new();
        for p in lex::inline(self.text, range, &self.doc.cast, dialogue) {
            let s = &self.text[p.span.clone()];
            match &p.kind {
                Inline::Text => {
                    for (i, c) in s.char_indices() {
                        // A line break inside a joined block reads as a space.
                        let c = if c == '\n' || c == '\r' { ' ' } else { c };
                        out.push(Glyph::plain(c, base, p.span.start + i));
                    }
                }
                Inline::Ref(alias) => {
                    let name = self
                        .doc
                        .cast
                        .inline_name(alias)
                        .unwrap_or_else(|| alias.clone());
                    out.extend(chip(&name, Style::Ref, p.span.start));
                }
                Inline::Ic { inner } => {
                    out.push(Glyph::deco('(', Style::Direction, Some(p.span.start)));
                    // Macros inside a direction are live too.
                    out.extend(self.inline_glyphs(inner.clone(), Style::Direction, dialogue));
                    out.push(Glyph::deco(')', Style::Direction, Some(inner.end)));
                }
                Inline::Under { inner } => {
                    out.push(Glyph::deco('_', Style::Faint, Some(p.span.start)));
                    for (i, c) in self.text[inner.clone()].char_indices() {
                        out.push(Glyph::plain(c, Style::Under, inner.start + i));
                    }
                    out.push(Glyph::deco('_', Style::Faint, Some(inner.end)));
                }
                Inline::Wrap { name, inner } => {
                    let (open, close, style) = match name.as_str() {
                        "textit" | "emph" | "textbf" => ('‹', '›', Style::Direction),
                        _ => ('«', '»', base),
                    };
                    out.push(Glyph::deco(open, Style::Faint, Some(p.span.start)));
                    out.extend(self.inline_glyphs(inner.clone(), style, dialogue));
                    out.push(Glyph::deco(close, Style::Faint, Some(inner.end)));
                }
                Inline::Special if s.starts_with("\\\\") => {
                    // A line break inside a line of dialogue (verse).
                    out.push(Glyph::deco('\n', base, Some(p.span.start)));
                }
                Inline::Special => {
                    let shown = match s {
                        "--" => "–",
                        "---" => "—",
                        "~" => " ",
                        _ => s.get(1..).unwrap_or(s),
                    };
                    let shown = if shown.is_empty() { s } else { shown };
                    out.extend(chip(shown, base, p.span.start));
                }
                Inline::Raw => {
                    for (i, c) in s.char_indices() {
                        out.push(Glyph::plain(c, Style::Faint, p.span.start + i));
                    }
                }
            }
        }
        out
    }

    /// A block shown as the PDF reads it. False when it can only be raw.
    fn rendered(&mut self, b: usize) -> bool {
        let block = self.doc.blocks[b].clone();
        let span = block.span.clone();
        let lead = span.start
            + (self.text[span.clone()].len() - self.text[span.clone()].trim_start().len());
        let row = self.out.rows.len();
        match &block.kind {
            Kind::Blank => {
                self.ensure_row(row, b);
                self.stop(span.start, row, 0);
                if span.end > span.start {
                    self.stop(span.end, row, 0);
                }
            }
            Kind::Line { alias, text } => {
                let Some(c) = self.doc.cast.get(alias) else {
                    return false;
                };
                let style = if c.star {
                    Style::NameStar
                } else if matches!(c.kind, cast::Kind::Member | cast::Kind::Hidden) {
                    Style::NameMuted
                } else {
                    Style::Name
                };
                let name_w = self.out.name_width;
                let narrow = self.width < 60;
                let glyphs = self.inline_glyphs(text.clone(), Style::Text, true);
                if narrow {
                    // Name on its own row, text below it.
                    let name = chip(&format!("{}:", c.script), style, lead);
                    let used = self.flow(b, &name, 0, self.width, Align::Left, None, row);
                    self.flow(
                        b,
                        &glyphs,
                        2,
                        self.width,
                        Align::Left,
                        Some(text.end),
                        row + used,
                    );
                } else {
                    // The name wraps inside its column, as in the PDF.
                    let name = chip(&c.script, style, lead);
                    self.flow(
                        b,
                        &name,
                        0,
                        name_w.saturating_sub(1),
                        Align::Left,
                        None,
                        row,
                    );
                    self.flow_in_column(b, &glyphs, name_w, Some(text.end), row);
                }
            }
            Kind::Do { aliases, text } => {
                let names: Vec<String> = aliases
                    .iter()
                    .map(|a| self.doc.cast.inline_name(a).unwrap_or_else(|| a.clone()))
                    .collect();
                let mut glyphs = chip(&format!("({} ", names.join(", ")), Style::Direction, lead);
                glyphs.extend(self.inline_glyphs(text.clone(), Style::Direction, true));
                self.flow(b, &glyphs, 0, self.width, Align::Left, None, row);
                // The end of the text sits before the closing parenthesis.
                let (r, col) = self.last_pos(row);
                self.stop(text.end, r, col);
                self.out.rows[r].cells.push(Cell {
                    col,
                    ch: ')',
                    style: Style::Direction,
                    byte: None,
                });
            }
            Kind::Action { text } => {
                let mut glyphs = vec![Glyph::deco('(', Style::Direction, Some(lead))];
                glyphs.extend(self.inline_glyphs(text.clone(), Style::Direction, false));
                self.flow(b, &glyphs, 0, self.width, Align::Left, None, row);
                let (r, col) = self.last_pos(row);
                self.stop(text.end, r, col);
                self.out.rows[r].cells.push(Cell {
                    col,
                    ch: ')',
                    style: Style::Direction,
                    byte: None,
                });
                self.stop(span.end, r, col + 1);
            }
            Kind::Stage { text, right } => {
                let style = if *right {
                    Style::Direction
                } else {
                    Style::Text
                };
                let glyphs = self.inline_glyphs(text.clone(), style, false);
                let align = if *right { Align::Right } else { Align::Center };
                self.flow(b, &glyphs, 0, self.width, align, Some(text.end), row);
                self.stop(lead, row, 0);
            }
            Kind::Act { title, .. } => {
                let act = self
                    .doc
                    .act_of(b)
                    .map_or(0, |a| self.doc.outline.acts[a].number);
                // As the PDF sets it: "I. PERDE", the title on a line of its own.
                let head = format!(
                    "{}.  {}",
                    outline::roman(act),
                    spaced(lang::l("PERDE", "ACT"))
                );
                let mut glyphs = chip(&head, Style::Heading, lead);
                let title = title.trim();
                if !title.is_empty() {
                    glyphs.push(Glyph::deco('\n', Style::Heading, None));
                    glyphs.extend(title.chars().map(|c| Glyph::deco(c, Style::Heading, None)));
                }
                self.flow(
                    b,
                    &glyphs,
                    0,
                    self.width,
                    Align::Center,
                    Some(span.end),
                    row,
                );
            }
            Kind::Section { which, title, .. } => {
                let name = if title.trim().is_empty() {
                    match which {
                        lex::Section::Preface => lang::l("ÖNSÖZ", "PREFACE"),
                        lex::Section::Notes => lang::l("REJİSÖRE NOTLAR", "NOTES FOR THE DIRECTOR"),
                    }
                    .to_string()
                } else {
                    cast::upper_tr(title.trim())
                };
                let glyphs = chip(&name, Style::Heading, lead);
                let used = self.flow(
                    b,
                    &glyphs,
                    0,
                    self.width,
                    Align::Center,
                    Some(span.end),
                    row,
                );
                self.ensure_row(row + used, b);
            }
            Kind::Scene { place, .. } => {
                let scene = self.doc.scene_of(b).map(|s| &self.doc.outline.scenes[s]);
                let label = scene.map_or("?".to_string(), |s| s.label.clone());
                let mut s = format!("{} {label}", lang::l("Sahne", "Scene"));
                if !place.trim().is_empty() {
                    s.push_str(&format!(" — {}", place.trim()));
                }
                let glyphs = chip(&s, Style::Heading, lead);
                let used = self.flow(
                    b,
                    &glyphs,
                    0,
                    self.width,
                    Align::Center,
                    Some(span.end),
                    row,
                );
                // Who is in the scene, as the class lists it under the heading.
                if let Some(scene) = scene.filter(|s| !s.cast.is_empty()) {
                    let names: Vec<String> = scene
                        .cast
                        .iter()
                        .map(|a| self.doc.cast.get(a).map_or(a.clone(), |c| c.script.clone()))
                        .collect();
                    let glyphs: Vec<Glyph> = names
                        .join(", ")
                        .chars()
                        .map(|c| Glyph::deco(c, Style::SceneCast, None))
                        .collect();
                    self.flow(b, &glyphs, 0, self.width, Align::Center, None, row + used);
                }
            }
            Kind::Beat => {
                let glyphs = chip(lang::l("(Sus.)", "(Pause.)"), Style::Direction, lead);
                self.flow(b, &glyphs, 0, self.width, Align::Left, Some(span.end), row);
            }
            Kind::Unknown { .. } => {
                self.raw(b, Style::Warn);
            }
            Kind::Other => match self.text[span.clone()].trim() {
                "\\maketitle" => self.cover(b, lead, span.end),
                "\\kisiler" => self.cast_table(b, lead, span.end),
                cmd if is_page_break(cmd) => {
                    let s = lang::l("· yeni sayfa ·", "· new page ·");
                    let glyphs = chip(s, Style::Faint, lead);
                    self.flow(
                        b,
                        &glyphs,
                        0,
                        self.width,
                        Align::Center,
                        Some(span.end),
                        row,
                    );
                }
                _ => match latex_heading(self.text, span.clone()) {
                    Some(h) => {
                        let glyphs = chip(&cast::upper_tr(&h), Style::Heading, lead);
                        let used = self.flow(
                            b,
                            &glyphs,
                            0,
                            self.width,
                            Align::Center,
                            Some(span.end),
                            row,
                        );
                        self.ensure_row(row + used, b);
                    }
                    // Plain prose, a preface say: as text, its escapes shown
                    // as what they print.
                    None if !self.text[lead..span.end].starts_with('\\') => {
                        let glyphs = self.inline_glyphs(lead..span.end, Style::Text, false);
                        self.flow(b, &glyphs, 0, self.width, Align::Left, Some(span.end), row);
                    }
                    None => return false,
                },
            },
            Kind::Comment => return false,
        }
        true
    }

    /// Rows of decoration for a chip block. Its two stops, `lead` and `end`,
    /// go before and after row `at`, so the cursor sits on that row.
    fn chip_rows(
        &mut self,
        b: usize,
        rows: Vec<(Vec<Glyph>, Align, u16)>,
        at: usize,
        lead: usize,
        end: usize,
    ) {
        let mut row = self.out.rows.len();
        for (n, (mut glyphs, align, indent)) in rows.into_iter().enumerate() {
            if glyphs.is_empty() {
                self.ensure_row(row, b);
                row += 1;
                continue;
            }
            if n == at {
                glyphs[0].stop = Some(lead);
            }
            let width = self.width;
            let used = self.flow(b, &glyphs, indent, width, align, None, row);
            if n == at {
                let (r, col) = self.last_pos(row);
                let r = r.min(row + used - 1);
                self.stop(end, r, col);
            }
            row += used;
        }
    }

    /// `\\maketitle`: the cover in miniature, with room around it as on the page.
    fn cover(&mut self, b: usize, lead: usize, end: usize) {
        let get = |name| {
            info::get(self.text, name)
                .map(|v| v.replace("\\\\", " ").trim().to_string())
                .filter(|v| !v.is_empty())
        };
        let text = |s: &str, style| -> Vec<Glyph> {
            s.chars().map(|c| Glyph::deco(c, style, None)).collect()
        };
        let blank = || (Vec::new(), Align::Center, 0);
        let mut rows = vec![blank(), blank(), blank()];
        let title = self
            .doc
            .title(self.text)
            .unwrap_or_else(|| lang::l("(başlıksız)", "(untitled)").into());
        let title = cast::upper_tr(&title);
        // Letter-spaced when it fits on one row.
        let wide = spaced(&title);
        let shown = if wide.width() + 4 <= self.width as usize {
            wide
        } else {
            title
        };
        let at = rows.len();
        rows.push((text(&shown, Style::Title), Align::Center, 0));
        if let Some(a) = get("author") {
            rows.push(blank());
            rows.push((text(&a, Style::Text), Align::Center, 0));
        }
        rows.push(blank());
        if info::is_final(self.text) {
            let starred: Vec<String> = self
                .doc
                .cast
                .characters
                .iter()
                .filter(|c| c.star)
                .map(|c| c.display_name().to_string())
                .collect();
            for s in starred {
                rows.push((text(&s, Style::NameStar), Align::Center, 0));
            }
        } else {
            let mut d = lang::l("TASLAK", "DRAFT").to_string();
            if let Some(n) = get("draft") {
                d = format!("{d} {n}");
            }
            rows.push((text(&d, Style::SceneCast), Align::Center, 0));
        }
        let small: Vec<String> = [
            get("copyrightnotice").map(|c| format!("© {c}")),
            get("address"),
            get("telephone"),
            get("email"),
        ]
        .into_iter()
        .flatten()
        .collect();
        if !small.is_empty() {
            rows.extend([blank(), blank(), blank()]);
            for s in small {
                rows.push((text(&s, Style::Faint), Align::Right, 0));
            }
        }
        rows.push(blank());
        self.chip_rows(b, rows, at, lead, end);
    }

    /// `\\kisiler`: the cast table, names in a column, groups braced.
    fn cast_table(&mut self, b: usize, lead: usize, end: usize) {
        let cast = &self.doc.cast;
        let text = |s: &str, style| -> Vec<Glyph> {
            s.chars().map(|c| Glyph::deco(c, style, None)).collect()
        };
        let name_style = |c: &cast::Character| if c.star { Style::NameStar } else { Style::Name };
        // (name, its style, brace, description)
        let mut lines: Vec<(String, Style, Option<char>, String)> = Vec::new();
        for row in cast.rows() {
            match row {
                cast::Row::Character(i) => {
                    let c = &cast.characters[i];
                    let desc = c.desc.clone().unwrap_or_default();
                    lines.push((c.display_name().to_string(), name_style(c), None, desc));
                }
                cast::Row::Group(g) => {
                    let group = &cast.groups[g];
                    let n = group.members.len();
                    for (k, &m) in group.members.iter().enumerate() {
                        let c = &cast.characters[m];
                        let brace = match (n, k) {
                            (1, _) => '}',
                            (_, 0) => '┐',
                            (_, k) if k + 1 == n => '┘',
                            _ => '┤',
                        };
                        let desc = if k == (n - 1) / 2 {
                            group.desc.clone()
                        } else {
                            String::new()
                        };
                        lines.push((
                            c.display_name().to_string(),
                            name_style(c),
                            Some(brace),
                            desc,
                        ));
                    }
                }
            }
        }
        let names_w = lines.iter().map(|l| l.0.width()).max().unwrap_or(0) as u16;
        let col = (names_w + 3).min(self.width / 2);
        let descs_w = lines.iter().map(|l| l.3.width()).max().unwrap_or(0) as u16;
        let table_w = (col + descs_w).min(self.width);
        let indent = (self.width - table_w) / 2;
        let mut rows = vec![
            (Vec::new(), Align::Center, 0),
            (
                text(lang::l("KİŞİLER", "CAST"), Style::Heading),
                Align::Center,
                0,
            ),
            (Vec::new(), Align::Center, 0),
        ];
        for (name, style, brace, desc) in lines {
            let mut g = text(&name, style);
            let pad = (col as usize).saturating_sub(name.width() + 2);
            g.extend(text(&" ".repeat(pad.max(1)), Style::Text));
            match brace {
                Some(c) => g.push(Glyph::deco(c, Style::Faint, None)),
                None => g.push(Glyph::deco(' ', Style::Faint, None)),
            }
            g.push(Glyph::deco(' ', Style::Faint, None));
            // Spaces would let the row wrap anywhere: keep the columns with
            // non-breaking ones and let only the description wrap.
            for x in g.iter_mut() {
                if x.ch == ' ' {
                    x.ch = '\u{a0}';
                }
            }
            g.extend(text(&desc, Style::SceneCast));
            rows.push((g, Align::Left, indent));
        }
        rows.push((Vec::new(), Align::Center, 0));
        self.chip_rows(b, rows, 1, lead, end);
    }

    /// Text in the column right of the names, aligned with the first name row.
    fn flow_in_column(
        &mut self,
        b: usize,
        glyphs: &[Glyph],
        indent: u16,
        end: Option<usize>,
        row: usize,
    ) {
        // The name's rows are set aside, the text laid out from the same
        // row, and the name's cells put back in front.
        let saved = self.out.rows.split_off(row);
        self.flow(b, glyphs, indent, self.width, Align::Left, end, row);
        for (n, name_row) in saved.into_iter().enumerate() {
            self.ensure_row(row + n, b);
            let mut cells = name_row.cells;
            cells.extend(std::mem::take(&mut self.out.rows[row + n].cells));
            self.out.rows[row + n].cells = cells;
        }
    }

    /// The row and column right after the last cell from `row` on.
    fn last_pos(&self, row: usize) -> (usize, u16) {
        let r = self.out.rows.len().saturating_sub(1).max(row);
        let col = self
            .out
            .rows
            .get(r)
            .and_then(|x| x.cells.last())
            .map_or(0, |c| c.col + cw(c.ch));
        (r, col)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLAY: &str = include_str!("../examples/ornek-oyun.tex");

    fn row_text(l: &Layout, r: usize) -> String {
        let mut s = String::new();
        for c in &l.rows[r].cells {
            while s.chars().count() < c.col as usize {
                s.push(' ');
            }
            s.push(c.ch);
        }
        s.trim_end().to_string()
    }

    fn find_row(l: &Layout, needle: &str) -> usize {
        (0..l.rows.len())
            .find(|&r| row_text(l, r).contains(needle))
            .unwrap_or_else(|| panic!("no row with {needle:?}"))
    }

    fn lay(width: u16) -> (Doc, Layout) {
        let doc = Doc::new(PLAY);
        let l = layout(
            PLAY,
            &doc,
            Options {
                width,
                ..Options::default()
            },
        );
        (doc, l)
    }

    #[test]
    fn cover_and_cast_table_preview() {
        let (doc, l) = lay(70);
        // Room around the cover, the title letter-spaced.
        let title = find_row(&l, "K A R   M A S A L I");
        assert!(row_text(&l, title - 1).is_empty());
        assert!(row_text(&l, title + 2).contains("Örnek Yazar"));
        assert!(row_text(&l, title + 4).contains("TASLAK 0.1"));
        // Each contact line on its own row.
        assert!(row_text(&l, find_row(&l, "© 2026")).ends_with("© 2026 Örnek Yazar"));
        find_row(&l, "ornek@example.com");
        let cast = find_row(&l, "KİŞİLER");
        assert!(cast > title);
        // Full names from the cast table, descriptions in their own column.
        // Names keep their spaces unbreakable, so they never wrap.
        let ozan = find_row(&l, "Değirmenci, Selin");
        assert!(row_text(&l, ozan).contains("Ozan\u{a0}Usta"));
        let selin = find_row(&l, "Köy öğretmeni");
        let col = |r: usize, s: &str| {
            let t = row_text(&l, r);
            t[..t.find(s).unwrap()].chars().count()
        };
        assert_eq!(col(selin, "Köy"), col(ozan, "Değirmenci"));
        // A group: its members braced, its name beside them.
        let kadin = find_row(&l, "Köylüler");
        assert!(row_text(&l, kadin).contains("Köylüler"));
        assert!(row_text(&l, kadin + 1).contains("ÇOBAN"));
        // Hidden characters stay out, as in the PDF.
        assert!((cast..find_row(&l, "P E R D E")).all(|r| !row_text(&l, r).contains("HERKES")));
        // Each is one chip, its two stops on the title row.
        let mk = doc.parts.body.start + PLAY[doc.parts.body.clone()].find("\\maketitle").unwrap();
        let stops: Vec<_> = l
            .stops
            .iter()
            .filter(|s| s.byte >= mk && s.byte <= mk + 10)
            .collect();
        assert_eq!(stops.len(), 2);
        assert!(stops.iter().all(|s| s.row == title));
        // In final mode the cover names the starred characters instead.
        let fin = PLAY.replace("[draft]{tiyatro}", "[final]{tiyatro}");
        let d = Doc::new(&fin);
        let l = layout(
            &fin,
            &d,
            Options {
                width: 70,
                ..Options::default()
            },
        );
        let t = find_row(&l, "K A R   M A S A L I");
        assert!((t..t + 6).any(|r| row_text(&l, r).trim() == "Selin"));
        assert!((0..l.rows.len()).all(|r| !row_text(&l, r).contains("TASLAK")));
    }

    #[test]
    fn pages_curtains_and_sections() {
        let (_, l) = lay(70);
        let gaps: Vec<usize> = (0..l.rows.len()).filter(|&r| l.rows[r].gap).collect();
        // Between the cover and the preface (one page break, not two), and
        // before the cast, the notes and each act; two rows each.
        assert_eq!(gaps.len(), 2 * 5, "{gaps:?}");
        assert!(gaps.iter().all(|&r| l.rows[r].cells.is_empty()));
        assert!(!l.stops.iter().any(|s| l.rows[s.row].gap));
        assert!(l.rows[find_row(&l, "ÖNSÖZ") - 2].gap);
        assert!(l.rows[find_row(&l, "REJİSÖRE NOTLAR") - 2].gap);
        // The curtain before act II, and at the end.
        let curtains: Vec<usize> = (0..l.rows.len())
            .filter(|&r| row_text(&l, r).trim() == "PERDE SONU")
            .collect();
        assert_eq!(curtains.len(), 2);
        assert!(l.rows[curtains[0] + 1].gap);
        assert_eq!(row_text(&l, curtains[1] + 1).trim(), "SON");
        // Old LaTeX reads too.
        let old = "\\documentclass{tiyatro}\n\\begin{document}\nÖnce.\n\\clearpage\n\\section*{Ön söz}\nMetin.\n\\end{document}\n";
        let d = Doc::new(old);
        let l = layout(
            old,
            &d,
            Options {
                width: 60,
                ..Options::default()
            },
        );
        let head = find_row(&l, "ÖN SÖZ");
        assert!(l.rows[head - 2].gap);
        find_row(&l, "· yeni sayfa ·");
        // No pages in the all-source view.
        let l = layout(
            PLAY,
            &Doc::new(PLAY),
            Options {
                width: 70,
                raw_all: true,
                ..Options::default()
            },
        );
        assert!(!l.rows.iter().any(|r| r.gap));
    }

    #[test]
    fn reads_like_the_pdf() {
        let (_, l) = lay(80);
        assert!(row_text(&l, 0).starts_with("▸ Kişiler (6)"));
        let act = find_row(&l, "P E R D E");
        assert_eq!(row_text(&l, act).trim(), "I.  P E R D E");
        assert_eq!(row_text(&l, act + 1).trim(), "Kar Yağarken");
        assert!(row_text(&l, find_row(&l, "Sahne 1")).contains("Sahne 1 — İç, Değirmen — Akşam"));
        assert!(row_text(&l, find_row(&l, "Sahne 1") + 1).contains("SELİN, OZAN, İHTİYAR"));
        assert_eq!(
            row_text(&l, find_row(&l, "Amca")),
            "SELİN    Amca, yine mi taş durdu?"
        );
        assert!(
            row_text(&l, find_row(&l, "Durdu"))
                .contains("OZAN     Durdu kızım. (elini taşa koyar) Soğuk")
        );
        assert!(row_text(&l, find_row(&l, "Ama ")).contains("Ama Ozan Usta bu kış"));
        assert!(row_text(&l, find_row(&l, "Kapıyı")).contains("(İhtiyar Kapıyı çalar.)"));
        assert!(
            row_text(&l, find_row(&l, "bakarlar"))
                .contains("(Selin, Ozan Usta Birbirlerine bakarlar.)")
        );
        assert!(row_text(&l, find_row(&l, "Добрый")).contains("«Добрый вечер», dedi yolcu"));
        assert!(row_text(&l, find_row(&l, "Sahne 2.1")).contains("Sahne 2.1"));
    }

    #[test]
    fn macros_inside_macros() {
        let t = "\\karakter[x]{KADIN}{ka}\n\\begin{document}\n\\ka \\textit{0.001\\% of it} \\ic{bakar \\ka{}ya}\n\\end{document}\n";
        let doc = Doc::new(t);
        let l = layout(
            t,
            &doc,
            Options {
                width: 80,
                ..Options::default()
            },
        );
        assert_eq!(
            row_text(&l, find_row(&l, "0.001")),
            "KADIN  ‹0.001% of it› (bakar Kadınya)"
        );
    }

    #[test]
    fn margin_numbers_and_focus() {
        let doc = Doc::new(PLAY);
        let l = layout(
            PLAY,
            &doc,
            Options {
                width: 80,
                ..Options::default()
            },
        );
        let numbers: Vec<usize> = l.rows.iter().filter_map(|r| r.number).collect();
        assert_eq!(numbers, (1..=18).collect::<Vec<_>>());
        assert_eq!(l.rows[find_row(&l, "Amca")].number, Some(3));
        // Focus on the second scene: the first folds to one line.
        let l = layout(
            PLAY,
            &doc,
            Options {
                width: 80,
                focus_scene: Some(1),
                ..Options::default()
            },
        );
        assert!(
            row_text(&l, find_row(&l, "▸ Sahne 1"))
                .contains("▸ Sahne 1 — İç, Değirmen — Akşam · 6 replik")
        );
        assert!((0..l.rows.len()).all(|r| !row_text(&l, r).contains("Amca")));
        assert!(row_text(&l, find_row(&l, "Kar dinmeyecek")).contains("KADIN"));
    }

    #[test]
    fn verse_breaks() {
        let t = "\\karakter[x]{KADIN}{ka}\n\\begin{document}\n\\ka Bir.\\\\ İki.\\\\[1ex] Üç.\n\\end{document}\n";
        let doc = Doc::new(t);
        let l = layout(
            t,
            &doc,
            Options {
                width: 80,
                ..Options::default()
            },
        );
        let r = find_row(&l, "Bir.");
        assert_eq!(row_text(&l, r), "KADIN  Bir.");
        assert_eq!(row_text(&l, r + 1), "       İki.");
        assert_eq!(row_text(&l, r + 2), "       Üç.");
    }

    #[test]
    fn long_lines_wrap_under_the_text() {
        let (_, l) = lay(62);
        assert_eq!(l.name_width, 9);
        let r = find_row(&l, "Durdu");
        assert_eq!(
            row_text(&l, r),
            "OZAN     Durdu kızım. (elini taşa koyar) Soğuk her şeyi"
        );
        assert_eq!(row_text(&l, r + 1), "         durdurur.");
    }

    #[test]
    fn narrow_screens_put_the_name_above() {
        let doc = Doc::new(PLAY);
        let l = layout(
            PLAY,
            &doc,
            Options {
                width: 40,
                ..Options::default()
            },
        );
        let r = find_row(&l, "SELİN:");
        assert!(row_text(&l, r + 1).starts_with("  Amca"));
    }

    #[test]
    fn every_stop_is_a_char_boundary_in_order() {
        for width in [30, 50, 80, 120] {
            let (_, l) = lay(width);
            assert!(!l.stops.is_empty());
            assert!(l.stops.windows(2).all(|w| w[0].byte < w[1].byte));
            for s in &l.stops {
                assert!(PLAY.is_char_boundary(s.byte), "{}", s.byte);
                assert!(s.row < l.rows.len());
            }
        }
    }

    #[test]
    fn chips_are_atomic() {
        let (_, l) = lay(80);
        let at = PLAY.find("\\se Amca").unwrap();
        // The line's start and the text's start are stops; nothing between.
        assert!(l.is_stop(at));
        assert!(l.is_stop(at + 4));
        assert!(!l.is_stop(at + 1) && !l.is_stop(at + 3));
        // Inside the text every character is a stop.
        assert!(l.is_stop(at + 5));
        // \ic{…}: before the macro, inside it, and its end.
        let ic = PLAY.find("\\ic{elini").unwrap();
        assert!(l.is_stop(ic) && l.is_stop(ic + 4) && !l.is_stop(ic + 1));
    }

    #[test]
    fn raw_block_shows_source() {
        let doc = Doc::new(PLAY);
        let at = PLAY.find("\\se Amca").unwrap();
        let b = doc.block_at(at).unwrap();
        let l = layout(
            PLAY,
            &doc,
            Options {
                width: 80,
                raw_block: Some(b),
                ..Options::default()
            },
        );
        assert_eq!(
            row_text(&l, find_row(&l, "Amca,")),
            "\\se Amca, yine mi taş durdu?"
        );
        assert!(l.is_stop(at + 1));
    }

    #[test]
    fn preamble_opens() {
        let doc = Doc::new(PLAY);
        let l = layout(
            PLAY,
            &doc,
            Options {
                width: 80,
                preamble_open: true,
                ..Options::default()
            },
        );
        assert_eq!(
            row_text(&l, 0),
            "% Örnek oyun — Dactile ile yazılmış, tamamen uydurma bir metin."
        );
        assert!(l.is_stop(1));
    }

    #[test]
    fn real_text_fits_any_width() {
        // A long word and a long name in a narrow column.
        let t = "\\karakter[x]{SPİKER YAHUT MİKAİL}{mi}\n\\begin{document}\n\\mi Çokçokçokçokçokçokçokuzunbirkelime ve daha\n\\end{document}\n";
        let doc = Doc::new(t);
        for w in [20, 30, 45, 61, 100] {
            let l = layout(
                t,
                &doc,
                Options {
                    width: w,
                    ..Options::default()
                },
            );
            for r in &l.rows {
                for c in &r.cells {
                    assert!(c.col < w.max(20) + 2, "width {w}: col {}", c.col);
                }
            }
        }
    }
}
