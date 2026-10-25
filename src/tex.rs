//! Reading TeX source the way TeX itself splits it, as far as plays need:
//! comments, control words and arguments, all as byte positions in the text.

use std::ops::Range;

/// A letter for a control word. XeLaTeX treats Unicode letters as letters, so
/// `\maş` is one command, not `\ma` followed by "ş".
pub fn is_letter(c: char) -> bool {
    c.is_alphabetic()
}

/// Whether the byte at `at` is escaped by an odd run of backslashes before it.
pub fn is_escaped(text: &str, at: usize) -> bool {
    text.as_bytes()[..at]
        .iter()
        .rev()
        .take_while(|&&b| b == b'\\')
        .count()
        % 2
        == 1
}

/// Where the comment on the line holding `line_start` begins, if any: the
/// first unescaped `%`.
pub fn comment_start(line: &str) -> Option<usize> {
    line.char_indices()
        .find(|&(i, c)| c == '%' && !is_escaped(line, i))
        .map(|(i, _)| i)
}

/// Line ranges of `text`, each without its line ending ("\n" or "\r\n").
pub fn lines(text: &str) -> Vec<Range<usize>> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut start = 0;
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'\n' {
            let end = if i > start && bytes[i - 1] == b'\r' {
                i - 1
            } else {
                i
            };
            out.push(start..end);
            start = i + 1;
        }
    }
    if start < text.len() {
        out.push(start..text.len());
    }
    out
}

/// A control word found in the text.
#[derive(Debug, Clone, PartialEq)]
pub struct Command<'a> {
    /// The name without the backslash.
    pub name: &'a str,
    /// From the backslash to the end of the name.
    pub span: Range<usize>,
}

/// Every control word in `range` outside comments, in order.
pub fn commands(text: &str, range: Range<usize>) -> Vec<Command<'_>> {
    let mut out = Vec::new();
    let mut i = range.start;
    while i < range.end {
        let rest = &text[i..range.end];
        let Some(c) = rest.chars().next() else { break };
        match c {
            '%' => {
                // Skip to the end of the line.
                i = text[i..range.end]
                    .find('\n')
                    .map_or(range.end, |n| i + n + 1);
            }
            '\\' => {
                let name_len: usize = rest[1..]
                    .chars()
                    .take_while(|&c| is_letter(c))
                    .map(char::len_utf8)
                    .sum();
                if name_len > 0 {
                    out.push(Command {
                        name: &text[i + 1..i + 1 + name_len],
                        span: i..i + 1 + name_len,
                    });
                    i += 1 + name_len;
                } else {
                    // A control symbol like \% or \\: skip the backslash and the symbol.
                    i += 1 + rest[1..].chars().next().map_or(0, char::len_utf8);
                }
            }
            _ => i += c.len_utf8(),
        }
    }
    out
}

/// A `{…}` or `[…]` argument.
#[derive(Debug, Clone, PartialEq)]
pub struct Arg<'a> {
    /// The text inside the delimiters.
    pub inner: &'a str,
    pub inner_span: Range<usize>,
    /// Including the delimiters.
    pub span: Range<usize>,
}

/// Skips spaces, tabs and at most one line break, as TeX does between a
/// command and its arguments.
pub fn skip_space(text: &str, mut at: usize) -> usize {
    let mut newlines = 0;
    while let Some(c) = text[at..].chars().next() {
        match c {
            ' ' | '\t' | '\r' => at += 1,
            '\n' if newlines == 0 => {
                newlines += 1;
                at += 1
            }
            _ => break,
        }
    }
    at
}

/// A balanced `{…}` starting at `at` (after spaces).
pub fn group(text: &str, at: usize) -> Option<Arg<'_>> {
    let start = skip_space(text, at);
    if !text[start..].starts_with('{') {
        return None;
    }
    let close = matching_brace(text, start)?;
    Some(Arg {
        inner: &text[start + 1..close],
        inner_span: start + 1..close,
        span: start..close + 1,
    })
}

