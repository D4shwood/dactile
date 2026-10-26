//! The play's body as the tiyatro class reads it: blocks of whole source
//! lines (a line of dialogue, a direction, an act, a scene…) and, inside a
//! line, the inline pieces (a reference to a character, `\ic{…}`, `_word_`).
//!
//! The class's rules, which this follows:
//! - a line of dialogue or a `\xdo` direction runs to the end of the source
//!   line; a `%` at its end joins the next line to it, as TeX does
//! - the first macro on a line is the speaker; a later alias is a name
//! - an argument in braces may run over several lines

use crate::cast::Cast;
use crate::tex;
use std::ops::Range;

#[derive(Debug, Clone, PartialEq)]
pub enum Kind {
    /// `\alias Text`: `text` is what follows the alias.
    Line {
        alias: String,
        text: Range<usize>,
    },
    /// `\adod\bdo Text`.
    Do {
        aliases: Vec<String>,
        text: Range<usize>,
    },
    /// `\action{…}`.
    Action {
        text: Range<usize>,
    },
    /// `\stagedir{…}` (centred) or `\middir{…}` (right).
    Stage {
        text: Range<usize>,
        right: bool,
    },
    /// `\perde[Title]`.
    Act {
        title: String,
    },
    /// `\sahne[Place][Number]`.
    Scene {
        place: String,
        label: Option<String>,
    },
    /// `\beat` on its own line.
    Beat,
    /// A line that starts with a macro nobody declared. It may be a speaker
    /// whose `\karakter` line is missing.
    Unknown {
        name: String,
    },
    /// `% …` lines.
    Comment,
    Blank,
    /// Anything else: other LaTeX, plain paragraphs.
    Other,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub kind: Kind,
    /// Whole lines, without the last line's ending.
    pub span: Range<usize>,
    /// Index of its first line in the text.
    pub line: usize,
}

impl Block {
    /// Whether the class gives this block a draft line number.
    pub fn numbered(&self) -> bool {
        matches!(
            self.kind,
            Kind::Line { .. } | Kind::Do { .. } | Kind::Action { .. } | Kind::Stage { .. }
        )
    }
}

/// Splits `range` of `text` into blocks.
pub fn blocks(text: &str, range: Range<usize>, cast: &Cast) -> Vec<Block> {
    let lines: Vec<Range<usize>> = tex::lines(&text[range.clone()])
        .into_iter()
        .map(|r| r.start + range.start..r.end + range.start)
        .collect();
    let first_line = text[..range.start].matches('\n').count();
    let mut out = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let start = lines[i].start;
        // A block runs on while a brace is open; a line of dialogue or a
        // direction also runs on when a comment swallows its line end.
        let speaks = matches!(
            classify(text, lines[i].clone(), cast),
            Kind::Line { .. } | Kind::Do { .. }
        );
        let mut j = i;
        loop {
            let span = start..lines[j].end;
            let open = depth(&text[span]) > 0;
            let joined = speaks && ends_in_comment(&text[lines[j].clone()]);
            if (open || joined) && j + 1 < lines.len() {
                j += 1;
            } else {
                break;
            }
        }
        let span = start..lines[j].end;
        out.push(Block {
            kind: classify(text, span.clone(), cast),
            span,
            line: first_line + i,
        });
        i = j + 1;
    }
    out
}

fn ends_in_comment(line: &str) -> bool {
    tex::comment_start(line).is_some()
}

/// Open braces left at the end of `s`, ignoring escaped braces and comments.
fn depth(s: &str) -> i64 {
    let mut d = 0i64;
    for line in s.split('\n') {
        let code = tex::comment_start(line).map_or(line, |c| &line[..c]);
        for (i, c) in code.char_indices() {
            match c {
                '{' if !tex::is_escaped(code, i) => d += 1,
                '}' if !tex::is_escaped(code, i) => d -= 1,
                _ => {}
            }
        }
    }
    d
}

