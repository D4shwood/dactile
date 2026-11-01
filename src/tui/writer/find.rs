//! Finding and replacing.

use super::*;

impl Writer {
    /// The next match of `needle` after the cursor (wrapping), ignoring case
    /// the Turkish way.
    pub(super) fn find_from(&self, needle: &str, from: usize) -> Option<std::ops::Range<usize>> {
        if needle.is_empty() {
            return None;
        }
        let t = &self.buf.text;
        let n = dcast::lower_tr(needle);
        let n: Vec<char> = n.chars().collect();
        let starts: Vec<usize> = t.char_indices().map(|(i, _)| i).collect();
        let try_at = |i: usize| -> Option<usize> {
            let mut end = i;
            for &c in &n {
                let ch = t[end..].chars().next()?;
                let low: Vec<char> = dcast::lower_tr(&ch.to_string()).chars().collect();
                if low.first() != Some(&c) {
                    return None;
                }
                end += ch.len_utf8();
            }
            Some(end)
        };
        let body = self.doc.parts.body.clone();
        let after = starts.iter().filter(|&&i| i >= from && i < body.end);
        let before = starts.iter().filter(|&&i| i >= body.start && i < from);
        after.chain(before).find_map(|&i| try_at(i).map(|e| i..e))
    }

    pub(super) fn find_next(&mut self, needle: &str) {
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
        let matches = self
            .buf
            .selected_text()
            .is_some_and(|s| dcast::lower_tr(s) == dcast::lower_tr(needle));
        if matches && self.editable() {
            let r = self.buf.selection().unwrap();
            let at = r.start;
            self.buf.apply(
                &[dactile::tex::Edit::new(r, with.to_string())],
                at + with.len(),
            );
            self.edited();
        }
        self.find_next(needle);
    }

    pub(super) fn replace_all(&mut self, needle: &str, with: &str) {
        if needle.is_empty() || !self.editable() {
            return;
        }
        let mut edits = Vec::new();
        let mut at = self.doc.parts.body.start;
        while let Some(r) = self.find_from(needle, at) {
            if r.start < at
                || edits
                    .iter()
                    .any(|e: &dactile::tex::Edit| e.range.start == r.start)
            {
                break;
            }
            at = r.end;
            edits.push(dactile::tex::Edit::new(r, with.to_string()));
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
