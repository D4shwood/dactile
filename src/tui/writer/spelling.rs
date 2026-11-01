//! Spell checking the words on screen.

use super::*;

impl Writer {
    pub(super) fn refresh_known(&mut self) {
        self.known = spell::cast_words(&self.doc.cast)
            .into_iter()
            .chain(self.sidecar.words.iter().cloned())
            .map(|w| dcast::lower_tr(&w))
            .collect();
        self.spell_cache.clear();
    }

    /// The misspelt words of the blocks on screen.
    pub(super) fn mark_misspelt(&mut self) {
        self.misspelt.clear();
        if self.speller.is_none() {
            return;
        }
        let h = self.text_height();
        let mut blocks: Vec<usize> = self
            .lay
            .rows
            .iter()
            .skip(self.shown)
            .take(h)
            .filter_map(|r| r.block)
            .collect();
        blocks.dedup();
        let Some(sp) = &self.speller else { return };
        let text = &self.buf.text;
        for b in blocks {
            for r in spell::words(text, &self.doc, b) {
                if !word_ok(sp, &self.known, &mut self.spell_cache, &text[r.clone()]) {
                    self.misspelt.push(r);
                }
            }
        }
    }

    pub(super) fn suggest(&mut self) {
        if self.speller.is_none() {
            let why = if self.speller_rx.is_some() {
                lang::l(
                    "Sözlük yükleniyor, birazdan.",
                    "The dictionary is loading, a moment.",
                )
            } else {
                lang::l(
                    "Türkçe sözlük bulunamadı (tr_TR.aff/.dic): paru -S hunspell-tr ile kurulabilir.",
                    "No Turkish dictionary (tr_TR.aff/.dic): install hunspell-tr.",
                )
            };
            self.say(why, true);
            return;
        }
        let cur = self.buf.cursor;
        let Some(b) = self.block() else { return };
        let words = spell::words(&self.buf.text, &self.doc, b);
        let Some(range) = words.into_iter().find(|r| r.start <= cur && cur <= r.end) else {
            self.say(
                lang::l(
                    "İmleç bir kelimenin üstünde değil.",
                    "The cursor isn't on a word.",
                ),
                false,
            );
            return;
        };
        let word = self.buf.text[range.clone()].to_string();
        let mut items = self
            .speller
            .as_ref()
            .map(|s| s.suggest(&word))
            .unwrap_or_default();
        items.push(lang::f(
            "+ «{}» oyunun sözlüğüne",
            "+ «{}» to the play's words",
            &[&word],
        ));
        self.popup = Popup::Suggest(range, items, 0);
    }
}

/// Whether a word is spelt right: a name in the play, or in the dictionary.
pub(super) fn word_ok(
    sp: &Speller,
    known: &HashSet<String>,
    cache: &mut HashMap<String, bool>,
    w: &str,
) -> bool {
    if let Some(&ok) = cache.get(w) {
        return ok;
    }
    let lower = dcast::lower_tr(w);
    let stem = lower.split(['\'', '’']).next().unwrap_or("");
    let ok = known.contains(&lower) || known.contains(stem) || sp.check(w);
    cache.insert(w.to_string(), ok);
    ok
}