fn classify(text: &str, span: Range<usize>, cast: &Cast) -> Kind {
    let s = &text[span.clone()];
    let trimmed = s.trim_start();
    if trimmed.trim().is_empty() {
        return Kind::Blank;
    }
    if trimmed.starts_with('%') {
        return Kind::Comment;
    }
    let lead = span.start + (s.len() - trimmed.len());
    let cmds = tex::commands(text, lead..span.end);
    let Some(first) = cmds.first().filter(|c| c.span.start == lead) else {
        return Kind::Other;
    };
    let name = first.name;
    let after = first.span.end;
    // Where the text of a line or direction starts: after the spaces TeX
    // skips, and after an optional opening brace.
    let text_from = |at: usize| {
        let at = skip_inline_space(text, at, span.end);
        at..span.end
    };
    if cast.get(name).is_some() {
        return Kind::Line {
            alias: name.to_string(),
            text: text_from(after),
        };
    }
    if let Some(alias) = name.strip_suffix("do").filter(|a| cast.get(a).is_some()) {
        // Collect the chain: \adod\bdo… with only spaces between.
        let mut aliases = vec![alias.to_string()];
        let mut end = after;
        for c in &cmds[1..] {
            if !text[end..c.span.start].trim().is_empty() {
                break;
            }
            match c.name.strip_suffix("do").filter(|a| cast.get(a).is_some()) {
                Some(a) => {
                    aliases.push(a.to_string());
                    end = c.span.end;
                }
                None => break,
            }
        }
        return Kind::Do {
            aliases,
            text: text_from(end),
        };
    }
    match name {
        "action" | "stagedir" | "middir" => {
            let Some(arg) = tex::group(text, after) else {
                return Kind::Other;
            };
            let inner = arg.inner_span;
            match name {
                "action" => Kind::Action { text: inner },
                n => Kind::Stage {
                    text: inner,
                    right: n == "middir",
                },
            }
        }
        "perde" | "act" => Kind::Act {
            title: tex::optional(text, after)
                .map(|o| o.inner.to_string())
                .unwrap_or_default(),
        },
        "sahne" | "scene" => {
            let place = tex::optional(text, after);
            let label = place.as_ref().and_then(|p| tex::optional(text, p.span.end));
            Kind::Scene {
                place: place.map(|p| p.inner.to_string()).unwrap_or_default(),
                label: label
                    .map(|l| l.inner.to_string())
                    .filter(|l| !l.trim().is_empty()),
            }
        }
        "beat" if text[after..span.end].trim().is_empty() => Kind::Beat,
        n if is_known_command(n) => Kind::Other,
        n => Kind::Unknown {
            name: n.to_string(),
        },
    }
}

fn skip_inline_space(text: &str, mut at: usize, end: usize) -> usize {
    while at < end && matches!(text.as_bytes()[at], b' ' | b'\t') {
        at += 1;
    }
    at
}

/// LaTeX commands that start a line without being a speaker.
fn is_known_command(name: &str) -> bool {
    const KNOWN: &[&str] = &[
        "maketitle",
        "kisiler",
        "clearpage",
        "newpage",
        "pagebreak",
        "section",
        "subsection",
        "chapter",
        "par",
        "noindent",
        "vspace",
        "vskip",
        "bigskip",
        "medskip",
        "smallskip",
        "centering",
        "begin",
        "end",
        "item",
        "tableofcontents",
        "textit",
        "textbf",
        "emph",
        "ic",
        "beat",
        "input",
        "include",
        "setinlinename",
        "karakter",
        "gizlikarakter",
        "karaktergrubu",
        "label",
        "footnote",
        "hfill",
        "vfill",
        "thispagestyle",
        "pagestyle",
        "ru",
        "en",
        "de",
        "fr",
        "el",
        "yenidil",
        "fontyukle",
        "setmaxnamewidth",
        "linebreak",
        "newline",
        "small",
        "large",
        "Large",
        "normalsize",
        "itshape",
        "bfseries",
        "scshape",
    ];
    KNOWN.contains(&name)
}

// ------------------------------------------------------------------ inside a line

