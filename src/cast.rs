//! The characters a play declares in its preamble, read with their exact
//! places in the text so a change rewrites only what changed.
//!
//! ```latex
//! \karakter*[Full name][Description]{SCRIPT}{alias}   in the cast table
//! \karakter[Description]{SCRIPT}{alias}
//! \karakter{SCRIPT}{alias}                            not in the cast table
//! \gizlikarakter{SCRIPT}{alias}                       not in the cast table
//! \karaktergrubu[Description]{ \gkarakter*[Full name]{SCRIPT}{alias} … }
//! \setinlinename{alias}{Name}
//! ```

use crate::source;
use crate::tex::{self, Edit};
use std::ops::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Kind {
    /// `\karakter` with a description: a row of the cast table.
    #[default]
    Cast,
    /// `\karakter{SCRIPT}{alias}`: speaks, but isn't in the cast table.
    Plain,
    /// `\gizlikarakter`: HERKES, SEYİRCİ… speaks, isn't in the cast table.
    Hidden,
    /// `\gkarakter` inside a `\karaktergrubu`.
    Member,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Character {
    pub alias: String,
    /// The name printed before a line: "SELİN".
    pub script: String,
    /// The name in the cast table when it differs from the script name.
    pub full: Option<String>,
    pub desc: Option<String>,
    /// `\karakter*`: shown on the cover, bold in the cast table.
    pub star: bool,
    pub kind: Kind,
    /// The group it belongs to, for `Kind::Member`.
    pub group: Option<usize>,
    /// The whole declaration.
    pub span: Range<usize>,
    /// The alias inside its braces.
    pub alias_span: Range<usize>,
}

impl Character {
    /// The name in the cast table.
    pub fn display_name(&self) -> &str {
        self.full.as_deref().unwrap_or(&self.script)
    }

    pub fn spec(&self) -> Spec {
        Spec {
            alias: self.alias.clone(),
            script: self.script.clone(),
            full: self.full.clone().unwrap_or_default(),
            desc: self.desc.clone().unwrap_or_default(),
            star: self.star,
            kind: self.kind,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Group {
    pub desc: String,
    pub span: Range<usize>,
    /// Inside the braces holding the members.
    pub body: Range<usize>,
    /// Indexes into `Cast::characters`.
    pub members: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InlineName {
    pub alias: String,
    pub name: String,
    pub span: Range<usize>,
    pub alias_span: Range<usize>,
}

/// A row of the cast table, in the order the play declares them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Row {
    Character(usize),
    Group(usize),
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Cast {
    /// In declaration order.
    pub characters: Vec<Character>,
    pub groups: Vec<Group>,
    pub inline: Vec<InlineName>,
}

impl Cast {
    /// Reads the declarations in `range` (normally the preamble).
    pub fn parse(text: &str, range: Range<usize>) -> Cast {
        let mut cast = Cast::default();
        let commands = tex::commands(text, range.clone());
        let mut skip_until = 0;
        for c in commands {
            if c.span.start < skip_until {
                continue;
            }
            match c.name {
                "karakter" => {
                    if let Some(ch) = read_karakter(text, &c) {
                        cast.characters.push(ch);
                    }
                }
                "gizlikarakter" => {
                    if let Some((script, alias, end)) = two_groups(text, c.span.end) {
                        cast.characters.push(Character {
                            alias: alias.inner.to_string(),
                            script: script.inner.to_string(),
                            full: None,
                            desc: None,
                            star: false,
                            kind: Kind::Hidden,
                            group: None,
                            span: c.span.start..end,
                            alias_span: alias.inner_span,
                        });
                    }
                }
                "karaktergrubu" | "chargroup" => {
                    let desc = tex::optional(text, c.span.end);
                    let after = desc.as_ref().map_or(c.span.end, |d| d.span.end);
                    let Some(body) = tex::group(text, after) else {
                        continue;
                    };
                    let g = cast.groups.len();
                    let mut members = Vec::new();
                    for m in tex::commands(text, body.inner_span.clone()) {
                        if matches!(m.name, "gkarakter" | "gcharacter")
                            && let Some(mut ch) = read_member(text, &m)
                        {
                            ch.group = Some(g);
                            members.push(cast.characters.len());
                            cast.characters.push(ch);
                        }
                    }
                    cast.groups.push(Group {
                        desc: desc.map(|d| d.inner.to_string()).unwrap_or_default(),
                        span: c.span.start..body.span.end,
                        body: body.inner_span.clone(),
                        members,
                    });
                    skip_until = body.span.end;
                }
                "setinlinename" => {
                    if let Some((alias, name, end)) = two_groups(text, c.span.end) {
                        cast.inline.push(InlineName {
                            alias: alias.inner.to_string(),
                            name: name.inner.to_string(),
                            span: c.span.start..end,
                            alias_span: alias.inner_span,
                        });
                    }
                }
                _ => {}
            }
        }
        cast
    }

    /// Reads the preamble of a whole play.
    pub fn of_play(text: &str) -> Cast {
        Cast::parse(text, source::split(text).preamble)
    }

    pub fn get(&self, alias: &str) -> Option<&Character> {
        self.characters.iter().find(|c| c.alias == alias)
    }

    pub fn index(&self, alias: &str) -> Option<usize> {
        self.characters.iter().position(|c| c.alias == alias)
    }

    /// The cast table's rows, in declaration order: characters of kind
    /// `Cast`, and groups.
    pub fn rows(&self) -> Vec<Row> {
        let mut rows: Vec<(usize, Row)> = self
            .characters
            .iter()
            .enumerate()
            .filter(|(_, c)| c.kind == Kind::Cast)
            .map(|(i, c)| (c.span.start, Row::Character(i)))
            .chain(
                self.groups
                    .iter()
                    .enumerate()
                    .map(|(i, g)| (g.span.start, Row::Group(i))),
            )
            .collect();
        rows.sort_by_key(|(at, _)| *at);
        rows.into_iter().map(|(_, r)| r).collect()
    }

    /// The name used for a character inside a line: `\setinlinename`, or the
    /// script name in title case ("SELİN" → "Selin"), as the class prints it.
    pub fn inline_name(&self, alias: &str) -> Option<String> {
        if let Some(i) = self.inline.iter().rev().find(|i| i.alias == alias) {
            return Some(i.name.clone());
        }
        self.get(alias).map(|c| title_case(&c.script))
    }

    /// Whether `alias` can't be used for a new character: taken, clashing with
    /// another character's commands, or a TeX/LaTeX command.
    pub fn alias_taken(&self, alias: &str) -> bool {
        is_reserved(alias)
            || self
                .characters
                .iter()
                .any(|c| aliases_clash(alias, &c.alias))
    }

    /// An alias for a new character: the first letters of its script name,
    /// growing until free. MARİA → "ma", then "mar", "mari"…
    pub fn suggest_alias(&self, script: &str) -> String {
        let letters: String = fold_ascii(script);
        let letters = if letters.is_empty() {
            "kisi".to_string()
        } else {
            letters
        };
        let chars: Vec<char> = letters.chars().collect();
        for n in 2..=chars.len().max(2) {
            let candidate: String = chars.iter().take(n).collect();
            if candidate.chars().count() >= 2 && !self.alias_taken(&candidate) {
                return candidate;
            }
        }
        // Every prefix is taken: add letters to the first two.
        let base: String = chars.iter().take(2).collect();
        for extra in 'a'..='z' {
            let candidate = format!("{base}{extra}");
            if !self.alias_taken(&candidate) {
                return candidate;
            }
        }
        for a in 'a'..='z' {
            for b in 'a'..='z' {
                let candidate = format!("{base}{a}{b}");
                if !self.alias_taken(&candidate) {
                    return candidate;
                }
            }
        }
        format!("{base}kisi")
    }
}

fn two_groups<'a>(text: &'a str, at: usize) -> Option<(tex::Arg<'a>, tex::Arg<'a>, usize)> {
    let a = tex::group(text, at)?;
    let b = tex::group(text, a.span.end)?;
    let end = b.span.end;
    Some((a, b, end))
}

fn some_if_nonblank(s: &str) -> Option<String> {
    (!s.trim().is_empty()).then(|| s.to_string())
}

fn read_karakter(text: &str, c: &tex::Command) -> Option<Character> {
    let star = tex::star(text, c.span.end);
    let mut at = star.as_ref().map_or(c.span.end, |s| s.end);
    let o1 = tex::optional(text, at);
    if let Some(o) = &o1 {
        at = o.span.end;
    }
    let o2 = o1.as_ref().and_then(|_| tex::optional(text, at));
    if let Some(o) = &o2 {
        at = o.span.end;
    }
    let (script, alias, end) = two_groups(text, at)?;
    let (kind, full, desc) = match (&o1, &o2) {
        (None, _) => (Kind::Plain, None, None),
        (Some(d), None) => (Kind::Cast, None, Some(d.inner.to_string())),
        (Some(f), Some(d)) => (
            Kind::Cast,
            some_if_nonblank(f.inner),
            Some(d.inner.to_string()),
        ),
    };
    Some(Character {
        alias: alias.inner.to_string(),
        script: script.inner.to_string(),
        full,
        desc,
        star: star.is_some(),
        kind,
        group: None,
        span: c.span.start..end,
        alias_span: alias.inner_span,
    })
}

fn read_member(text: &str, c: &tex::Command) -> Option<Character> {
    let star = tex::star(text, c.span.end);
    let mut at = star.as_ref().map_or(c.span.end, |s| s.end);
    let full = tex::optional(text, at);
    if let Some(f) = &full {
        at = f.span.end;
    }
    let (script, alias, end) = two_groups(text, at)?;
    Some(Character {
        alias: alias.inner.to_string(),
        script: script.inner.to_string(),
        full: full.and_then(|f| some_if_nonblank(f.inner)),
        desc: None,
        star: star.is_some(),
        kind: Kind::Member,
        group: None,
        span: c.span.start..end,
        alias_span: alias.inner_span,
    })
}

// ------------------------------------------------------------------ writing

/// What a declaration says, independent of where it is.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Spec {
    pub alias: String,
    pub script: String,
    /// Empty: the cast table shows the script name.
    pub full: String,
    pub desc: String,
    pub star: bool,
    pub kind: Kind,
}

/// The declaration's source, in its shortest form.
pub fn declaration(s: &Spec) -> String {
    let star = if s.star { "*" } else { "" };
    let tail = format!("{{{}}}{{{}}}", s.script, s.alias);
    match s.kind {
        Kind::Cast if s.full.trim().is_empty() => format!("\\karakter{star}[{}]{tail}", s.desc),
        Kind::Cast => format!("\\karakter{star}[{}][{}]{tail}", s.full, s.desc),
        Kind::Plain => format!("\\karakter{tail}"),
        Kind::Hidden => format!("\\gizlikarakter{tail}"),
        Kind::Member if s.full.trim().is_empty() => format!("\\gkarakter{star}{tail}"),
        Kind::Member => format!("\\gkarakter{star}[{}]{tail}", s.full),
    }
}

/// The end of the line holding `at` (before its line ending).
fn line_end(text: &str, at: usize) -> usize {
    let end = text[at..].find('\n').map_or(text.len(), |i| at + i);
    if end > at && text.as_bytes()[end - 1] == b'\r' {
        end - 1
    } else {
        end
    }
}

fn line_start(text: &str, at: usize) -> usize {
    text[..at].rfind('\n').map_or(0, |i| i + 1)
}

/// Adds a character after the last declaration (or before
/// `\begin{document}` when there is none). A member goes at the end of its
/// group, `group` being an index into `Cast::groups`.
pub fn add(text: &str, cast: &Cast, spec: &Spec, group: Option<usize>) -> Edit {
    let nl = source::line_ending(text);
    if let (Kind::Member, Some(g)) = (spec.kind, group)
        && let Some(group) = cast.groups.get(g)
    {
        let indent = group
            .members
            .first()
            .map(|&m| {
                let at = cast.characters[m].span.start;
                text[line_start(text, at)..at].to_string()
            })
            .unwrap_or_else(|| "\t".to_string());
        let after = group.members.last().map_or(group.body.start, |&m| {
            line_end(text, cast.characters[m].span.end)
        });
        return Edit::new(after..after, format!("{nl}{indent}{}", declaration(spec)));
    }
    let last = cast
        .characters
        .iter()
        .filter(|c| c.group.is_none())
        .map(|c| c.span.end)
        .chain(cast.groups.iter().map(|g| g.span.end))
        .max();
    match last {
        Some(end) => {
            let at = line_end(text, end);
            Edit::new(at..at, format!("{nl}{}", declaration(spec)))
        }
        None => {
            let at = source::split(text).preamble.end;
            let at = line_start(text, at);
            Edit::new(at..at, format!("{}{nl}{nl}", declaration(spec)))
        }
    }
}

/// Rewrites a character's declaration, or nothing when it says the same.
pub fn update(cast: &Cast, alias: &str, spec: &Spec) -> Option<Edit> {
    let c = cast.get(alias)?;
    (c.spec() != *spec).then(|| Edit::new(c.span.clone(), declaration(spec)))
}

/// Removes a declaration, with its line when nothing else is on it.
pub fn remove(text: &str, cast: &Cast, alias: &str) -> Option<Edit> {
    let c = cast.get(alias)?;
    Some(Edit::new(whole_line_or(text, c.span.clone()), ""))
}

fn whole_line_or(text: &str, span: Range<usize>) -> Range<usize> {
    let ls = line_start(text, span.start);
    let le = line_end(text, span.end);
    let alone = text[ls..span.start].trim().is_empty() && text[span.end..le].trim().is_empty();
    if !alone {
        return span;
    }
    // Take the line ending too.
    let after = if text[le..].starts_with("\r\n") {
        le + 2
    } else if text[le..].starts_with('\n') {
        le + 1
    } else {
        le
    };
    ls..after
}

/// Renames an alias everywhere: the declaration, `\setinlinename`, and every
/// `\old`, `\olddo` and `\oldfont` in the text. Comments are left alone.
pub fn rename(text: &str, cast: &Cast, old: &str, new: &str) -> Vec<Edit> {
    let mut edits = Vec::new();
    if let Some(c) = cast.get(old) {
        edits.push(Edit::new(c.alias_span.clone(), new));
    }
    for i in cast.inline.iter().filter(|i| i.alias == old) {
        edits.push(Edit::new(i.alias_span.clone(), new));
    }
    for cmd in tex::commands(text, 0..text.len()) {
        let suffix = if cmd.name == old {
            ""
        } else if cmd.name.strip_prefix(old) == Some("do") {
            "do"
        } else if cmd.name.strip_prefix(old) == Some("font") {
            "font"
        } else {
            continue;
        };
        let name = cmd.span.start + 1..cmd.span.end;
        edits.push(Edit::new(name, format!("{new}{suffix}")));
    }
    edits
}

/// How many times an alias is used: lines spoken (`\alias` starting a line
/// or inline) and directions (`\aliasdo`).
pub fn uses(text: &str, range: Range<usize>, alias: &str) -> (usize, usize) {
    let do_name = format!("{alias}do");
    tex::commands(text, range).iter().fold((0, 0), |(a, d), c| {
        if c.name == alias {
            (a + 1, d)
        } else if c.name == do_name {
            (a, d + 1)
        } else {
            (a, d)
        }
    })
}

// ------------------------------------------------------------------ names

/// Turkish lower case: I → ı, İ → i.
pub fn lower_tr(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'I' => "ı".to_string(),
            'İ' => "i".to_string(),
            c => c.to_lowercase().collect(),
        })
        .collect()
}

/// Turkish upper case: i → İ, ı → I.
pub fn upper_tr(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'i' => "İ".to_string(),
            'ı' => "I".to_string(),
            c => c.to_uppercase().collect(),
        })
        .collect()
}