/// An optional `[…]` starting at `at` (after spaces). Brackets inside braces
/// don't end it.
pub fn optional(text: &str, at: usize) -> Option<Arg<'_>> {
    let start = skip_space(text, at);
    if !text[start..].starts_with('[') {
        return None;
    }
    let mut depth = 0usize;
    let mut i = start + 1;
    while i < text.len() {
        let c = text[i..].chars().next()?;
        match c {
            '\\' => {
                i += 1 + text[i + 1..].chars().next().map_or(0, char::len_utf8);
                continue;
            }
            '{' => depth += 1,
            '}' => depth = depth.saturating_sub(1),
            ']' if depth == 0 => {
                return Some(Arg {
                    inner: &text[start + 1..i],
                    inner_span: start + 1..i,
                    span: start..i + 1,
                });
            }
            '%' => {
                i = text[i..].find('\n').map_or(text.len(), |n| i + n + 1);
                continue;
            }
            _ => {}
        }
        i += c.len_utf8();
    }
    None
}

/// A `*` right after a command.
pub fn star(text: &str, at: usize) -> Option<Range<usize>> {
    text[at..].starts_with('*').then(|| at..at + 1)
}

/// The `}` closing the `{` at `open`, skipping escaped braces and comments.
pub fn matching_brace(text: &str, open: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut i = open;
    while i < text.len() {
        let c = text[i..].chars().next()?;
        match c {
            '\\' => {
                i += 1 + text[i + 1..].chars().next().map_or(0, char::len_utf8);
                continue;
            }
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            '%' => {
                i = text[i..].find('\n').map_or(text.len(), |n| i + n + 1);
                continue;
            }
            _ => {}
        }
        i += c.len_utf8();
    }
    None
}

/// A change to the text: replace `range` with `text`.
#[derive(Debug, Clone, PartialEq)]
pub struct Edit {
    pub range: Range<usize>,
    pub text: String,
}

impl Edit {
    pub fn new(range: Range<usize>, text: impl Into<String>) -> Edit {
        Edit {
            range,
            text: text.into(),
        }
    }
}

/// Applies edits that don't overlap, in any order.
pub fn apply(text: &str, edits: &[Edit]) -> String {
    let mut sorted: Vec<&Edit> = edits.iter().collect();
    sorted.sort_by_key(|e| (e.range.start, e.range.end));
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    for e in sorted {
        debug_assert!(e.range.start >= at, "overlapping edits");
        out.push_str(&text[at..e.range.start]);
        out.push_str(&e.text);
        at = e.range.end;
    }
    out.push_str(&text[at..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_words_take_unicode_letters() {
        let t = r"\maş \ma{x} \mado\% \\ \sahne[a]";
        let names: Vec<&str> = commands(t, 0..t.len()).iter().map(|c| c.name).collect();
        assert_eq!(names, vec!["maş", "ma", "mado", "sahne"]);
    }

    #[test]
    fn comments_hide_commands() {
        let t = "\\a % \\b\n\\c \\% \\d";
        let names: Vec<&str> = commands(t, 0..t.len()).iter().map(|c| c.name).collect();
        assert_eq!(names, vec!["a", "c", "d"]);
        assert_eq!(comment_start("a \\% b % c"), Some(7));
    }

    #[test]
    fn arguments() {
        let t = "\\k*[Ad [x]][{a]b} c]\n {SELİN}{se}";
        let at = "\\k".len();
        assert_eq!(star(t, at), Some(at..at + 1));
        let o1 = optional(t, at + 1).unwrap();
        // A bracket ends at the first "]" outside braces.
        assert_eq!(o1.inner, "Ad [x");
        let t = "\\k[{a]b} c]{x}";
        let o = optional(t, 2).unwrap();
        assert_eq!(o.inner, "{a]b} c");
        let g = group(t, o.span.end).unwrap();
        assert_eq!(g.inner, "x");
        let t = "\\k{a{b}\\}c}";
        assert_eq!(group(t, 2).unwrap().inner, "a{b}\\}c");
    }

    #[test]
    fn line_ranges() {
        let t = "a\r\nbc\n\nd";
        let got: Vec<&str> = lines(t).into_iter().map(|r| &t[r]).collect();
        assert_eq!(got, vec!["a", "bc", "", "d"]);
        let t = "a\n";
        assert_eq!(
            lines(t).into_iter().map(|r| &t[r]).collect::<Vec<_>>(),
            vec!["a"]
        );
    }

    #[test]
    fn edits() {
        let t = "abcdef";
        let out = apply(
            t,
            &[
                Edit::new(4..5, "X"),
                Edit::new(0..1, ""),
                Edit::new(2..2, "-"),
            ],
        );
        assert_eq!(out, "b-cdXf");
    }
}
