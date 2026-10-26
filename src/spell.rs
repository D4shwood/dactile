//! Turkish spell checking with a Hunspell dictionary (`tr_TR.aff/.dic`),
//! for the words of the play only: never macros, comments or passages in
//! other languages.

use crate::cast::Cast;
use crate::lex::{self, Inline, Kind};
use crate::render::Doc;
use anyhow::{Context, Result};
use std::collections::HashSet;
use std::ops::Range;
use std::path::PathBuf;

pub struct Speller {
    dict: spellbook::Dictionary,
    /// The play's own words: character names, words added by hand.
    extra: HashSet<String>,
}

/// Where a `tr_TR` dictionary may be: $DACTILE_DICT, then the usual places.
pub fn find() -> Option<(PathBuf, PathBuf)> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    if let Some(d) = std::env::var_os("DACTILE_DICT") {
        dirs.push(d.into());
    }
    if let Some(d) = dirs::data_dir() {
        dirs.push(d.join("dactile/dict"));
        dirs.push(d.join("hunspell"));
    }
    for d in [
        "/usr/share/hunspell",
        "/usr/share/myspell/dicts",
        "/usr/share/myspell",
        "/usr/local/share/hunspell",
    ] {
        dirs.push(d.into());
    }
    dirs.into_iter().find_map(|d| {
        let aff = d.join("tr_TR.aff");
        let dic = d.join("tr_TR.dic");
        (aff.is_file() && dic.is_file()).then_some((aff, dic))
    })
}

impl Speller {
    /// Loads the dictionary; a few seconds for Turkish, so callers run it
    /// in the background.
    pub fn load(aff: &std::path::Path, dic: &std::path::Path) -> Result<Speller> {
        let aff =
            std::fs::read_to_string(aff).with_context(|| format!("reading {}", aff.display()))?;
        let dic =
            std::fs::read_to_string(dic).with_context(|| format!("reading {}", dic.display()))?;
        let (aff, dic) = without_zero_flag(aff, dic);
        let dict = spellbook::Dictionary::new(&aff, &dic)
            .map_err(|e| anyhow::anyhow!("dictionary: {e}"))?;
        Ok(Speller {
            dict,
            extra: HashSet::new(),
        })
    }

    /// Words the play knows besides the dictionary.
    pub fn set_extra<I: IntoIterator<Item = String>>(&mut self, words: I) {
        self.extra = words
            .into_iter()
            .map(|w| crate::cast::lower_tr(&w))
            .collect();
    }

    pub fn check(&self, word: &str) -> bool {
        if word.chars().any(|c| c.is_ascii_digit()) || word.chars().count() < 2 {
            return true;
        }
        let lower = crate::cast::lower_tr(word);
        if self.extra.contains(&lower) {
            return true;
        }
        // "Selin'in": the name is what matters; Turkish suffixes after an
        // apostrophe are checked with the name the dictionary may not know.
        if let Some((stem, _)) = word.split_once(['\'', '’'])
            && (self.extra.contains(&crate::cast::lower_tr(stem)) || self.dict.check(stem))
        {
            return true;
        }
        self.dict.check(word) || self.dict.check(&lower) || self.dict.check(&title(&lower))
    }

    pub fn suggest(&self, word: &str) -> Vec<String> {
        let mut out = Vec::new();
        self.dict.suggest(word, &mut out);
        out.truncate(8);
        out
    }
}

/// Hunspell accepts a flag numbered 0 in `FLAG num` dictionaries (the
/// Turkish one uses it); spellbook doesn't. Renumber it past the highest.
fn without_zero_flag(aff: String, dic: String) -> (String, String) {
    if !aff.lines().any(|l| l.trim() == "FLAG num") {
        return (aff, dic);
    }
    let max = aff
        .lines()
        .filter(|l| l.starts_with("SFX ") || l.starts_with("PFX "))
        .filter_map(|l| l.split_whitespace().nth(1)?.parse::<u32>().ok())
        .max()
        .unwrap_or(0);
    let new = (max + 1).to_string();
    let fix_list = |list: &str| -> String {
        list.split(',')
            .map(|f| if f == "0" { new.as_str() } else { f })
            .collect::<Vec<_>>()
            .join(",")
    };
    let mut aff_out = String::with_capacity(aff.len());
    for line in aff.lines() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        let fixed = if (fields.first() == Some(&"SFX") || fields.first() == Some(&"PFX"))
            && fields.len() >= 4
        {
            let mut f: Vec<String> = fields.iter().map(|x| x.to_string()).collect();
            if f[1] == "0" {
                f[1] = new.clone();
            }
            // A rule's continuation flags: "add/12,0".
            if f.len() >= 5
                && let Some((add, flags)) = f[3].split_once('/')
            {
                f[3] = format!("{add}/{}", fix_list(flags));
            }
            f.join(" ")
        } else {
            line.to_string()
        };
        aff_out.push_str(&fixed);
        aff_out.push('\n');
    }
    let mut dic_out = String::with_capacity(dic.len() + 1024);
    for line in dic.lines() {
        match line.split_once('/') {
            Some((word, rest)) => {
                let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
                dic_out.push_str(word);
                dic_out.push('/');
                dic_out.push_str(&fix_list(&rest[..end]));
                dic_out.push_str(&rest[end..]);
            }
            None => dic_out.push_str(line),
        }
        dic_out.push('\n');
    }
    (aff_out, dic_out)
}

