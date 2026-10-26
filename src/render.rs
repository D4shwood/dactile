//! The live view: a play's text laid out the way the PDF reads, with every
//! place the cursor can stand mapped back to a byte of the source.
//!
//! A macro shows as what it means: `\se Amca` is "SELİN  Amca", `\ic{güler}`
//! is "(güler)". Such a piece is a chip: the cursor steps over it as one unit,
//! and editing it opens the block as raw source (`Options::raw_block`).
//! Blocks the app doesn't understand are always shown raw.

use crate::cast::{self, Cast};
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
    for i in 0..doc.blocks.len() {
        let start = b.out.rows.len();
        let raw = opts.raw_all || opts.raw_block == Some(i);
        if doc.in_preamble(i) && !opts.preamble_open && !opts.raw_all {
            // One summary row for the whole preamble.
            if i == 0 {
                let n = doc.cast.characters.len();
                let s = format!("▸ Kişiler ({n}) · oyun bilgisi");
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
                let s = format!("▸ Sahne {}{place} · {} replik", scene.label, scene.lines);
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
    }
    b.out.stops.sort_by_key(|s| s.byte);
    // One stop per byte: the first placed wins (a block's own start over the
    // end of the one before).
    b.out.stops.dedup_by_key(|s| s.byte);
    b.out
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
            });
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
            Kind::Act { title } => {
                let act = self
                    .doc
                    .act_of(b)
                    .map_or(0, |a| self.doc.outline.acts[a].number);
                let mut s = format!("{}. PERDE", outline::roman(act));
                if !title.trim().is_empty() {
                    s.push_str(&format!(" · {}", title.trim()));
                }
                let glyphs = chip(&s, Style::Heading, lead);
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
            Kind::Scene { place, .. } => {
                let scene = self.doc.scene_of(b).map(|s| &self.doc.outline.scenes[s]);
                let label = scene.map_or("?".to_string(), |s| s.label.clone());
                let mut s = format!("Sahne {label}");
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
                let glyphs = chip("(Sus.)", Style::Direction, lead);
                self.flow(b, &glyphs, 0, self.width, Align::Left, Some(span.end), row);
            }
            Kind::Unknown { .. } => {
                self.raw(b, Style::Warn);
            }
            Kind::Comment | Kind::Other => return false,
        }
        true
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
    fn reads_like_the_pdf() {
        let (_, l) = lay(80);
        assert!(row_text(&l, 0).starts_with("▸ Kişiler (6)"));
        assert!(row_text(&l, find_row(&l, "PERDE")).contains("I. PERDE · Kar Yağarken"));
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
