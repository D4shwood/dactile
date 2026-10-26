//! The text being edited: the whole `.tex` file, a cursor, a selection, and
//! undo. Every change is an edit on the text; the play model is rebuilt from
//! the text afterwards, never the other way round.

use crate::tex::{self, Edit};
use std::ops::Range;
use std::time::{Duration, Instant};

/// Typing within this long of the last keystroke joins the same undo step.
const COALESCE: Duration = Duration::from_millis(1500);
const UNDO_LIMIT: usize = 500;

#[derive(Debug, Clone)]
struct Step {
    text: String,
    cursor: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Last {
    None,
    Typing,
    Deleting,
}

pub struct Buffer {
    pub text: String,
    /// A byte offset on a character boundary.
    pub cursor: usize,
    /// The other end of the selection, when there is one.
    pub anchor: Option<usize>,
    /// Changed since the last save.
    pub dirty: bool,
    pub changed_at: Option<Instant>,
    undo: Vec<Step>,
    redo: Vec<Step>,
    last: Last,
    /// Where the last typed character ended, and whether it was a space.
    typed_end: Option<(usize, bool)>,
}

impl Buffer {
    pub fn new(text: String) -> Buffer {
        Buffer {
            text,
            cursor: 0,
            anchor: None,
            dirty: false,
            changed_at: None,
            undo: Vec::new(),
            redo: Vec::new(),
            last: Last::None,
            typed_end: None,
        }
    }

    pub fn selection(&self) -> Option<Range<usize>> {
        let a = self.anchor?;
        (a != self.cursor).then(|| a.min(self.cursor)..a.max(self.cursor))
    }

    pub fn selected_text(&self) -> Option<&str> {
        self.selection().map(|r| &self.text[r])
    }

    fn checkpoint(&mut self, kind: Last) {
        let joins = kind != Last::None
            && kind == self.last
            && self.changed_at.is_some_and(|t| t.elapsed() < COALESCE);
        if !joins {
            self.undo.push(Step {
                text: self.text.clone(),
                cursor: self.cursor,
            });
            if self.undo.len() > UNDO_LIMIT {
                self.undo.remove(0);
            }
        }
        self.redo.clear();
        self.last = kind;
    }

    fn changed(&mut self) {
        self.dirty = true;
        self.changed_at = Some(Instant::now());
    }

    /// Applies edits as one undo step and puts the cursor at `cursor`
    /// (a position in the new text).
    pub fn apply(&mut self, edits: &[Edit], cursor: usize) {
        if edits.is_empty() {
            return;
        }
        self.typed_end = None;
        self.checkpoint(Last::None);
        self.text = tex::apply(&self.text, edits);
        self.cursor = cursor.min(self.text.len());
        self.anchor = None;
        self.changed();
    }

    /// Types `s` at the cursor, replacing the selection.
    pub fn insert(&mut self, s: &str) {
        let range = self.selection().unwrap_or(self.cursor..self.cursor);
        // A word and the space before it are one undo step: typing joins the
        // step unless the cursor moved or a word just ended in a space.
        let single = range.is_empty() && s.chars().count() == 1;
        let space = s.starts_with(char::is_whitespace);
        let continues = self
            .typed_end
            .is_some_and(|(end, was_space)| end == range.start && !(space && !was_space));
        if single && !continues {
            self.last = Last::None;
        }
        self.checkpoint(if single { Last::Typing } else { Last::None });
        self.typed_end = single.then(|| (range.start + s.len(), space));
        self.text.replace_range(range.clone(), s);
        self.cursor = range.start + s.len();
        self.anchor = None;
        self.changed();
    }

    /// Deletes `range` (or the selection, if there is one).
    pub fn delete(&mut self, range: Range<usize>) {
        let range = self.selection().unwrap_or(range);
        if range.is_empty() {
            return;
        }
        self.typed_end = None;
        self.checkpoint(Last::Deleting);
        self.text.replace_range(range.clone(), "");
        self.cursor = range.start;
        self.anchor = None;
        self.changed();
    }

