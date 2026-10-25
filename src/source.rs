//! A play's `.tex` file as one text, split into its three parts. Nothing is
//! ever rebuilt from a model: changes are edits on the text, so whatever the
//! app doesn't touch stays byte for byte as it was.

use crate::tex;
use std::ops::Range;

#[derive(Debug, Clone, PartialEq)]
pub struct Parts {
    /// From the start of the file to `\begin{document}`: the class, the cover
    /// details and the characters.
    pub preamble: Range<usize>,
    /// Between `\begin{document}` and `\end{document}`: the play.
    pub body: Range<usize>,
    /// From `\end{document}` to the end of the file.
    pub tail: Range<usize>,
}

/// Splits on the first `\begin{document}` and the last `\end{document}`
/// outside comments. A file without them is all preamble or all body.
pub fn split(text: &str) -> Parts {
    let begin = find_env(text, "\\begin{document}", false);
    let end = find_env(text, "\\end{document}", true);
    match (begin, end) {
        (Some(b), Some(e)) if e.start >= b.end => Parts {
            preamble: 0..b.start,
            body: b.end..e.start,
            tail: e.start..text.len(),
        },
        (Some(b), _) => Parts {
            preamble: 0..b.start,
            body: b.end..text.len(),
            tail: text.len()..text.len(),
        },
        // A fragment (e.g. an act kept in its own file): all body.
        (None, _) => Parts {
            preamble: 0..0,
            body: 0..text.len(),
            tail: text.len()..text.len(),
        },
    }
}

fn find_env(text: &str, needle: &str, last: bool) -> Option<Range<usize>> {
    let hits = text.match_indices(needle).map(|(i, _)| i).filter(|&i| {
        let line_start = text[..i].rfind('\n').map_or(0, |n| n + 1);
        let before = &text[line_start..i];
        tex::comment_start(before).is_none() && !tex::is_escaped(text, i)
    });
    let at = if last {
        hits.last()
    } else {
        hits.into_iter().next()
    }?;
    Some(at..at + needle.len())
}

/// How the file ends its lines, so inserted lines match.
pub fn line_ending(text: &str) -> &'static str {
    match text.find('\n') {
        Some(i) if i > 0 && text.as_bytes()[i - 1] == b'\r' => "\r\n",
        _ => "\n",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_parts() {
        let t = "\\documentclass{tiyatro}\n% \\begin{document} not this\n\\begin{document}\nX\n\\end{document}\n";
        let p = split(t);
        assert!(t[p.preamble.clone()].ends_with("not this\n"));
        assert_eq!(&t[p.body.clone()], "\nX\n");
        assert_eq!(&t[p.tail.clone()], "\\end{document}\n");
        // The three parts cover the file exactly.
        assert_eq!(p.preamble.end, p.body.start - "\\begin{document}".len());
        assert_eq!(p.tail.end, t.len());
    }

    #[test]
    fn fragments() {
        let p = split("\\se Merhaba\n");
        assert_eq!(p.body, 0..12);
        let p = split("\\begin{document}\n\\se x");
        assert_eq!(p.body.end, 22);
    }

    #[test]
    fn endings() {
        assert_eq!(line_ending("a\r\nb"), "\r\n");
        assert_eq!(line_ending("a\nb"), "\n");
        assert_eq!(line_ending("a"), "\n");
    }
}