#[derive(Debug, Clone, PartialEq)]
pub enum Inline {
    /// Plain text.
    Text,
    /// `\alias` inside a line: the character's name.
    Ref(String),
    /// `\ic{…}`: `inner` is the direction's text.
    Ic { inner: Range<usize> },
    /// `_word_`, underlined.
    Under { inner: Range<usize> },
    /// `\ru{…}`, `\en{…}` and other language or style wrappers.
    Wrap { name: String, inner: Range<usize> },
    /// `\%`, `\&`, `\\`… and `~`, `--`, `---`.
    Special,
    /// A macro the app doesn't know: shown as it is.
    Raw,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Piece {
    pub kind: Inline,
    pub span: Range<usize>,
}

/// The pieces of a line's text, covering `range` exactly.
pub fn inline(text: &str, range: Range<usize>, cast: &Cast, in_dialogue: bool) -> Vec<Piece> {
    let mut out: Vec<Piece> = Vec::new();
    let mut plain_from = range.start;
    let mut i = range.start;
    let push = |out: &mut Vec<Piece>, plain_from: &mut usize, piece: Piece| {
        if *plain_from < piece.span.start {
            out.push(Piece {
                kind: Inline::Text,
                span: *plain_from..piece.span.start,
            });
        }
        *plain_from = piece.span.end;
        out.push(piece);
    };
    while i < range.end {
        let c = text[i..].chars().next().unwrap();
        match c {
            '\\' => {
                let rest = &text[i + 1..range.end];
                let name_len: usize = rest
                    .chars()
                    .take_while(|&c| tex::is_letter(c))
                    .map(char::len_utf8)
                    .sum();
                if name_len == 0 {
                    let sym = rest.chars().next().map_or(0, char::len_utf8);
                    let mut end = i + 1 + sym;
                    // `\\[1ex]`: the spacing goes with the line break.
                    if rest.starts_with('\\')
                        && let Some(o) = tex::optional(text, end)
                            .filter(|o| o.span.start == end && o.span.end <= range.end)
                    {
                        end = o.span.end;
                    }
                    push(
                        &mut out,
                        &mut plain_from,
                        Piece {
                            kind: Inline::Special,
                            span: i..end,
                        },
                    );
                    i = end;
                    continue;
                }
                let name = &text[i + 1..i + 1 + name_len];
                let end = i + 1 + name_len;
                let piece = if cast.get(name).is_some() {
                    // `\\ka{}ya`: the empty group only ends the name.
                    let end = if text[end..range.end].starts_with("{}") {
                        end + 2
                    } else {
                        end
                    };
                    Piece {
                        kind: Inline::Ref(name.to_string()),
                        span: i..end,
                    }
                } else if let Some(arg) = (name == "ic"
                    || matches!(
                        name,
                        "ru" | "en" | "de" | "fr" | "el" | "textit" | "emph" | "textbf"
                    ))
                .then(|| tex::group(text, end))
                .flatten()
                .filter(|a| a.span.end <= range.end)
                {
                    let kind = if name == "ic" {
                        Inline::Ic {
                            inner: arg.inner_span.clone(),
                        }
                    } else {
                        Inline::Wrap {
                            name: name.to_string(),
                            inner: arg.inner_span.clone(),
                        }
                    };
                    Piece {
                        kind,
                        span: i..arg.span.end,
                    }
                } else {
                    Piece {
                        kind: Inline::Raw,
                        span: i..end,
                    }
                };
                i = piece.span.end;
                push(&mut out, &mut plain_from, piece);
            }
            '_' if in_dialogue => {
                if let Some(close) = text[i + 1..range.end].find('_') {
                    let end = i + 1 + close + 1;
                    push(
                        &mut out,
                        &mut plain_from,
                        Piece {
                            kind: Inline::Under {
                                inner: i + 1..end - 1,
                            },
                            span: i..end,
                        },
                    );
                    i = end;
                } else {
                    i += 1;
                }
            }
            '~' => {
                push(
                    &mut out,
                    &mut plain_from,
                    Piece {
                        kind: Inline::Special,
                        span: i..i + 1,
                    },
                );
                i += 1;
            }
            '-' if text[i..range.end].starts_with("--") => {
                let n = if text[i..range.end].starts_with("---") {
                    3
                } else {
                    2
                };
                push(
                    &mut out,
                    &mut plain_from,
                    Piece {
                        kind: Inline::Special,
                        span: i..i + n,
                    },
                );
                i += n;
            }
            '%' => {
                // The rest of the line is a comment.
                let end = text[i..range.end].find('\n').map_or(range.end, |n| i + n);
                push(
                    &mut out,
                    &mut plain_from,
                    Piece {
                        kind: Inline::Raw,
                        span: i..end,
                    },
                );
                i = end;
            }
            c => i += c.len_utf8(),
        }
    }
    if plain_from < range.end {
        out.push(Piece {
            kind: Inline::Text,
            span: plain_from..range.end,
        });
    }
    out
}

/// The words of a stretch of text as an audience hears it: macros, `\ic{…}`
/// directions and punctuation left out.
pub fn spoken_words(text: &str, range: Range<usize>, cast: &Cast) -> usize {
    inline(text, range.clone(), cast, true)
        .iter()
        .map(|p| match &p.kind {
            Inline::Text => count_words(&text[p.span.clone()]),
            Inline::Under { inner } | Inline::Wrap { inner, .. } => {
                count_words(&text[inner.clone()])
            }
            // A name said aloud is a word.
            Inline::Ref(_) => 1,
            _ => 0,
        })
        .sum()
}

/// Words in plain text: runs with at least one letter or digit.
pub fn count_words(s: &str) -> usize {
    s.split(|c: char| c.is_whitespace())
        .filter(|w| w.chars().any(char::is_alphanumeric))
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source;

    const PLAY: &str = include_str!("../examples/ornek-oyun.tex");

    fn body_blocks(t: &str) -> (Cast, Vec<Block>) {
        let cast = Cast::of_play(t);
        let b = blocks(t, source::split(t).body, &cast);
        (cast, b)
    }

    #[test]
    fn example_blocks() {
        let (_, b) = body_blocks(PLAY);
        let kinds: Vec<String> = b
            .iter()
            .filter(|b| !matches!(b.kind, Kind::Blank))
            .map(|b| match &b.kind {
                Kind::Line { alias, .. } => alias.clone(),
                Kind::Do { aliases, .. } => format!("{}do", aliases.join("+")),
                Kind::Action { .. } => "action".into(),
                Kind::Stage { right, .. } => if *right { "middir" } else { "stagedir" }.into(),
                Kind::Act { title } => format!("perde:{title}"),
                Kind::Scene { place, label } => {
                    format!("sahne:{place}:{}", label.clone().unwrap_or_default())
                }
                Kind::Beat => "beat".into(),
                Kind::Unknown { name } => format!("?{name}"),
                Kind::Comment => "%".into(),
                Kind::Blank => unreachable!(),
                Kind::Other => "other".into(),
            })
            .collect();
        assert_eq!(
            kinds,
            vec![
                "other",
                "other",
                "perde:Kar Yağarken",
                "sahne:İç, Değirmen — Akşam:",
                "action",
                "stagedir",
                "se",
                "oz",
                "se",
                "oz",
                "ihdo",
                "se",
                "ih",
                "se+ozdo",
                "middir",
                "sahne:Dış, Meydan — Gece:",
                "ka",
                "co",
                "hep",
                "se",
                "beat",
                "perde:",
                "sahne::2.1",
                "oz",
                "ih",
                "ihdo",
            ]
        );
    }

    #[test]
    fn line_text_and_numbers() {
        let (_, b) = body_blocks(PLAY);
        let first = b
            .iter()
            .find(|b| matches!(b.kind, Kind::Line { .. }))
            .unwrap();
        let Kind::Line { text, .. } = &first.kind else {
            unreachable!()
        };
        assert_eq!(&PLAY[text.clone()], "Amca, yine mi taş durdu?");
        assert_eq!(b.iter().filter(|b| b.numbered()).count(), 18);
        // The block's line number points at its source line.
        let lines: Vec<&str> = PLAY.lines().collect();
        assert!(lines[first.line].starts_with("\\se Amca"));
    }

    #[test]
    fn multi_line_and_joined() {
        let t = "\\karakter[x]{A}{a}\n\\begin{document}\n\\action{bir\niki}\n\\a bir %\niki\n\\a üç\n\\xyz yeni\n\\end{document}";
        let (_, b) = body_blocks(t);
        let real: Vec<&Block> = b
            .iter()
            .filter(|b| !matches!(b.kind, Kind::Blank))
            .collect();
        assert_eq!(&t[real[0].span.clone()], "\\action{bir\niki}");
        assert_eq!(&t[real[1].span.clone()], "\\a bir %\niki");
        assert!(matches!(real[2].kind, Kind::Line { .. }));
        assert_eq!(real[3].kind, Kind::Unknown { name: "xyz".into() });
    }

    #[test]
    fn inline_pieces() {
        let cast = Cast::of_play(PLAY);
        let t = "Ama \\oz bu \\ic{güler} _hiç_ 5\\% -- \\ru{Да} \\foo{x}";
        let pieces = inline(t, 0..t.len(), &cast, true);
        let kinds: Vec<String> = pieces
            .iter()
            .map(|p| match &p.kind {
                Inline::Text => format!("T{:?}", &t[p.span.clone()]),
                Inline::Ref(a) => format!("R{a}"),
                Inline::Ic { inner } => format!("I{}", &t[inner.clone()]),
                Inline::Under { inner } => format!("U{}", &t[inner.clone()]),
                Inline::Wrap { name, inner } => format!("W{name}:{}", &t[inner.clone()]),
                Inline::Special => format!("S{}", &t[p.span.clone()]),
                Inline::Raw => format!("X{}", &t[p.span.clone()]),
            })
            .collect();
        assert_eq!(
            kinds,
            vec![
                "T\"Ama \"",
                "Roz",
                "T\" bu \"",
                "Igüler",
                "T\" \"",
                "Uhiç",
                "T\" 5\"",
                "S\\%",
                "T\" \"",
                "S--",
                "T\" \"",
                "Wru:Да",
                "T\" \"",
                "X\\foo",
                "T\"{x}\"",
            ]
        );
        // The pieces cover the text exactly.
        assert_eq!(pieces.first().unwrap().span.start, 0);
        assert_eq!(pieces.last().unwrap().span.end, t.len());
        assert!(pieces.windows(2).all(|w| w[0].span.end == w[1].span.start));
    }

    #[test]
    fn words() {
        let cast = Cast::of_play(PLAY);
        let t = "Durdu kızım. \\ic{elini taşa koyar} Soğuk her şeyi durdurur.";
        assert_eq!(spoken_words(t, 0..t.len(), &cast), 6);
        let t = "Ama \\oz bu kış — başka.";
        assert_eq!(spoken_words(t, 0..t.len(), &cast), 5);
    }
}