    pub fn undo(&mut self) -> bool {
        let Some(step) = self.undo.pop() else {
            return false;
        };
        self.redo.push(Step {
            text: std::mem::replace(&mut self.text, step.text),
            cursor: self.cursor,
        });
        self.cursor = step.cursor.min(self.text.len());
        self.anchor = None;
        self.last = Last::None;
        self.changed();
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(step) = self.redo.pop() else {
            return false;
        };
        self.undo.push(Step {
            text: std::mem::replace(&mut self.text, step.text),
            cursor: self.cursor,
        });
        self.cursor = step.cursor.min(self.text.len());
        self.anchor = None;
        self.last = Last::None;
        self.changed();
        true
    }

    /// Replaces the whole text (a reload from disk) without an undo step.
    pub fn reload(&mut self, text: String) {
        self.cursor = self.cursor.min(text.len());
        while !text.is_char_boundary(self.cursor) {
            self.cursor -= 1;
        }
        self.text = text;
        self.anchor = None;
        self.dirty = false;
        self.undo.clear();
        self.redo.clear();
    }

    pub fn prev_char(&self, at: usize) -> usize {
        self.text[..at]
            .char_indices()
            .next_back()
            .map_or(0, |(i, _)| i)
    }

    pub fn next_char(&self, at: usize) -> usize {
        self.text[at..]
            .chars()
            .next()
            .map_or(at, |c| at + c.len_utf8())
    }
}

/// What typing a character writes into the file, outside raw editing: TeX's
/// special characters escaped so they print as typed. `_` stays as it is in
/// dialogue, where `_word_` underlines.
pub fn escape_typed(c: char, in_dialogue: bool) -> String {
    match c {
        '%' | '#' | '$' | '&' | '{' | '}' => format!("\\{c}"),
        '_' if in_dialogue => "_".into(),
        '_' => "\\_".into(),
        '^' => "\\^{}".into(),
        '~' => "\\textasciitilde{}".into(),
        c => c.to_string(),
    }
}

/// Pasted text, escaped the same way, backslashes included.
pub fn escape_pasted(s: &str, in_dialogue: bool) -> String {
    s.replace("\r\n", "\n")
        .chars()
        .map(|c| match c {
            '\\' => "\\textbackslash{}".to_string(),
            '\t' => " ".to_string(),
            c => escape_typed(c, in_dialogue),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typing_and_undo() {
        let mut b = Buffer::new("ab".into());
        b.cursor = 2;
        for c in ["c", "d", " ", "e", "f"] {
            b.insert(c);
        }
        assert_eq!(b.text, "abcd ef");
        // " ef" is one step, "cd" another.
        assert!(b.undo());
        assert_eq!(b.text, "abcd");
        assert!(b.undo());
        assert_eq!(b.text, "ab");
        assert!(b.redo());
        assert_eq!(b.text, "abcd");
        assert_eq!(b.cursor, 4);
        // Moving the cursor starts a new step.
        let mut b = Buffer::new("x".into());
        b.cursor = 1;
        b.insert("y");
        b.cursor = 0;
        b.insert("z");
        b.undo();
        assert_eq!(b.text, "xy");
    }

    #[test]
    fn selection_is_replaced() {
        let mut b = Buffer::new("merhaba dünya".into());
        b.anchor = Some(8);
        b.cursor = 14;
        assert_eq!(b.selected_text(), Some("dünya"));
        b.insert("x");
        assert_eq!(b.text, "merhaba x");
        b.anchor = Some(0);
        b.cursor = 8;
        b.delete(0..0);
        assert_eq!(b.text, "x");
    }

    #[test]
    fn edits_are_one_step() {
        let mut b = Buffer::new("\\a x \\a".into());
        b.apply(&[Edit::new(1..2, "bb"), Edit::new(6..7, "bb")], 0);
        assert_eq!(b.text, "\\bb x \\bb");
        b.undo();
        assert_eq!(b.text, "\\a x \\a");
    }

    #[test]
    fn escaping() {
        assert_eq!(escape_typed('%', true), "\\%");
        assert_eq!(escape_typed('_', true), "_");
        assert_eq!(escape_typed('_', false), "\\_");
        assert_eq!(escape_typed('ş', false), "ş");
        assert_eq!(escape_pasted("5% \\o/", false), "5\\% \\textbackslash{}o/");
    }

    #[test]
    fn char_steps() {
        let b = Buffer::new("aş".into());
        assert_eq!(b.next_char(1), 3);
        assert_eq!(b.prev_char(3), 1);
    }
}
