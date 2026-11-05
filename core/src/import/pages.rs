//! A play in a book: the cover, the imprint, the contents, a foreword, the
//! author's life, the cast page, and what comes after the play. None of it
//! is dialogue, though `Editör: …` looks like it, so the play's start and
//! end are found first, by where people answer each other, and only that
//! part is read for who speaks. The rest waits in `Aside`s, to be shown in
//! the preview and taken in if it was the play after all.

use super::Para;
use super::shape::{
    Level, NOT_NAMES, cast_entry, heading, is_cast_heading, key, name_alone, speech_head,
};
use crate::cast::upper_tr;
use std::collections::HashMap;
use std::ops::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Cover,
    Imprint,
    Contents,
    Preface,
    Notes,
    Author,
    Cast,
    Back,
    Other,
}

/// Paragraphs in a row that a person says the same thing about.
#[derive(Debug)]
pub(super) struct Part {
    pub page: Page,
    pub range: Range<usize>,
    /// Before the play, or after it.
    pub after: bool,
}

#[derive(Debug)]
pub(super) struct Layout {
    pub play: Range<usize>,
    pub parts: Vec<Part>,
}

/// Paragraphs to look ahead in for people answering each other.
const WINDOW: usize = 25;
/// Paragraphs back from the first line to look for its act and scene.
const BACK: usize = 12;
/// Untitled paragraphs just before the first line that set the scene.
const OPENING: usize = 3;
/// As long as a line of an imprint or a title gets.
const SHORT: usize = 14;

/// Words of an imprint: as they start, and as they are.
const PRESS_STEMS: [&str; 22] = [
    "YAYIN",
    "MATBAA",
    "BASIMEV",
    "SERTİFİKA",
    "EDİTÖR",
    "REDAKSİYON",
    "TASHİH",
    "DÜZELTİ",
    "DİZGİ",
    "ÇEVİR",
    "TÜRKÇESİ",
    "TELİF",
    "TELEFON",
    "GRAFİK",
    "PUBLISH",
    "TRANSLAT",
    "COPYRIGHT",
    "PRINTED",
    "EDITOR",
    "EDITION",
    "ISBN",
    "ISSN",
];
const PRESS_WORDS: [&str; 10] = [
    "BASKI",
    "BASKISI",
    "BASIM",
    "BASIMI",
    "TEL",
    "FAKS",
    "FAX",
    "ADRES",
    "PRESS",
    "DİZİSİ",
];
const PRESS_PHRASES: [&str; 5] = [
    "GENEL YAYIN",
    "KAPAK TASARIM",
    "KAPAK RESM",
    "KAPAK RESİM",
    "DİZİ ",
];
/// Who made a staging, listed by the cast: a job, not a part.
const CREDITS: [&str; 10] = [
    "YÖNET",
    "REJİ",
    "DEKOR",
    "KOSTÜM",
    "MÜZİK",
    "IŞIK",
    "KOREOGRAF",
    "SAHNE",
    "DIRECTOR",
    "DESIGN",
];

fn words(up: &str) -> impl Iterator<Item = &str> {
    up.split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
}

fn press(up: &str) -> bool {
    PRESS_PHRASES
        .iter()
        .any(|p| up.starts_with(p) || up.contains(&format!(" {p}")))
        || words(up)
            .any(|w| PRESS_WORDS.contains(&w) || PRESS_STEMS.iter().any(|s| w.starts_with(s)))
}

/// A name that is a job in a book or a staging. A play may still have
/// such a part, and then it speaks often.
pub(super) fn book_word(name: &str) -> bool {
    let up = upper_tr(name);
    press(&up) || words(&up).any(|w| CREDITS.iter().any(|s| w.starts_with(s)))
}

/// A line of an imprint: `Genel Yayın: 2408`, `ISBN …`, `© 2012`, a
/// publisher, a printer, an address on the web.
pub(super) fn imprint(t: &str) -> bool {
    let t = t.trim();
    if t.split_whitespace().count() > SHORT {
        return false;
    }
    let up = upper_tr(t);
    up.contains('©')
        || up.contains("WWW.")
        || up.contains("HTTP")
        || t.contains('@')
        || press(&up)
        || t.split_once(':').is_some_and(|(k, _)| book_word(k))
}

