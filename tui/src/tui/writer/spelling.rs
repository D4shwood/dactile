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
        let here = words.into_iter().find(|r| r.start <= cur && cur <= r.end);
        // A word that is fine: on to the next one that isn't, in sight.
        let range = match (here, next_misspelt(&self.misspelt, cur)) {
            (Some(r), _) if self.misspelt.contains(&r) => r,
            (_, Some(r)) => {
                self.go(r.start, false);
                self.keep_visible();
                r
            }
            (Some(r), None) => r,
            (None, None) => {
                self.say(
                    lang::l(
                        "İmleç bir kelimenin üstünde değil.",
                        "The cursor isn't on a word.",
                    ),
                    false,
                );
                return;
            }
        };
        let word = self.buf.text[range.clone()].to_string();
        let mut items = self
            .speller
            .as_ref()
            .map(|s| few(s.suggest(&word)))
            .unwrap_or_default();
        items.push(lang::f(
            "+ «{}» oyunun sözlüğüne",
            "+ «{}» to the play's words",
            &[&word],
        ));
        self.popup = Popup::Suggest(range, items, 0);
    }
}

/// The first misspelt word after `at`, else the first of them.
fn next_misspelt(words: &[std::ops::Range<usize>], at: usize) -> Option<std::ops::Range<usize>> {
    let mut sorted: Vec<_> = words.to_vec();
    sorted.sort_by_key(|r| r.start);
    let after = sorted.iter().find(|r| r.start > at);
    after.or(sorted.first()).cloned()
}

/// "durdu" and "Durdu" are one suggestion, the one first given; eight at most.
fn few(words: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    words
        .into_iter()
        .filter(|w| seen.insert(dcast::lower_tr(w)))
        .take(8)
        .collect()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_next_misspelt_word_comes_round() {
        let words = [20..24, 5..9];
        assert_eq!(next_misspelt(&words, 0), Some(5..9));
        assert_eq!(next_misspelt(&words, 5), Some(20..24));
        assert_eq!(next_misspelt(&words, 30), Some(5..9));
        assert_eq!(next_misspelt(&[], 0), None);
    }

    #[test]
    fn suggestions_have_no_doubles_and_eight_at_most() {
        let many = ["durdu", "Durdu", "DURDU", "durma"].map(String::from);
        assert_eq!(few(many.to_vec()), ["durdu", "durma"]);
        let long: Vec<String> = (0..20).map(|i| format!("söz{i}")).collect();
        assert_eq!(few(long).len(), 8);
    }
}