/// "SELİN" → "Selin", "IŞIK" → "Işık": the first letter upper case, the rest
/// lower, in Turkish.
pub fn title_case(s: &str) -> String {
    let lower = lower_tr(s);
    let mut chars = lower.chars();
    match chars.next() {
        Some(first) => upper_tr(&first.to_string()) + chars.as_str(),
        None => String::new(),
    }
}

/// ASCII letters for an alias: "ŞÜKRÜ BEY" → "sukrubey".
pub fn fold_ascii(s: &str) -> String {
    lower_tr(s)
        .chars()
        .filter_map(|c| {
            let c = match c {
                'ı' => 'i',
                'ş' => 's',
                'ğ' => 'g',
                'ü' => 'u',
                'ö' => 'o',
                'ç' => 'c',
                'â' | 'à' | 'á' | 'ä' => 'a',
                'î' | 'í' => 'i',
                'û' | 'ú' => 'u',
                'é' | 'è' | 'ê' => 'e',
                c => c,
            };
            c.is_ascii_lowercase().then_some(c)
        })
        .collect()
}

/// Whether two aliases' commands collide: `\a`, `\ado`, `\afont` against
/// `\b`, `\bdo`, `\bfont`.
fn aliases_clash(a: &str, b: &str) -> bool {
    let forms = |x: &str| [x.to_string(), format!("{x}do"), format!("{x}font")];
    let fa = forms(a);
    forms(b).iter().any(|f| fa.contains(f))
}

