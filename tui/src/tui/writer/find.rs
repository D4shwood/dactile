//! Finding and replacing.

use super::*;
use regex::{Regex, RegexBuilder};

/// How the find form looks: each stays on for the next search.
#[derive(Clone, Copy, Default, PartialEq)]
pub struct FindOptions {
    /// Capitals and small letters apart.
    pub case: bool,
    pub whole_word: bool,
    pub regexp: bool,
    /// The cover and cast lines too, not just the play's body.
    pub whole_file: bool,
}

impl FindOptions {
    /// `needle` as a regular expression, in these options.
    fn regex(self, needle: &str) -> Result<Regex, regex::Error> {
        let mut p = if self.regexp {
            format!("(?:{needle})")
        } else {
            regex::escape(needle)
        };
        if self.whole_word {
            p = format!(r"\b{p}\b");
        }
        RegexBuilder::new(&p)
            .case_insensitive(!self.case)
            .multi_line(true)
            .build()
    }
}

impl Writer {
    /// The next match of `needle` after the cursor (wrapping), ignoring case
    /// the Turkish way unless asked not to.
    pub(super) fn find_from(&self, needle: &str, from: usize) -> Option<std::ops::Range<usize>> {
        if needle.is_empty() {
            return None;
        }
        let o = self.find_options;
        let t = &self.buf.text;
        let area = if o.whole_file {
            0..t.len()
        } else {
            self.doc.parts.body.clone()
        };
        if o.regexp {
            let re = o.regex(needle).ok()?;
            let mut first = None;
            for m in re.find_iter(&t[area.clone()]).filter(|m| !m.is_empty()) {
                let r = area.start + m.start()..area.start + m.end();
                if r.start >= from {
                    return Some(r);
                }
                first.get_or_insert(r);
            }
            return first;
        }
        let fold = |s: &str| {
            if o.case {
                s.to_string()
            } else {
                dcast::lower_tr(s)
            }
        };
        let n = fold(needle);
        let n: Vec<char> = n.chars().collect();
        let starts: Vec<usize> = t.char_indices().map(|(i, _)| i).collect();
        let try_at = |i: usize| -> Option<usize> {
            let mut end = i;
            for &c in &n {
                let ch = t[end..].chars().next()?;
                let low: Vec<char> = fold(&ch.to_string()).chars().collect();
                if low.first() != Some(&c) {
                    return None;
                }
                end += ch.len_utf8();
            }
            let apart = !o.whole_word
                || !t[..i]
                    .chars()
                    .next_back()
                    .is_some_and(char::is_alphanumeric)
                    && !t[end..].chars().next().is_some_and(char::is_alphanumeric);
            apart.then_some(end)
        };
        let after = starts.iter().filter(|&&i| i >= from && i < area.end);
        let before = starts.iter().filter(|&&i| i >= area.start && i < from);
        after.chain(before).find_map(|&i| try_at(i).map(|e| i..e))
    }

    /// What goes in place of the match at `r`: with a regexp, `$1` is its
    /// first group.
    fn replacement(&self, needle: &str, r: &std::ops::Range<usize>, with: &str) -> String {
        let o = self.find_options;
        if !o.regexp {
            return with.to_string();
        }
        let caps = o
            .regex(needle)
            .ok()
            .and_then(|re| re.captures_at(&self.buf.text, r.start));
        let Some(caps) = caps else {
            return with.to_string();
        };
        let mut out = String::new();
        caps.expand(with, &mut out);
        out
    }

    pub(super) fn find_next(&mut self, needle: &str) {
        if self.find_options.regexp
            && let Err(e) = self.find_options.regex(needle)
        {
            self.say(
                &lang::f("Düzenli ifade bozuk: {}", "A broken regexp: {}", &[&e]),
                true,
            );
            return;
        }
        let from = self.buf.selection().map_or(self.buf.cursor, |s| s.end);
        match self.find_from(needle, from) {
            Some(r) => {
                // Show the line raw when the match is inside a macro.
                self.buf.anchor = Some(r.start);
                self.buf.cursor = r.end;
                if !self.lay.is_stop(r.start) || !self.lay.is_stop(r.end) {
                    self.raw_here = true;
                    self.relayout();
                }
                self.keep_visible();
            }
            None => self.say(
                &lang::f("«{}» bulunamadı.", "«{}» not found.", &[&needle]),
                false,
            ),
        }
    }

    pub(super) fn replace_one(&mut self, needle: &str, with: &str) {
        let found = self
            .buf
            .selection()
            .filter(|s| self.find_from(needle, s.start).as_ref() == Some(s));
        if let Some(r) = found
            && self.editable()
        {
            let at = r.start;
            let with = self.replacement(needle, &r, with);
            let end = at + with.len();
            self.buf.apply(&[dactile::tex::Edit::new(r, with)], end);
            self.edited();
        }
        self.find_next(needle);
    }

    pub(super) fn replace_all(&mut self, needle: &str, with: &str) {
        if needle.is_empty() || !self.editable() {
            return;
        }
        let mut edits = Vec::new();
        let mut at = if self.find_options.whole_file {
            0
        } else {
            self.doc.parts.body.start
        };
        while let Some(r) = self.find_from(needle, at) {
            if r.start < at
                || edits
                    .iter()
                    .any(|e: &dactile::tex::Edit| e.range.start == r.start)
            {
                break;
            }
            at = r.end;
            let with = self.replacement(needle, &r, with);
            edits.push(dactile::tex::Edit::new(r, with));
        }
        let n = edits.len();
        if n > 0 {
            let cursor = self.buf.cursor.min(self.buf.text.len());
            self.buf.apply(&edits, cursor.min(edits[0].range.start));
            self.edited();
        }
        self.say(
            &lang::f(
                "{} yer değişti (^Z geri alır).",
                "{} replaced (^Z undoes).",
                &[&n],
            ),
            false,
        );
    }

    /// Puts `text` on a line of its own at `at` and the cursor on the line after.
    pub(super) fn write_line(&mut self, at: usize, text: &str, nl: &str) {
        let line_start = self.buf.text[..at].rfind('\n').map_or(0, |i| i + 1);
        let line_end = self.buf.text[at..]
            .find(['\r', '\n'])
            .map_or(self.buf.text.len(), |i| at + i);
        let alone = self.buf.text[line_start..line_end].trim().is_empty();
        let (range, insert) = if alone {
            (line_start..line_end, format!("{text}{nl}"))
        } else {
            (at..at, format!("{nl}{text}{nl}"))
        };
        let cursor = range.start + insert.len();
        self.buf
            .apply(&[dactile::tex::Edit::new(range, insert)], cursor);
        self.raw_here = false;
        self.edited();
        self.snap_forward_if_inside();
    }
}