fn title(lower: &str) -> String {
    crate::cast::title_case(lower)
}

/// Names a play's characters bring: every word of their names.
pub fn cast_words(cast: &Cast) -> Vec<String> {
    let mut out = Vec::new();
    for c in &cast.characters {
        for name in [Some(c.script.as_str()), c.full.as_deref()]
            .into_iter()
            .flatten()
        {
            out.extend(
                name.split(|ch: char| !ch.is_alphabetic())
                    .filter(|w| !w.is_empty())
                    .map(str::to_string),
            );
        }
    }
    for i in &cast.inline {
        out.extend(
            i.name
                .split(|ch: char| !ch.is_alphabetic())
                .filter(|w| !w.is_empty())
                .map(str::to_string),
        );
    }
    out
}

/// The words to check in a block: byte ranges of letters (with inner
/// apostrophes), from its text only.
pub fn words(text: &str, doc: &Doc, block: usize) -> Vec<Range<usize>> {
    let b = &doc.blocks[block];
    let (range, dialogue) = match &b.kind {
        Kind::Line { text: t, .. } | Kind::Do { text: t, .. } => (t.clone(), true),
        Kind::Action { text: t } | Kind::Stage { text: t, .. } => (t.clone(), false),
        _ => return Vec::new(),
    };
    let mut out = Vec::new();
    collect(text, range, &doc.cast, dialogue, &mut out);
    out
}

fn collect(
    text: &str,
    range: Range<usize>,
    cast: &Cast,
    dialogue: bool,
    out: &mut Vec<Range<usize>>,
) {
    for p in lex::inline(text, range, cast, dialogue) {
        match &p.kind {
            Inline::Text => split_words(text, p.span.clone(), out),
            Inline::Ic { inner } | Inline::Under { inner } => {
                collect(text, inner.clone(), cast, dialogue, out)
            }
            Inline::Wrap { name, inner }
                if matches!(name.as_str(), "textit" | "emph" | "textbf") =>
            {
                collect(text, inner.clone(), cast, dialogue, out)
            }
            _ => {}
        }
    }
}

fn split_words(text: &str, range: Range<usize>, out: &mut Vec<Range<usize>>) {
    let s = &text[range.clone()];
    let mut start: Option<usize> = None;
    let chars: Vec<(usize, char)> = s.char_indices().collect();
    for (k, &(i, c)) in chars.iter().enumerate() {
        let inner_apostrophe = matches!(c, '\'' | '’')
            && start.is_some()
            && chars.get(k + 1).is_some_and(|&(_, n)| n.is_alphabetic());
        if c.is_alphabetic() || inner_apostrophe {
            start.get_or_insert(i);
        } else if let Some(st) = start.take() {
            out.push(range.start + st..range.start + i);
        }
    }
    if let Some(st) = start {
        out.push(range.start + st..range.end);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLAY: &str = include_str!("../examples/ornek-oyun.tex");

    #[test]
    fn words_of_a_line() {
        let doc = Doc::new(PLAY);
        let at = PLAY.find("\\oz Durdu").unwrap();
        let b = doc.block_at(at).unwrap();
        let got: Vec<&str> = words(PLAY, &doc, b).into_iter().map(|r| &PLAY[r]).collect();
        assert_eq!(
            got,
            vec![
                "Durdu", "kızım", "elini", "taşa", "koyar", "Soğuk", "her", "şeyi", "durdurur"
            ]
        );
        // Russian and English passages and names are left out.
        let at = PLAY.find("\\se \\ru").unwrap();
        let b = doc.block_at(at).unwrap();
        let got: Vec<&str> = words(PLAY, &doc, b).into_iter().map(|r| &PLAY[r]).collect();
        assert_eq!(got, vec!["dedi", "yolcu"]);
        let t = "Selin'in evi";
        let mut out = Vec::new();
        split_words(t, 0..t.len(), &mut out);
        assert_eq!(
            out.iter().map(|r| &t[r.clone()]).collect::<Vec<_>>(),
            vec!["Selin'in", "evi"]
        );
    }

    #[test]
    fn names_are_words() {
        let doc = Doc::new(PLAY);
        let w = cast_words(&doc.cast);
        assert!(w.contains(&"SELİN".to_string()) && w.contains(&"Usta".to_string()));
    }

    /// With a dictionary at hand: DACTILE_DICT=dir cargo test spell -- --ignored
    #[test]
    #[ignore]
    fn turkish_dictionary() {
        let (aff, dic) = find().expect("no tr_TR dictionary");
        let t = std::time::Instant::now();
        let mut s = Speller::load(&aff, &dic).unwrap();
        println!("loaded in {:?}", t.elapsed());
        s.set_extra(["Selin".to_string()]);
        for w in [
            "merhaba",
            "kitaplarımızdan",
            "İstanbul'da",
            "Selin'in",
            "Gidiyorum",
            "SELİN",
        ] {
            assert!(s.check(w), "{w}");
        }
        for w in ["kalemm", "ğsdk"] {
            assert!(!s.check(w), "{w}");
        }
        let t = std::time::Instant::now();
        println!(
            "suggest kalemm: {:?} in {:?}",
            s.suggest("kalemm"),
            t.elapsed()
        );
    }
}