/// The title of a page of the book.
fn page_title(t: &str) -> Option<Page> {
    let whole = t.trim();
    let t = first_line(whole);
    // A PDF's title may have the page's next short lines under it.
    if whole.contains('\n') && t.chars().any(char::is_lowercase) {
        return None;
    }
    if t.split_whitespace().count() > 8 || t.ends_with(['.', '!', '?']) || contents_line(t) {
        return None;
    }
    let up = upper_tr(t.trim_end_matches(':'));
    let any = |list: &[&str]| list.iter().any(|w| up.contains(w));
    if is_cast_heading(t) {
        return Some(Page::Cast);
    }
    if let Some((Level::Preface, _)) = heading(t) {
        return Some(Page::Preface);
    }
    if let Some((Level::Notes, _)) = heading(t) {
        return Some(Page::Notes);
    }
    if matches!(
        up.as_str(),
        "İÇİNDEKİLER" | "CONTENTS" | "TABLE OF CONTENTS"
    ) {
        return Some(Page::Contents);
    }
    if matches!(
        up.as_str(),
        "ÖN SÖZ" | "SUNU" | "TAKDİM" | "BAŞLARKEN" | "GİRİŞ" | "FOREWORD" | "INTRODUCTION"
    ) || any(&[
        "ÖNSÖZ",
        "ÇEVİRENİN NOTU",
        "ÇEVİRMENİN NOTU",
        "YAYINCININ NOTU",
        "EDİTÖRÜN NOTU",
    ]) {
        return Some(Page::Preface);
    }
    if any(&[
        "HAKKINDA",
        "ÖZGEÇMİŞ",
        "HAYATI",
        "ABOUT THE AUTHOR",
        "BIOGRAPHY",
    ]) {
        return Some(Page::Author);
    }
    if any(&[
        "AYNI YAZARIN",
        "YAZARIN DİĞER",
        "YAZARIN ÖTEKİ",
        "KİTAPLARIMIZ",
        "SONSÖZ",
        "SON SÖZ",
        "KAYNAKÇA",
        "AFTERWORD",
        "BIBLIOGRAPHY",
        "GLOSSARY",
    ]) || up == "SÖZLÜK"
    {
        return Some(Page::Back);
    }
    None
}

/// A line of a table of contents: `Birinci Perde ........ 9`.
fn contents_line(t: &str) -> bool {
    let t = t.trim();
    !t.contains('\n') && (t.contains("...") || t.contains("…") || t.contains(". ."))
        || t.split_whitespace().count() <= 10 && t.ends_with(|c: char| c.is_ascii_digit())
}

/// The word a play ends on.
fn end_mark(t: &str) -> bool {
    let k = key(t.trim_end_matches(['.', '!']));
    NOT_NAMES.contains(&k.as_str())
        || matches!(k.as_str(), "OYUNUN SONU" | "PERDE İNER" | "PERDE İNİYOR")
}

/// Who a paragraph has speaking, if it looks so.
fn spoken(t: &str) -> Option<String> {
    if t.is_empty()
        || heading(t).is_some()
        || is_cast_heading(t)
        || imprint(t)
        || page_title(t).is_some()
    {
        return None;
    }
    let name = speech_head(t)
        .map(|(name, ..)| name)
        .or_else(|| name_alone(t).map(|(name, _)| name))?;
    let k = key(&name);
    (!NOT_NAMES.contains(&k.as_str()) && !book_word(&k)).then_some(k)
}

/// The entries under a cast list's title: each name once, so a name again
/// is the play begun.
fn listed(texts: &[String]) -> Vec<usize> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < texts.len() {
        i += 1;
        if !is_cast_heading(&texts[i - 1]) {
            continue;
        }
        let mut seen = Vec::new();
        while let Some(t) = texts.get(i)
            && let Some((name, _)) = cast_entry(first_line(t))
            && heading(t).is_none()
            && !seen.contains(&key(&name))
        {
            seen.push(key(&name));
            out.push(i);
            i += 1;
        }
    }
    out
}

/// What a cast page's line looks like beside a line of dialogue: no
/// sentence after the name, or a name with another under it.
fn list_line(t: &str, name_next: bool) -> bool {
    if name_alone(t).is_some() {
        return name_next;
    }
    cast_entry(t).is_some() && !t.ends_with(['.', '?', '!', '…'])
}

/// Where the play is in the paragraphs. A document with no people to be
/// sure of is all play, as it was given.
pub(super) fn find(paras: &[Para]) -> Layout {
    let all = Layout {
        play: 0..paras.len(),
        parts: Vec::new(),
    };
    let texts: Vec<String> = paras.iter().map(|p| p.text().trim().to_string()).collect();
    let mut names: Vec<Option<String>> = texts.iter().map(|t| spoken(t)).collect();
    for i in listed(&texts) {
        names[i] = None;
    }
    let mut count: HashMap<&str, usize> = HashMap::new();
    for n in names.iter().flatten() {
        *count.entry(n).or_default() += 1;
    }
    let speaker = |i: usize| {
        names[i]
            .as_deref()
            .filter(|n| count.get(n).is_some_and(|&c| c >= 3))
    };
    // People answer each other: someone speaks again soon. A cast page has
    // the same names, once each.
    let answered = |i: usize| {
        let mut here: HashMap<&str, usize> = HashMap::new();
        for j in i..paras.len().min(i + WINDOW) {
            if let Some(n) = speaker(j) {
                *here.entry(n).or_default() += 1;
            }
        }
        here.values().sum::<usize>() >= 3 && here.values().any(|&c| c >= 2)
    };
    // And at once: a foreword may quote a line, not a conversation.
    let close = |i: usize| {
        (i + 1..paras.len().min(i + 5))
            .filter(|&j| speaker(j).is_some())
            .count()
            >= 2
    };
    let opens = |i: usize| {
        speaker(i).is_some()
            && close(i)
            && answered(i)
            && !list_line(&texts[i], names.get(i + 1).is_some_and(Option::is_some))
    };
    let Some(first) = (0..paras.len()).find(|&i| opens(i)) else {
        return all;
    };
    let last = (first..paras.len())
        .rev()
        .find(|&i| speaker(i).is_some())
        .unwrap_or(first);

    let start = opening_heading(&texts, first);
    let mut pages = front_pages(&texts, start, &count);
    let start = with_opening(&texts, &pages, start);
    pages.truncate(start);

    // After the last line: words that end the play, or a page of the book.
    let mut end = paras.len();
    for (k, t) in texts.iter().enumerate().skip(last + 1) {
        if end_mark(t) {
            end = k + 1;
            break;
        }
        // `EDİTÖR: …` said often is a part in the play.
        let part = speech_head(t).is_some_and(|(name, ..)| {
            let k = key(&name);
            texts
                .iter()
                .filter(|t| speech_head(t).is_some_and(|(n, ..)| key(&n) == k))
                .count()
                >= 3
        });
        if page_title(t).is_some() || imprint(t) && !part {
            end = k;
            break;
        }
    }
    let back = classify(&texts, end..paras.len(), Page::Back, &count);

    let mut parts = runs(&pages, 0, false);
    parts.extend(runs(&back, end, true));
    Layout {
        play: start..end,
        parts,
    }
}