/// Commands an alias (or its `do`/`font` form) must not take over.
fn is_reserved(alias: &str) -> bool {
    if alias.is_empty() || !alias.chars().all(tex::is_letter) {
        return true;
    }
    [
        alias.to_string(),
        format!("{alias}do"),
        format!("{alias}font"),
    ]
    .iter()
    .any(|f| RESERVED.contains(&f.as_str()))
}

/// TeX, LaTeX and tiyatro commands that a short alias could hit.
#[rustfmt::skip]
const RESERVED: &[&str] = &[
    // The class
    "karakter", "gizlikarakter", "karaktergrubu", "gkarakter", "gcharacter", "chargroup",
    "setinlinename", "setmaxnamewidth", "action", "stagedir", "middir", "ic", "beat", "sahne",
    "scene", "perde", "act", "kisiler", "draft", "copyrightnotice", "address", "telephone",
    "email", "ru", "en", "de", "fr", "el", "yenidil", "fontyukle", "theact", "thescene",
    "namefont", "starfont", "actionfont", "castfont",
    // TeX and LaTeX
    "do", "it", "bf", "rm", "sf", "tt", "sc", "sl", "em", "up", "md", "aa", "ae", "oe", "ss",
    "AA", "AE", "OE", "i", "j", "l", "o", "b", "c", "d", "h", "k", "r", "t", "u", "v", "H", "L",
    "O", "S", "P", "ge", "le", "ne", "ni", "in", "to", "gets", "of", "or", "if", "fi", "par",
    "day", "year", "month", "the", "let", "def", "end", "box", "put", "item", "label", "ref",
    "cite", "line", "vec", "bar", "hat", "dot", "tilde", "sin", "cos", "tan", "log", "ln",
    "exp", "min", "max", "lim", "deg", "det", "dim", "ker", "hom", "arg", "gcd", "inf", "sup",
    "pr", "sec", "csc", "cot", "sum", "int", "prod", "cup", "cap", "mid", "mp", "pm", "wp",
    "Re", "Im", "ell", "eta", "mu", "nu", "pi", "xi", "chi", "phi", "psi", "rho", "tau", "beta",
    "zeta", "iota", "alpha", "gamma", "delta", "kappa", "theta", "sigma", "omega", "lambda",
    "quad", "qquad", "hfil", "hfill", "vfil", "vfill", "hss", "vss", "kern", "skip", "break",
    "relax", "space", "empty", "null", "hline", "vline", "newline", "large", "Large", "huge",
    "Huge", "small", "tiny", "title", "author", "date", "textbf", "textit", "emph", "underline",
    "url", "cr", "span", "omit", "hbox", "vbox", "vtop", "copy", "lower", "raise", "show", "wd",
    "ht", "dp", "count", "dimen", "toks", "char", "accent", "atop", "over", "above", "left",
    "right", "middle", "else", "ifx", "ifnum", "ifdim", "loop", "repeat", "value", "index",
    "mark", "input", "string", "number", "meaning", "edef", "gdef", "xdef", "global", "long",
    "outer", "dag", "ddag", "copyright", "pounds", "LaTeX", "TeX", "today", "and", "thanks",
    "part", "chapter", "section", "paragraph", "footnote", "centering", "noindent", "indent",
    "vspace", "hspace", "smallskip", "medskip", "bigskip", "clearpage", "newpage", "pagebreak",
    "linebreak", "nobreak", "protect", "fbox", "mbox", "parbox", "rule", "strut", "phantom",
    "lq", "rq", "ldots", "dots", "cdot", "times", "div", "neq", "leq", "geq", "sim", "approx",
    "equiv", "cong", "perp", "angle", "nabla", "partial", "infty", "forall", "exists", "neg",
    "lnot", "land", "lor", "top", "bot", "emptyset", "aleph", "hbar", "imath", "jmath", "Pr",
    "arccos", "arcsin", "arctan", "cosh", "coth", "sinh", "tanh", "liminf", "limsup", "tag",
    "text", "frac", "sqrt", "binom", "choose",
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tex::apply;

    const PLAY: &str = include_str!("../examples/ornek-oyun.tex");

    #[test]
    fn example_play() {
        let cast = Cast::of_play(PLAY);
        let aliases: Vec<&str> = cast.characters.iter().map(|c| c.alias.as_str()).collect();
        assert_eq!(aliases, vec!["se", "oz", "ih", "ka", "co", "hep"]);

        let se = cast.get("se").unwrap();
        assert_eq!(se.script, "SELİN");
        assert_eq!(se.full.as_deref(), Some("Selin"));
        assert_eq!(se.desc.as_deref(), Some("Köy öğretmeni"));
        assert!(se.star);
        assert_eq!(
            &PLAY[se.span.clone()],
            "\\karakter*[Selin][Köy öğretmeni]{SELİN}{se}"
        );

        let ih = cast.get("ih").unwrap();
        assert_eq!(
            (ih.full.as_deref(), ih.desc.as_deref()),
            (None, Some("Yaşlı bir yolcu"))
        );
        assert_eq!(cast.get("ka").unwrap().kind, Kind::Member);
        assert_eq!(
            cast.get("ka").unwrap().full.as_deref(),
            Some("Birinci Kadın")
        );
        assert_eq!(cast.get("hep").unwrap().kind, Kind::Hidden);

        assert_eq!(cast.groups.len(), 1);
        assert_eq!(cast.groups[0].desc, "Köylüler");
        assert_eq!(cast.groups[0].members, vec![3, 4]);
        assert_eq!(
            cast.rows(),
            vec![
                Row::Character(0),
                Row::Character(1),
                Row::Character(2),
                Row::Group(0)
            ]
        );

        assert_eq!(cast.inline_name("oz").as_deref(), Some("Ozan Usta"));
        assert_eq!(cast.inline_name("se").as_deref(), Some("Selin"));
        assert_eq!(cast.inline_name("ih").as_deref(), Some("İhtiyar"));
    }

    #[test]
    fn every_form() {
        let t = "\\karakter{A}{a}\n\
                 \\karakter [Tanım] {B}{b}\n\
                 \\karakter*[Tam][]{C}{c} % \\karakter{X}{x}\n\
                 \\chargroup{\\gcharacter*{D}{d}}\n\
                 \\karakter[[x]]{E}{e}\n\
                 \\karakter[Kır]\n  {F}{f}\n";
        let cast = Cast::parse(t, 0..t.len());
        let got: Vec<(&str, Kind, bool)> = cast
            .characters
            .iter()
            .map(|c| (c.alias.as_str(), c.kind, c.star))
            .collect();
        assert_eq!(
            got,
            vec![
                ("a", Kind::Plain, false),
                ("b", Kind::Cast, false),
                ("c", Kind::Cast, true),
                ("d", Kind::Member, true),
                ("f", Kind::Cast, false),
            ]
        );
        assert_eq!(cast.get("c").unwrap().desc.as_deref(), Some(""));
    }

    #[test]
    fn declarations_round_trip() {
        // The shortest form of each parsed declaration is the declaration.
        let cast = Cast::of_play(PLAY);
        for c in &cast.characters {
            assert_eq!(declaration(&c.spec()), PLAY[c.span.clone()], "{}", c.alias);
            assert_eq!(update(&cast, &c.alias, &c.spec()), None);
        }
    }

    #[test]
    fn add_update_remove() {
        let cast = Cast::of_play(PLAY);
        let spec = Spec {
            alias: "yo".into(),
            script: "YOLCU".into(),
            desc: "Bir başkası".into(),
            ..Spec::default()
        };
        let out = apply(PLAY, &[add(PLAY, &cast, &spec, None)]);
        assert!(out.contains("\\gizlikarakter{HERKES}{hep}\n\\karakter[Bir başkası]{YOLCU}{yo}\n"));
        assert_eq!(Cast::of_play(&out).characters.len(), 7);

        let member = Spec {
            alias: "ba".into(),
            script: "BALIKÇI".into(),
            kind: Kind::Member,
            ..Spec::default()
        };
        let out = apply(PLAY, &[add(PLAY, &cast, &member, Some(0))]);
        assert!(out.contains("\t\\gkarakter{ÇOBAN}{co}\n\t\\gkarakter{BALIKÇI}{ba}\n}"));
        assert_eq!(Cast::of_play(&out).groups[0].members.len(), 3);

        let mut oz = cast.get("oz").unwrap().spec();
        oz.star = true;
        let out = apply(PLAY, &[update(&cast, "oz", &oz).unwrap()]);
        assert!(out.contains("\\karakter*[Ozan Usta][Değirmenci, Selin'in amcası]{OZAN}{oz}\n"));

        let out = apply(PLAY, &[remove(PLAY, &cast, "ih").unwrap()]);
        assert!(!out.contains("İHTİYAR}{ih}"));
        assert!(out.contains("{OZAN}{oz}\n\\karaktergrubu"));
    }

    #[test]
    fn first_character() {
        let t = "\\documentclass{tiyatro}\n\\title{X}\n\n\\begin{document}\n\\end{document}\n";
        let cast = Cast::of_play(t);
        let spec = Spec {
            alias: "se".into(),
            script: "SELİN".into(),
            desc: "Öğretmen".into(),
            ..Spec::default()
        };
        let out = apply(t, &[add(t, &cast, &spec, None)]);
        assert_eq!(
            out,
            "\\documentclass{tiyatro}\n\\title{X}\n\n\\karakter[Öğretmen]{SELİN}{se}\n\n\\begin{document}\n\\end{document}\n"
        );
    }

    #[test]
    fn crlf_is_kept() {
        let t = "\\karakter[a]{A}{a}\r\n\\begin{document}\r\n\\end{document}\r\n";
        let cast = Cast::of_play(t);
        let spec = Spec {
            alias: "b".into(),
            script: "B".into(),
            ..Spec::default()
        };
        let out = apply(t, &[add(t, &cast, &spec, None)]);
        assert_eq!(
            out,
            "\\karakter[a]{A}{a}\r\n\\karakter[]{B}{b}\r\n\\begin{document}\r\n\\end{document}\r\n"
        );
        let out = apply(t, &[remove(t, &cast, "a").unwrap()]);
        assert_eq!(out, "\\begin{document}\r\n\\end{document}\r\n");
    }

    #[test]
    fn rename_everywhere() {
        let cast = Cast::of_play(PLAY);
        let out = apply(PLAY, &rename(PLAY, &cast, "oz", "ozan"));
        assert!(out.contains("{OZAN}{ozan}"));
        assert!(out.contains("\\setinlinename{ozan}{Ozan Usta}"));
        assert!(out.contains("\\ozan Durdu kızım."));
        assert!(out.contains("\\sedo\\ozando Birbirlerine"));
        assert!(out.contains("Ama \\ozan bu kış"));
        assert!(!out.contains("\\oz "));
        // Other commands starting with the alias stay.
        let t = "\\ozgur \\oz \\ozdo \\ozfont \\ozdoX % \\oz";
        let cast = Cast::default();
        let out = apply(t, &rename(t, &cast, "oz", "o"));
        assert_eq!(out, "\\ozgur \\o \\odo \\ofont \\ozdoX % \\oz");
    }

    #[test]
    fn counting_uses() {
        let p = source::split(PLAY);
        assert_eq!(uses(PLAY, p.body.clone(), "se"), (4, 1));
        assert_eq!(uses(PLAY, p.body, "ih"), (2, 2));
    }

    #[test]
    fn aliases() {
        let cast = Cast::of_play(PLAY);
        assert_eq!(cast.suggest_alias("MARİA"), "ma");
        // "se" is taken; "sel" is free.
        assert_eq!(cast.suggest_alias("SEVGİ"), "sev");
        // "mu" is a TeX command; "mus" is free.
        assert_eq!(cast.suggest_alias("MUSTAFA"), "mus");
        assert_eq!(cast.suggest_alias("ŞÜKRÜ"), "su");
        assert!(cast.alias_taken("se"));
        assert!(cast.alias_taken("sedo"));
        assert!(cast.alias_taken("do"));
        assert!(cast.alias_taken("i"));
        assert!(!cast.alias_taken("yv"));
    }

    #[test]
    fn turkish_case() {
        assert_eq!(title_case("İHTİYAR"), "İhtiyar");
        assert_eq!(title_case("IŞIK"), "Işık");
        assert_eq!(title_case("ÇOBAN"), "Çoban");
        assert_eq!(fold_ascii("ŞÜKRÜ BEY"), "sukrubey");
    }
}
