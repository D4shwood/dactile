//! Notes on a play from anyone (a reader, a workshop, an AI): kept beside it
//! in `oyun.notlar.md`, a Markdown task list that reads as well in any
//! editor (Obsidian too) as in Dactile:
//!
//! ```text
//! ## Spelling
//!
//! - [ ] 139 · **Claude** · “Gaztesi” → Gazetesi
//!   - **Salih**: done in the second act too
//! - [x] 20 · **Workshop** · “loş bir bir ışığın”: “bir” twice
//! ```
//!
//! `[ ]` open, `[x]` done, `[-]` hidden; under `## Arşiv: …` archived. The
//! number is a line of the play; the note finds its place again by what it
//! quotes, the number only says where to look first.

use crate::tex::Edit;
use anyhow::{Context, Result};
use std::ops::Range;
use std::path::{Path, PathBuf};

/// A quote this many lines from its note's number is still its place.
const NEAR: usize = 40;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum State {
    #[default]
    Open,
    Done,
    Hidden,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Reply {
    pub author: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Note {
    pub state: State,
    pub archived: bool,
    /// The play's line it is about (from 1), as last seen.
    pub line: Option<usize>,
    pub author: String,
    pub text: String,
    pub replies: Vec<Reply>,
    /// The heading it came under ("Spelling"), or none.
    pub group: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Notes {
    pub list: Vec<Note>,
}

/// `oyun.tex` → `oyun.notlar.md`.
pub fn path_for(play: &Path) -> PathBuf {
    let stem = play
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    play.with_file_name(format!("{stem}.notlar.md"))
}

impl Notes {
    /// The play's notes, or none.
    pub fn load(play: &Path) -> Result<Notes> {
        let path = path_for(play);
        match std::fs::read_to_string(&path) {
            Ok(t) => Ok(Notes::parse(&t)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Notes::default()),
            Err(e) => Err(e).with_context(|| format!("reading {}", path.display())),
        }
    }

    /// Writes them, or removes the file when there are none.
    pub fn save(&self, play: &Path) -> Result<()> {
        let path = path_for(play);
        if self.list.is_empty() {
            if path.exists() {
                std::fs::remove_file(&path)?;
            }
            return Ok(());
        }
        let name = play
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        crate::file::write_atomic(&path, &self.to_markdown(&name))
    }

    pub fn parse(text: &str) -> Notes {
        let mut list: Vec<Note> = Vec::new();
        let mut group = String::new();
        let mut archived = false;
        // Whether the last thing read was a note or a reply, for the lines
        // that go on from it.
        let mut in_reply = false;
        let mut started = false;
        for raw in text.lines() {
            let line = raw.trim_end();
            if let Some(h) = line.strip_prefix("## ") {
                let h = h.trim();
                let (arch, rest) = archive_heading(h);
                archived = arch;
                group = rest.to_string();
                started = true;
                continue;
            }
            if line.starts_with("# ") || line.trim_start().starts_with("<!--") {
                continue;
            }
            let indent = line.len() - line.trim_start().len();
            let body = line.trim_start();
            if body.is_empty() {
                continue;
            }
            if indent == 0
                && let Some(rest) = body.strip_prefix("- ").or_else(|| body.strip_prefix("* "))
            {
                let (state, rest) = checkbox(rest);
                let (line_no, rest) = number(rest);
                let (author, rest) = author(rest);
                list.push(Note {
                    state,
                    archived,
                    line: line_no,
                    author,
                    text: rest.trim().to_string(),
                    replies: Vec::new(),
                    group: group.clone(),
                });
                in_reply = false;
                started = true;
                continue;
            }
            let Some(note) = list.last_mut().filter(|_| started && indent > 0) else {
                continue;
            };
            if let Some(rest) = body.strip_prefix("- ").or_else(|| body.strip_prefix("* ")) {
                let (author, rest) = author(rest);
                note.replies.push(Reply {
                    author,
                    text: rest.trim().to_string(),
                });
                in_reply = true;
            } else {
                let to = if in_reply {
                    note.replies.last_mut().map(|r| &mut r.text)
                } else {
                    Some(&mut note.text)
                };
                if let Some(t) = to {
                    if !t.is_empty() {
                        t.push(' ');
                    }
                    t.push_str(body);
                }
            }
        }
        Notes { list }
    }

    pub fn to_markdown(&self, play_name: &str) -> String {
        let tr = crate::lang::is_tr();
        let mut out = format!(
            "# {}: {play_name}\n\n<!-- {} -->\n",
            if tr { "Notlar" } else { "Notes" },
            if tr {
                "Dactile: [ ] açık, [x] çözüldü, [-] gizli; satır · **kimden** · not; cevaplar altında girintili."
            } else {
                "Dactile: [ ] open, [x] done, [-] hidden; line · **who** · note; replies indented below."
            }
        );
        let archive = if tr { "Arşiv" } else { "Archive" };
        // Live notes first, by group in the order they came; then the archive.
        for arch in [false, true] {
            let mut groups: Vec<&str> = Vec::new();
            for n in self.list.iter().filter(|n| n.archived == arch) {
                if !groups.contains(&n.group.as_str()) {
                    groups.push(&n.group);
                }
            }
            // Notes without a heading go before any: else they'd read as
            // under the one before them.
            groups.sort_by_key(|g| !g.is_empty());
            for g in groups {
                out.push('\n');
                match (arch, g.is_empty()) {
                    (false, true) => {}
                    (false, false) => out.push_str(&format!("## {g}\n\n")),
                    (true, true) => out.push_str(&format!("## {archive}\n\n")),
                    (true, false) => out.push_str(&format!("## {archive}: {g}\n\n")),
                }
                for n in self
                    .list
                    .iter()
                    .filter(|n| n.archived == arch && n.group == g)
                {
                    out.push_str(&n.to_markdown());
                }
            }
        }
        out
    }

    /// Who wrote notes, in the order they first did: each one's colour.
    pub fn authors(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let names = self
            .list
            .iter()
            .flat_map(|n| std::iter::once(&n.author).chain(n.replies.iter().map(|r| &r.author)));
        for a in names {
            if !a.is_empty() && !out.contains(a) {
                out.push(a.clone());
            }
        }
        out
    }
}

impl Note {
    pub fn to_markdown(&self) -> String {
        let mark = match self.state {
            State::Open => ' ',
            State::Done => 'x',
            State::Hidden => '-',
        };
        let mut s = format!("- [{mark}] ");
        if let Some(l) = self.line {
            s.push_str(&format!("{l} · "));
        }
        if !self.author.is_empty() {
            s.push_str(&format!("**{}** · ", self.author));
        }
        s.push_str(&one_line(&self.text));
        s.push('\n');
        for r in &self.replies {
            if r.author.is_empty() {
                s.push_str(&format!("  - {}\n", one_line(&r.text)));
            } else {
                s.push_str(&format!("  - **{}**: {}\n", r.author, one_line(&r.text)));
            }
        }
        s
    }

    /// What it quotes of the play: “…”, "…", «…».
    pub fn quotes(&self) -> Vec<String> {
        quotes(&self.text)
    }

    /// The corrections it suggests: `“Gaztesi” → Gazetesi`.
    pub fn fixes(&self) -> Vec<Fix> {
        fixes(&self.text)
    }
}

fn one_line(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// `Arşiv: Yazım` → (true, "Yazım"); any other heading is a group.
fn archive_heading(h: &str) -> (bool, &str) {
    for w in ["Arşiv", "Archive", "arşiv", "archive"] {
        if let Some(rest) = h.strip_prefix(w) {
            let rest = rest.trim_start();
            if rest.is_empty() {
                return (true, "");
            }
            if let Some(g) = rest.strip_prefix(':').or_else(|| rest.strip_prefix('—')) {
                return (true, g.trim());
            }
        }
    }
    (false, h)
}

fn checkbox(s: &str) -> (State, &str) {
    let b = s.as_bytes();
    if b.len() >= 3 && b[0] == b'[' && b[2] == b']' {
        let state = match b[1] {
            b'x' | b'X' => State::Done,
            b'-' | b'~' => State::Hidden,
            _ => State::Open,
        };
        return (state, s[3..].trim_start());
    }
    (State::Open, s)
}

/// `139 · rest` → (139, rest).
fn number(s: &str) -> (Option<usize>, &str) {
    let digits = s.chars().take_while(char::is_ascii_digit).count();
    if digits > 0 && digits <= 6 {
        let rest = s[digits..].trim_start();
        if let Some(r) = rest.strip_prefix('·') {
            return (s[..digits].parse().ok(), r.trim_start());
        }
    }
    (None, s)
}

/// `**Claude** · rest` or `**Claude**: rest` → ("Claude", rest).
fn author(s: &str) -> (String, &str) {
    if let Some(rest) = s.strip_prefix("**")
        && let Some(end) = rest.find("**")
    {
        let name = rest[..end].trim();
        let after = rest[end + 2..].trim_start();
        if let Some(r) = after.strip_prefix('·').or_else(|| after.strip_prefix(':')) {
            return (name.to_string(), r.trim_start());
        }
    }
    (String::new(), s)
}

// ------------------------------------------------------------- a reply pasted

/// Notes out of a reply pasted from somewhere (an AI's, a reader's mail):
/// `139: …`, `- **20:** …`, `• 68 (gazete metni): …` each a note, by
/// `author`; a short line before a run of them names their group. What is
/// neither is left out.
pub fn from_reply(text: &str, author: &str) -> Vec<Note> {
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    let mut out: Vec<Note> = Vec::new();
    let mut group = String::new();
    // Whether the line before was a note's, so a line right under it goes on.
    let mut after_item = false;
    for (i, &l) in lines.iter().enumerate() {
        if l.is_empty() {
            after_item = false;
            continue;
        }
        if let Some((line, said)) = item(l) {
            out.push(Note {
                line,
                author: author.to_string(),
                text: said,
                group: group.clone(),
                ..Note::default()
            });
            after_item = true;
            continue;
        }
        let next_is_item = lines[i + 1..]
            .iter()
            .find(|l| !l.is_empty())
            .is_some_and(|l| item(l).is_some());
        if let Some(g) = group_line(l)
            && (next_is_item || l.starts_with('#'))
        {
            group = g;
            after_item = false;
            continue;
        }
        if after_item && let Some(n) = out.last_mut() {
            n.text.push(' ');
            n.text.push_str(&plain_md(l));
        }
    }
    out
}

/// A note's line in a reply: its line number, if it has one, and what it
/// says. Bullets without a number are notes too.
fn item(l: &str) -> Option<(Option<usize>, String)> {
    let mut s = l;
    let mut bullet = false;
    for b in ["- ", "* ", "• ", "– ", "+ "] {
        if let Some(r) = s.strip_prefix(b) {
            s = r.trim_start();
            bullet = true;
            break;
        }
    }
    let s = s.strip_prefix("**").unwrap_or(s);
    let digits = s.chars().take_while(char::is_ascii_digit).count();
    if digits > 0 && digits <= 5 {
        let n: usize = s[..digits].parse().ok()?;
        let mut rest = &s[digits..];
        // 139-141, 139–141, 139, 141: the first is enough.
        loop {
            let t = rest.trim_start_matches(['-', '–', ',', ' ']);
            let more = t.chars().take_while(char::is_ascii_digit).count();
            if more > 0 && t.len() < rest.len() {
                rest = &t[more..];
            } else {
                break;
            }
        }
        let rest = rest.strip_prefix("**").unwrap_or(rest);
        // A note on it in brackets: "103 (gazete metni):".
        let (aside, rest) = match rest.trim_start().strip_prefix('(') {
            Some(r) => match r.find(')') {
                Some(end) => (Some(&r[..end]), &r[end + 1..]),
                None => (None, rest),
            },
            None => (None, rest),
        };
        let rest = rest.strip_prefix("**").unwrap_or(rest);
        let rest = rest.trim_start();
        for sep in [":**", ":", ".", ")", "—", "–", "-"] {
            if let Some(r) = rest.strip_prefix(sep)
                && (sep != "." || r.starts_with(' '))
            {
                let r = r.strip_prefix("**").unwrap_or(r);
                let mut said = plain_md(r.trim());
                if let Some(a) = aside {
                    said = format!("({a}) {said}");
                }
                if said.is_empty() {
                    return None;
                }
                return Some((Some(n), said));
            }
        }
        // "74 “Ben gidiyorum” bir tekrar": a number, then a quote.
        if s[digits..].starts_with(char::is_whitespace) && rest.starts_with(OPEN) {
            return Some((Some(n), plain_md(rest)));
        }
        if bullet {
            return Some((None, plain_md(l.get(2..).unwrap_or("").trim())));
        }
        return None;
    }
    bullet.then(|| (None, plain_md(s.trim())))
}

/// A group's name: `## Spelling`, `**Spelling**`, or a short line on its
/// own.
fn group_line(l: &str) -> Option<String> {
    let t = l.trim_start_matches('#').trim();
    let t = t.trim_matches('*').trim().trim_end_matches(':').trim();
    if t.is_empty() || t.chars().count() > 70 {
        return None;
    }
    // A sentence is not a heading.
    if !l.starts_with('#') && (t.ends_with('.') && !t.ends_with("...")) {
        return None;
    }
    Some(t.to_string())
}

/// `**bold**` and `` `code` `` marks off: the words stay.
fn plain_md(s: &str) -> String {
    s.replace("**", "").replace("__", "")
}

// ------------------------------------------------------------- quotes, fixes

const OPEN: [char; 4] = ['“', '"', '«', '„'];

fn closing(c: char) -> char {
    match c {
        '“' | '„' => '”',
        '«' => '»',
        _ => '"',
    }
}

/// What a note quotes, in order; a quote cut short ("St. Peterb…") without
/// its dots.
pub fn quotes(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(i) = rest.find(OPEN) {
        let open = rest[i..].chars().next().unwrap_or('"');
        let after = &rest[i + open.len_utf8()..];
        let close = closing(open);
        // A straight quote closes with a straight one; a curly opening may
        // be closed straight by a careless hand.
        let end = after
            .find(close)
            .or_else(|| (open != '"').then(|| after.find('"')).flatten());
        let Some(end) = end else { break };
        let q = after[..end]
            .trim()
            .trim_end_matches('…')
            .trim_end_matches("...")
            .trim();
        if !q.is_empty() && q.chars().count() <= 300 {
            out.push(q.to_string());
        }
        let close_len = after[end..].chars().next().map_or(1, char::len_utf8);
        rest = &after[end + close_len..];
    }
    out
}

/// A correction a note suggests: these words of the play for those.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fix {
    pub from: String,
    pub to: String,
}

/// `“Gaztesi” → Gazetesi`, `"vatan’a" -> vatana;`, `dikeleniyo → dikleniyo`:
/// each arrow with what is before it (a quote, else a word) and after it (a
/// quote, else words up to a stop).
pub fn fixes(text: &str) -> Vec<Fix> {
    let mut out = Vec::new();
    let mut from_at = 0;
    loop {
        let rest = &text[from_at..];
        let found = ["→", "->", "⇒", "=>"]
            .iter()
            .filter_map(|a| rest.find(a).map(|i| (i, a.len())))
            .min();
        let Some((i, alen)) = found else { break };
        let at = from_at + i;
        let before = text[..at].trim_end();
        let after = text[at + alen..].trim_start();
        from_at = at + alen;
        let Some(from) = left_of(before) else {
            continue;
        };
        let Some(to) = right_of(after) else { continue };
        if from != to && !from.is_empty() && !to.is_empty() {
            out.push(Fix { from, to });
        }
    }
    out
}

fn left_of(before: &str) -> Option<String> {
    let last = before.chars().last()?;
    if let Some(open) = OPEN
        .iter()
        .find(|&&o| closing(o) == last || last == '"' && o == '"')
    {
        let inner = &before[..before.len() - last.len_utf8()];
        let start = inner.rfind(*open).or_else(|| inner.rfind(OPEN))?;
        let open_len = inner[start..].chars().next()?.len_utf8();
        let q = inner[start + open_len..].trim();
        return (!q.is_empty()).then(|| q.to_string());
    }
    let w = before
        .rsplit(char::is_whitespace)
        .next()?
        .trim_matches(|c: char| !(c.is_alphanumeric() || c == '\'' || c == '’'));
    (!w.is_empty()).then(|| w.to_string())
}

fn right_of(after: &str) -> Option<String> {
    let first = after.chars().next()?;
    if OPEN.contains(&first) {
        let inner = &after[first.len_utf8()..];
        let end = inner.find(closing(first)).or_else(|| inner.find('"'))?;
        let q = inner[..end].trim();
        return (!q.is_empty()).then(|| q.to_string());
    }
    // Bare words, up to a stop: a dot ends them unless it ends a short
    // abbreviation ("St. Petersburg").
    let mut end = after.len();
    for (i, c) in after.char_indices() {
        if matches!(c, ';' | ',' | '(' | ')' | '\n' | ':') {
            end = i;
            break;
        }
        if c == '.' {
            let word = after[..i].rsplit(' ').next().unwrap_or("");
            let next = after[i + 1..].chars().next();
            if next.is_none_or(char::is_whitespace) && word.chars().count() > 3 {
                end = i;
                break;
            }
        }
    }
    let to = after[..end].trim().trim_end_matches('.');
    let words = to.split_whitespace().count();
    (words > 0 && words <= 6).then(|| to.to_string())
}

// ------------------------------------------------------------- the play

/// The play's text made easy to look for quotes in: every run of spaces
/// and line ends one space, ’ an apostrophe, `~` a space; where each folded
/// byte came from.
pub struct Index {
    folded: String,
    starts: Vec<usize>,
    ends: Vec<usize>,
    lines: Vec<usize>,
    len: usize,
}

fn fold_char(c: char) -> char {
    match c {
        '’' | '‘' | '`' | '´' => '\'',
        '~' => ' ',
        c if c.is_whitespace() => ' ',
        c => c,
    }
}

fn fold(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars().map(fold_char) {
        if c == ' ' && (out.is_empty() || out.ends_with(' ')) {
            continue;
        }
        out.push(c);
    }
    out.trim_end().to_string()
}

/// Where a note is in the play.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Anchor {
    pub range: Range<usize>,
    /// Found by what it quotes, not only by its line.
    pub quoted: bool,
    pub line: usize,
}

impl Index {
    pub fn new(text: &str) -> Index {
        let mut folded = String::with_capacity(text.len());
        let mut starts = Vec::with_capacity(text.len());
        let mut ends = Vec::with_capacity(text.len());
        let mut lines = vec![0];
        for (i, c) in text.char_indices() {
            if c == '\n' {
                lines.push(i + 1);
            }
            let f = fold_char(c);
            let end = i + c.len_utf8();
            if f == ' ' && folded.ends_with(' ') {
                // The run of spaces grows.
                if let Some(e) = ends.last_mut() {
                    *e = end;
                }
                continue;
            }
            folded.push(f);
            for _ in 0..f.len_utf8() {
                starts.push(i);
                ends.push(end);
            }
        }
        Index {
            folded,
            starts,
            ends,
            lines,
            len: text.len(),
        }
    }

    /// The line (from 1) `byte` is on.
    pub fn line_of(&self, byte: usize) -> usize {
        self.lines.partition_point(|&s| s <= byte)
    }

    /// The bytes of line `n` (from 1), without its line end.
    pub fn line_range(&self, n: usize) -> Option<Range<usize>> {
        let start = *self.lines.get(n.checked_sub(1)?)?;
        let end = self.lines.get(n).map_or(self.len, |&e| e - 1);
        Some(start..end.max(start))
    }

    /// Every place `q` is, as byte ranges of the play.
    fn find(&self, q: &str) -> Vec<Range<usize>> {
        let q = fold(q);
        if q.is_empty() {
            return Vec::new();
        }
        self.folded
            .match_indices(&q)
            .map(|(i, m)| self.starts[i]..self.ends[i + m.len() - 1])
            .collect()
    }

    /// The place of `q` nearest line `near`, and how far it is in lines.
    fn nearest(&self, q: &str, near: Option<usize>) -> Option<(Range<usize>, usize)> {
        let all = self.find(q);
        match near {
            None => all.into_iter().next().map(|r| (r, 0)),
            Some(n) => all
                .into_iter()
                .map(|r| {
                    let d = self.line_of(r.start).abs_diff(n);
                    (r, d)
                })
                .min_by_key(|(_, d)| *d),
        }
    }

    /// Where the note is: the first thing it quotes that is near its line,
    /// else the first found anywhere, else its line.
    pub fn anchor(&self, note: &Note) -> Option<Anchor> {
        let mut far: Option<Range<usize>> = None;
        for q in note.quotes() {
            if let Some((r, d)) = self.nearest(&q, note.line) {
                if note.line.is_none() || d <= NEAR {
                    return Some(self.anchor_at(r, true));
                }
                far.get_or_insert(r);
            }
        }
        // A word fixed by an arrow without quotes: "dikeleniyo → dikleniyo".
        for f in note.fixes() {
            if let Some((r, d)) = self.nearest(&f.from, note.line)
                && (note.line.is_none() || d <= NEAR)
            {
                return Some(self.anchor_at(r, true));
            }
        }
        if let Some(r) = far {
            return Some(self.anchor_at(r, true));
        }
        let line = note.line?;
        let r = self.line_range(line.min(self.lines.len()))?;
        Some(Anchor {
            range: r,
            quoted: false,
            line,
        })
    }

    fn anchor_at(&self, range: Range<usize>, quoted: bool) -> Anchor {
        Anchor {
            line: self.line_of(range.start),
            range,
            quoted,
        }
    }

    /// Whether what the note points at seems put right: the words its
    /// arrows replace, else the first thing it quotes, no longer near its
    /// line.
    pub fn gone(&self, note: &Note) -> bool {
        let fixes = note.fixes();
        let near = |q: &str| {
            self.nearest(q, note.line)
                .is_some_and(|(_, d)| note.line.is_none() || d <= NEAR)
        };
        if !fixes.is_empty() {
            return fixes.iter().all(|f| !near(&f.from));
        }
        match note.quotes().first() {
            Some(q) => !near(q),
            None => false,
        }
    }

    /// The edits that make the note's corrections, each where its words are
    /// nearest the note's line. Only the words that differ change:
    /// `“kalbinin tamamen duyduğu işitilir” → “durduğu”` changes one word.
    pub fn fix_edits(&self, note: &Note) -> Vec<Edit> {
        let mut out: Vec<Edit> = Vec::new();
        for f in note.fixes() {
            let from = fold(&f.from);
            let Some((at, _)) = self.nearest(&from, note.line) else {
                continue;
            };
            // The match in folded bytes, to map words back.
            let fi = self.starts.partition_point(|&s| s < at.start);
            let (a, b, to) = narrow(&from, &f.to);
            let range = if b > a {
                self.starts[fi + a]..self.ends[fi + b - 1]
            } else if a > 0 {
                let p = self.ends[fi + a - 1];
                p..p
            } else {
                at.start..at.start
            };
            let to = crate::buffer::escape_pasted(&to, true);
            if !out
                .iter()
                .any(|e| e.range.start < range.end && range.start < e.range.end)
            {
                out.push(Edit::new(range, to));
            }
        }
        out
    }
}

/// The part of `from` (folded) that `to` changes, as folded byte offsets,
/// and what goes there.
fn narrow(from: &str, to: &str) -> (usize, usize, String) {
    let fw: Vec<(usize, &str)> = words(from);
    let tw: Vec<&str> = to.split_whitespace().collect();
    let mut p = 0;
    while p < fw.len() && p < tw.len() && fw[p].1 == tw[p] {
        p += 1;
    }
    let mut s = 0;
    while s < fw.len() - p && s < tw.len() - p && fw[fw.len() - 1 - s].1 == tw[tw.len() - 1 - s] {
        s += 1;
    }
    // One word for a phrase that shares none of it: the word most like it.
    if p == 0 && s == 0 && tw.len() == 1 && fw.len() > 1 {
        let core = |w: &str| {
            w.trim_matches(|c: char| !c.is_alphanumeric())
                .to_lowercase()
        };
        let want = core(tw[0]);
        let best = fw
            .iter()
            .enumerate()
            .map(|(i, (_, w))| (i, distance(&core(w), &want)))
            .min_by_key(|&(_, d)| d);
        if let Some((i, d)) = best
            && d * 2 <= want.chars().count().max(1)
        {
            let (at, w) = fw[i];
            // The word's letters, its punctuation kept.
            let lead = w.len() - w.trim_start_matches(|c: char| !c.is_alphanumeric()).len();
            let core_len = w.trim_matches(|c: char| !c.is_alphanumeric()).len();
            let word = tw[0].trim_matches(|c: char| !c.is_alphanumeric());
            return (at + lead, at + lead + core_len, word.to_string());
        }
    }
    let mid_to = tw[p..tw.len() - s].join(" ");
    let fe = fw.len() - s;
    if p < fe {
        let start = fw[p].0;
        let end = fw[fe - 1].0 + fw[fe - 1].1.len();
        (start, end, mid_to)
    } else {
        // Only added words: after the last shared one.
        let at = if p > 0 {
            fw[p - 1].0 + fw[p - 1].1.len()
        } else {
            0
        };
        let text = if p > 0 {
            format!(" {mid_to}")
        } else {
            format!("{mid_to} ")
        };
        (at, at, text)
    }
}

/// Each author's colour on paper, in the order they first wrote.
pub const COLORS: [&str; 6] = ["B0444A", "3F7F7A", "B0703A", "4F6AA8", "6B8540", "8A5E98"];

/// Where the notes copy of a play goes: `oyun - notlu.pdf` beside it.
pub fn copy_path(play: &Path) -> PathBuf {
    let stem = play
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let word = if crate::lang::is_tr() {
        "notlu"
    } else {
        "notes"
    };
    play.with_file_name(format!("{stem} - {word}.pdf"))
}

/// The play with its open notes (and the done ones, with `done`) in the
/// margin, each beside its line and numbered by where it is; at the end
/// all of them with their replies. Notes on the cover or a heading only
/// go in that list.
pub fn copy_tex(text: &str, doc: &crate::render::Doc, notes: &Notes, done: bool) -> String {
    use crate::casting::tex_text;
    use crate::lex::Kind;
    let ix = Index::new(text);
    let authors = notes.authors();
    let color = |a: &str| {
        let i = authors.iter().position(|x| x == a).unwrap_or(0);
        COLORS[i % COLORS.len()]
    };
    let who = |a: &str| {
        if a.is_empty() {
            "?".to_string()
        } else {
            tex_text(a)
        }
    };
    let body = &doc.parts.body;
    // (where in the text, if beside a line; the note)
    let mut taken: Vec<(Option<usize>, Option<Anchor>, &Note)> = notes
        .list
        .iter()
        .filter(|n| !n.archived && (n.state == State::Open || done && n.state == State::Done))
        .map(|n| {
            let a = ix.anchor(n);
            let at = a.as_ref().and_then(|a| {
                if !(body.start..body.end).contains(&a.range.start) {
                    return None;
                }
                let b = doc.block_at(a.range.start)?;
                let words = match &doc.blocks[b].kind {
                    Kind::Line { text, .. }
                    | Kind::Do { text, .. }
                    | Kind::Action { text }
                    | Kind::Stage { text, .. } => text.clone(),
                    _ => return None,
                };
                if words.contains(&a.range.start) {
                    Some(a.range.start)
                } else if !a.quoted {
                    Some(words.start)
                } else {
                    None
                }
            });
            (at, a, n)
        })
        .collect();
    // Numbered as they come in the play; those not beside a line last.
    taken.sort_by_key(|(at, a, _)| (at.is_none(), at.or(a.as_ref().map(|a| a.range.start))));
    let mut edits: Vec<Edit> = Vec::new();
    let mut lit: Vec<Range<usize>> = Vec::new();
    let mut list = String::new();
    for (k, (at, a, n)) in taken.iter().enumerate() {
        let num = k + 1;
        let c = color(&n.author);
        let said = tex_text(&n.text);
        if let Some(at) = *at {
            let mut mark = format!(
                "\\tiyatronote{{{c}}}{{{}}}{{{num}}}{{{said}}}",
                who(&n.author)
            );
            // The words themselves in its colour, when that is safe: on one
            // line, nothing TeX would read inside, not inside another's.
            if let Some(a) = a.as_ref().filter(|a| a.quoted && a.range.start == at) {
                let words = &text[a.range.clone()];
                let plain =
                    !words.contains(['\n', '{', '}', '%', '\\', '#', '$', '&', '^', '_', '~']);
                let free = !lit
                    .iter()
                    .any(|r| r.start < a.range.end && a.range.start < r.end);
                if plain && free {
                    mark.push_str(&format!("\\tiyatronoted{{{c}}}{{"));
                    edits.push(Edit::new(a.range.end..a.range.end, "}"));
                    lit.push(a.range.clone());
                }
            }
            edits.push(Edit::new(at..at, mark));
        }
        let place = match (n.line, a.as_ref()) {
            (_, Some(a)) => a.line,
            (Some(l), None) => l,
            (None, None) => 0,
        };
        let place = if place > 0 {
            crate::lang::f("{}. satır", "line {}", &[&place])
        } else {
            String::new()
        };
        list.push_str(&format!(
            "\\tiyatronoteitem{{{c}}}{{{num}}}{{{}}}{{{place}}}{{{said}}}\n",
            who(&n.author)
        ));
        for r in &n.replies {
            list.push_str(&format!(
                "\\tiyatronotereply{{{}}}{{{}}}{{{}}}\n",
                color(&r.author),
                who(&r.author),
                tex_text(&r.text)
            ));
        }
    }
    if doc.parts.preamble.end > doc.parts.preamble.start {
        edits.push(Edit::new(
            doc.parts.preamble.end..doc.parts.preamble.end,
            "\\tiyatronotes\n",
        ));
    }
    if !list.is_empty() {
        let title = crate::lang::l("Notlar", "Notes");
        edits.push(Edit::new(
            body.end..body.end,
            format!("\n\\tiyatronoteslist{{{title}}}{{\n{list}}}\n"),
        ));
    }
    crate::tex::apply(text, &edits)
}

fn words(s: &str) -> Vec<(usize, &str)> {
    let mut out = Vec::new();
    let mut start = None;
    for (i, c) in s.char_indices() {
        match (c == ' ', start) {
            (true, Some(st)) => {
                out.push((st, &s[st..i]));
                start = None;
            }
            (false, None) => start = Some(i),
            _ => {}
        }
    }
    if let Some(st) = start {
        out.push((st, &s[st..]));
    }
    out
}

fn distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for i in 1..=a.len() {
        let mut cur = vec![i; b.len() + 1];
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
        }
        prev = cur;
    }
    prev[b.len()]
}

#[cfg(test)]
mod tests;