/// The act and scene over the first line, a few paragraphs up.
fn opening_heading(texts: &[String], first: usize) -> usize {
    let mut start = first;
    let mut got = None;
    for j in (first.saturating_sub(BACK)..first).rev() {
        let t = &texts[j];
        if page_title(t).is_some() || contents_line(t) {
            break;
        }
        match heading(t) {
            Some((Level::Act, _)) if got == Some(Level::Act) => break,
            Some((Level::Scene, _)) if got.is_some() => break,
            Some((level, _)) => {
                got = Some(level);
                start = j;
            }
            None => {}
        }
    }
    start
}

fn front_pages(texts: &[String], start: usize, count: &HashMap<&str, usize>) -> Vec<Page> {
    classify(texts, 0..start, Page::Cover, count)
}

/// What each paragraph of `range` is, going on from `from`.
fn classify(
    texts: &[String],
    range: Range<usize>,
    from: Page,
    count: &HashMap<&str, usize>,
) -> Vec<Page> {
    let mut state = from;
    let mut out = Vec::with_capacity(range.len());
    for t in &texts[range] {
        if let Some(p) = page_title(t) {
            state = p;
            out.push(p);
            continue;
        }
        let prose = t.split_whitespace().count() > SHORT;
        let page = match state {
            Page::Contents if contents_line(t) => Page::Contents,
            Page::Cast if cast_line(t, count) => Page::Cast,
            // Past the list, past the contents, and past the cover at the
            // first paragraph that isn't a line.
            Page::Contents | Page::Cast => {
                state = Page::Other;
                Page::Other
            }
            Page::Cover if prose => {
                state = Page::Other;
                Page::Other
            }
            Page::Preface | Page::Notes | Page::Author => state,
            _ if imprint(t) => Page::Imprint,
            // A cast page with no title, its names the play's speakers.
            _ if known_name(t, count) => {
                state = Page::Cast;
                Page::Cast
            }
            _ => state,
        };
        out.push(page);
    }
    out
}

/// A PDF's short lines come as one paragraph, a line break between them.
fn first_line(t: &str) -> &str {
    t.lines().next().unwrap_or("").trim()
}

/// An entry of a cast list that isn't someone who staged the play.
fn cast_line(t: &str, count: &HashMap<&str, usize>) -> bool {
    cast_entry(first_line(t)).is_some_and(|(name, _)| {
        let k = key(&name);
        !book_word(&k) || count.get(k.as_str()).is_some_and(|&c| c >= 3)
    })
}

fn known_name(t: &str, count: &HashMap<&str, usize>) -> bool {
    cast_entry(first_line(t)).is_some_and(|(name, _)| {
        let k = key(&name);
        count.get(k.as_str()).is_some_and(|&c| c >= 3) && !book_word(&k)
    })
}

/// The start, moved up over the few untitled lines that set the scene.
fn with_opening(texts: &[String], pages: &[Page], start: usize) -> usize {
    let mut at = start;
    while at > 2
        && start - at < OPENING
        && matches!(pages[at - 1], Page::Other | Page::Cover)
        && page_title(&texts[at - 1]).is_none()
        && !imprint(first_line(&texts[at - 1]))
    {
        at -= 1;
    }
    at
}

/// Paragraphs of one page in a row, as parts.
fn runs(pages: &[Page], base: usize, after: bool) -> Vec<Part> {
    let mut out: Vec<Part> = Vec::new();
    for (i, &page) in pages.iter().enumerate() {
        match out.last_mut() {
            Some(p) if p.page == page => p.range.end = base + i + 1,
            _ => out.push(Part {
                page,
                range: base + i..base + i + 1,
                after,
            }),
        }
    }
    out
}
