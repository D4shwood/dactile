//! Spelling, as the terminal app checks it: the Turkish Hunspell
//! dictionary, the characters' names and the play's own words (kept in its
//! `.dactile.toml`). Only the words of the blocks in sight are checked.

use crate::view::{byte_at, unit_at};
use dactile::cast as dcast;
use dactile::render::Doc;
use dactile::spell::{self, Speller};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, OnceLock};

/// The dictionary, loaded once in the background; why not, when it isn't.
pub type Shared = Arc<OnceLock<Result<Speller, String>>>;

pub fn load(into: Shared) {
    std::thread::spawn(move || {
        let r = match spell::find() {
            Some((aff, dic)) => Speller::load(&aff, &dic).map_err(|e| format!("{e:#}")),
            None => Err(dactile::lang::l(
                "Türkçe sözlük bulunamadı (tr_TR.aff/.dic): paru -S hunspell-tr ile kurulabilir.",
                "No Turkish dictionary (tr_TR.aff/.dic): install hunspell-tr.",
            )
            .to_string()),
        };
        let _ = into.set(r);
    });
}

/// What counts as spelt right besides the dictionary, and what was asked.
#[derive(Default)]
pub struct Known {
    /// The play's own words (`.dactile.toml`), lower case.
    pub words: HashSet<String>,
    cache: HashMap<String, bool>,
}

impl Known {
    pub fn set_words(&mut self, words: &[String]) {
        self.words = words.iter().map(|w| dcast::lower_tr(w)).collect();
        self.cache.clear();
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Checked {
    Loading,
    Missing { why: String },
    Words { ranges: Vec<(usize, usize)> },
}

/// The misspelt words of the blocks between UTF-16 offsets `from` and `to`.
pub fn misspelt(sp: &Shared, known: &mut Known, text: &str, from: usize, to: usize) -> Checked {
    let sp = match sp.get() {
        None => return Checked::Loading,
        Some(Err(why)) => return Checked::Missing { why: why.clone() },
        Some(Ok(s)) => s,
    };
    let doc = Doc::new(text);
    let names: HashSet<String> = spell::cast_words(&doc.cast)
        .into_iter()
        .map(|w| dcast::lower_tr(&w))
        .collect();
    let (a, b) = (byte_at(text, from), byte_at(text, to));
    let mut ranges = Vec::new();
    for i in doc.body.clone() {
        let span = &doc.blocks[i].span;
        if span.end < a || span.start > b {
            continue;
        }
        for r in spell::words(text, &doc, i) {
            let w = &text[r.clone()];
            let ok = match known.cache.get(w) {
                Some(&ok) => ok,
                None => {
                    let lower = dcast::lower_tr(w);
                    let stem = lower.split(['\'', '’']).next().unwrap_or("");
                    let ok = names.contains(&lower)
                        || names.contains(stem)
                        || known.words.contains(&lower)
                        || known.words.contains(stem)
                        || sp.check(w);
                    // Names change with the play: only the dictionary's
                    // answers are kept.
                    if !names.contains(&lower) && !names.contains(stem) {
                        known.cache.insert(w.to_string(), ok);
                    }
                    ok
                }
            };
            if !ok {
                ranges.push((unit_at(text, r.start), unit_at(text, r.end)));
            }
        }
    }
    Checked::Words { ranges }
}

pub fn suggest(sp: &Shared, word: &str) -> Vec<String> {
    match sp.get() {
        Some(Ok(s)) => {
            // "durdu" and "Durdu" are one suggestion: the one first given.
            let mut seen = HashSet::new();
            s.suggest(word)
                .into_iter()
                .filter(|w| seen.insert(dcast::lower_tr(w)))
                .take(8)
                .collect()
        }
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLAY: &str = include_str!("../../../examples/ornek-oyun.tex");

    #[test]
    #[ignore] // loads the dictionary: seconds
    fn a_made_up_word_is_marked_and_a_name_is_not() {
        let Some((aff, dic)) = spell::find() else {
            return; // no dictionary here
        };
        let sp: Shared = Default::default();
        let _ = sp.set(Ok(Speller::load(&aff, &dic).unwrap()));
        let text = PLAY.replacen("taş durdu", "taş dzrdu", 1);
        let units = text.encode_utf16().count();
        let mut known = Known::default();
        let Checked::Words { ranges } = misspelt(&sp, &mut known, &text, 0, units) else {
            panic!("not checked");
        };
        let all: Vec<u16> = text.encode_utf16().collect();
        let words: Vec<String> = ranges
            .iter()
            .map(|&(a, b)| String::from_utf16(&all[a..b]).unwrap())
            .collect();
        assert!(words.contains(&"dzrdu".to_string()), "{words:?}");
        known.set_words(&["dzrdu".into()]);
        let Checked::Words { ranges } = misspelt(&sp, &mut known, &text, 0, units) else {
            panic!("not checked");
        };
        assert_eq!(ranges.len(), words.len() - 1);
    }
}
