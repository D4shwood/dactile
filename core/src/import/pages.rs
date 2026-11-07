//! A play in a book: the cover, the imprint, the contents, a foreword, the
//! author's life, the cast page, and what comes after the play. None of it
//! is dialogue, though `Editör: …` looks like it, so the play's start and
//! end are found first, by where people answer each other, and only that
//! part is read for who speaks. The rest waits in `Aside`s, to be shown in
//! the preview. There every paragraph's label can be changed, and the play
//! read again.

use super::shape::{
    Level, NOT_NAMES, cast_entry, fold, heading, is_cast_heading, key, name_alone, speech_head,
};
use super::{Para, Run, Source};
use crate::cast::upper_tr;
use std::collections::{HashMap, HashSet};
use std::ops::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Play,
    Cover,
    Imprint,
    Contents,
    Preface,
    Notes,
    Author,
    Cast,
    /// Those under the cast list with no names: "soldiers, musicians".
    Others,
    Back,
    Footnotes,
    Other,
}

impl Page {
    pub const ALL: [Page; 12] = [
        Page::Play,
        Page::Cover,
        Page::Imprint,
        Page::Contents,
        Page::Preface,
        Page::Notes,
        Page::Author,
        Page::Cast,
        Page::Others,
        Page::Back,
        Page::Footnotes,
        Page::Other,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Page::Play => "play",
            Page::Cover => "cover",
            Page::Imprint => "imprint",
            Page::Contents => "contents",
            Page::Preface => "preface",
            Page::Notes => "notes",
            Page::Author => "author",
            Page::Cast => "cast",
            Page::Others => "others",
            Page::Back => "back",
            Page::Footnotes => "footnotes",
            Page::Other => "other",
        }
    }

    pub fn named(name: &str) -> Option<Page> {
        Page::ALL.into_iter().find(|p| p.name() == name)
    }

    /// Written into the play: the play's own sections.
    pub fn kept(self) -> bool {
        matches!(self, Page::Preface | Page::Notes)
    }
}

/// Paragraphs in a row of one page of the book.
#[derive(Debug)]
pub(super) struct Part {
    pub page: Page,
    pub range: Range<usize>,
    /// Before the play, or after it.
    pub after: bool,
}

/// The labels' runs, the play's left out.
pub(super) fn parts(labels: &[Page]) -> Vec<Part> {
    let last = labels.iter().rposition(|&p| p == Page::Play);
    let mut out: Vec<Part> = Vec::new();
    for (i, &page) in labels.iter().enumerate() {
        match out.last_mut() {
            _ if page == Page::Play => {}
            Some(p) if p.page == page && p.range.end == i => p.range.end = i + 1,
            _ => out.push(Part {
                page,
                range: i..i + 1,
                after: last.is_some_and(|l| i > l),
            }),
        }
    }
    out
}

/// Paragraphs to look ahead in for people answering each other.
const WINDOW: usize = 25;
/// Paragraphs back from the first line to look for its act and scene.
const BACK: usize = 12;
/// Untitled paragraphs just before the first line that set the scene.
const OPENING: usize = 3;
/// Paragraphs of another label too few to be a part.
const SPECK: usize = 2;
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
pub(super) fn page_title(t: &str) -> Option<Page> {
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

/// What every paragraph is. The footnotes come last, by themselves. A
/// document with no people to be sure of is all play, as it was given.
pub fn labels(src: &Source) -> Vec<Page> {
    let n = src
        .paras
        .iter()
        .position(|p| p.foot)
        .unwrap_or(src.paras.len());
    let paras = &src.paras[..n];
    let from = src
        .body_page
        .and_then(|b| paras.iter().position(|p| p.page.is_some_and(|at| at >= b)));
    let mut out = find(paras, from);
    smooth(&mut out);
    out.resize(src.paras.len(), Page::Footnotes);
    out
}

/// A line or two that looked otherwise in the middle of a page: `Tel:` in
/// a letter, a publisher's name in an essay. Parts stay few to go through.
fn smooth(labels: &mut [Page]) {
    let weak = |p: Page| matches!(p, Page::Imprint | Page::Other | Page::Cast);
    let mut i = 0;
    while i < labels.len() {
        let end = (i..labels.len())
            .find(|&j| labels[j] != labels[i])
            .unwrap_or(labels.len());
        if i > 0
            && end < labels.len()
            && end - i <= SPECK
            && labels[i - 1] == labels[end]
            && weak(labels[i])
            && weak(labels[end])
        {
            let around = labels[end];
            labels[i..end].fill(around);
            i = 0;
            continue;
        }
        i = end;
    }
}

/// `from`: where a book's numbering says the text starts.
fn find(paras: &[Para], from: Option<usize>) -> Vec<Page> {
    let all = vec![Page::Play; paras.len()];
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
    let Some(first) = (from.unwrap_or(0)..paras.len()).find(|&i| opens(i)) else {
        return all;
    };
    let last = (first..paras.len())
        .rev()
        .find(|&i| speaker(i).is_some())
        .unwrap_or(first);

    let mut start = opening_heading(&texts, first);
    if let Some(at) = from.filter(|&at| first - at <= WINDOW) {
        start = at;
    }
    let mut pages = classify(&texts, 0..start, Page::Cover, &count);
    if from.is_none() {
        start = with_opening(&texts, &pages, start);
        pages.truncate(start);
    }

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
    pages.resize(end, Page::Play);
    pages.extend(classify(&texts, end..paras.len(), Page::Back, &count));
    pages
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
            // Under the list, those it doesn't name.
            Page::Cast if others_line(t, count) => {
                state = Page::Other;
                Page::Others
            }
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
/// "Köylüler, çobanlar, konuklar.": a sentence of people no one calls by
/// name, the first of them not speaking.
fn others_line(t: &str, count: &HashMap<&str, usize>) -> bool {
    let Some((first, _)) = t.split_once(',') else {
        return false;
    };
    let k = key(first);
    t.trim_end().ends_with('.')
        && first.chars().any(char::is_lowercase)
        && !imprint(t)
        && t.split_whitespace().count() <= 40
        && !count.contains_key(k.as_str())
}

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

/// The marks in a run: where each starts and ends, `* *` being one.
fn marks(text: &str) -> Vec<Range<usize>> {
    let sign = |c: char| c == '*' || c == '•';
    let mut out: Vec<Range<usize>> = Vec::new();
    for (i, c) in text.char_indices().filter(|&(_, c)| sign(c)) {
        match out.last_mut() {
            Some(m) if text[m.end..i].trim().is_empty() => m.end = i + c.len_utf8(),
            _ => out.push(i..i + c.len_utf8()),
        }
    }
    out
}

/// Each page's notes put where its text marks them, in order; those over,
/// at the end of the page's last paragraph. What was placed.
pub(super) fn attach_notes<'a>(paras: &mut [Para], notes: &[&'a Para]) -> Vec<&'a Para> {
    let mut placed = Vec::new();
    let mut at = 0;
    while at < notes.len() {
        let page = notes[at].page;
        let end = (at..notes.len())
            .find(|&j| notes[j].page != page)
            .unwrap_or(notes.len());
        let here: Vec<usize> = (0..paras.len())
            .filter(|&i| page.is_some() && paras[i].page == page)
            .collect();
        let mut left = notes[at..end].iter();
        for &i in &here {
            let mut runs = Vec::new();
            for r in std::mem::take(&mut paras[i].runs) {
                let mut from = 0;
                for m in marks(&r.text) {
                    let Some(note) = left.next() else { break };
                    runs.push(Run {
                        text: r.text[from..m.start].trim_end().to_string(),
                        ..r.clone()
                    });
                    runs.push(Run {
                        text: note.text(),
                        note: true,
                        ..Run::default()
                    });
                    placed.push(*note);
                    from = m.end;
                }
                runs.push(Run {
                    text: r.text[from..].to_string(),
                    ..r.clone()
                });
            }
            runs.retain(|r| !r.text.is_empty());
            paras[i].runs = runs;
        }
        if let Some(&last) = here.last() {
            for note in left {
                paras[last].runs.push(Run {
                    text: note.text(),
                    note: true,
                    ..Run::default()
                });
                placed.push(*note);
            }
        }
        at = end;
    }
    placed
}

/// A play of a volume of several: its title, where its title page starts,
/// and the first paragraph after that page.
#[derive(Debug, Clone, PartialEq)]
pub struct Volume {
    pub title: String,
    pub start: usize,
    pub from: usize,
    /// The contents list it; else it is a guess from the pages.
    pub sure: bool,
}

/// As many speeches as show that what follows a title page is a play.
const SPOKEN: usize = 20;

/// The plays of a book of several: pages of a title or so, the title being
/// in the contents, each with people speaking after it. A book without
/// contents that show them is read for the same pages by who speaks.
pub fn volume(paras: &[Para]) -> Vec<Volume> {
    let listed = by_contents(paras);
    if listed.is_empty() {
        by_speakers(paras)
    } else {
        listed
    }
}

fn by_contents(paras: &[Para]) -> Vec<Volume> {
    let Some(contents) = paras
        .iter()
        .find(|p| page_title(&p.text()) == Some(Page::Contents))
        .and_then(|p| p.page)
    else {
        return Vec::new();
    };
    let listed: String = paras
        .iter()
        .filter(|p| p.page == Some(contents))
        .map(|p| fold(&p.text()))
        .collect();
    let mut found: Vec<Volume> = Vec::new();
    let mut at = 0;
    while at < paras.len() {
        let page = paras[at].page;
        let end = (at..paras.len())
            .find(|&j| paras[j].page != page)
            .unwrap_or(paras.len());
        let title = first_line(&paras[at].text()).trim().to_string();
        let folded = fold(&title);
        if page.is_some()
            && page != Some(contents)
            && end - at <= 2
            && !paras[at].foot
            && folded.chars().count() >= 3
            && title.split_whitespace().count() <= 10
            && listed.contains(&folded)
        {
            found.push(Volume {
                title,
                start: at,
                from: end,
                sure: true,
            });
        }
        at = end;
    }
    // What the contents also list, a chronology, has no one speaking.
    let bound: Vec<usize> = found
        .iter()
        .skip(1)
        .map(|v| v.from)
        .chain([paras.len()])
        .collect();
    let plays: Vec<Volume> = found
        .into_iter()
        .zip(bound)
        .filter(|(v, end)| {
            paras[v.from..*end]
                .iter()
                .filter(|p| speech_head(&p.text()).is_some())
                .count()
                >= SPOKEN
        })
        .map(|(v, _)| v)
        .collect();
    if plays.len() >= 2 { plays } else { Vec::new() }
}

/// Whether a page's first line could be the title of a play.
fn title_like(title: &str, whole: &str) -> bool {
    title.chars().filter(|c| c.is_alphabetic()).count() >= 3
        && title.split_whitespace().count() <= 10
        && !title.ends_with(['.', ',', ';', ':', '!', '?'])
        && !title.starts_with(['(', '['])
        && page_title(whole).is_none()
        && heading(title).is_none()
        && speech_head(title).is_none()
}

/// Who speaks in these paragraphs.
fn speakers(paras: &[Para]) -> HashSet<String> {
    paras
        .iter()
        .filter_map(|p| speech_head(&p.text()).map(|h| fold(&h.0)))
        .collect()
}

/// Pages of a line or so with people speaking after them, where those who
/// speak change: the same people going on are one play with a page of a
/// quotation in it, not two.
fn by_speakers(paras: &[Para]) -> Vec<Volume> {
    let mut pages: Vec<Volume> = Vec::new();
    let mut at = 0;
    while at < paras.len() {
        let page = paras[at].page;
        let end = (at..paras.len())
            .find(|&j| paras[j].page != page)
            .unwrap_or(paras.len());
        let whole = paras[at].text();
        let title = first_line(&whole).trim().to_string();
        if page.is_some() && end - at <= 2 && !paras[at].foot && title_like(&title, &whole) {
            pages.push(Volume {
                title,
                start: at,
                from: end,
                sure: false,
            });
        }
        at = end;
    }
    let bound: Vec<usize> = pages
        .iter()
        .skip(1)
        .map(|v| v.start)
        .chain([paras.len()])
        .collect();
    let mut plays: Vec<(Volume, HashSet<String>)> = Vec::new();
    for (v, end) in pages.into_iter().zip(bound) {
        let part = &paras[v.from..end];
        let spoken = part
            .iter()
            .filter(|p| speech_head(&p.text()).is_some())
            .count();
        if spoken < SPOKEN {
            continue;
        }
        let who = speakers(part);
        match plays.last_mut() {
            Some((_, seen)) if seen.intersection(&who).count() * 2 >= seen.len().min(who.len()) => {
                seen.extend(who)
            }
            _ => plays.push((v, who)),
        }
    }
    if plays.len() >= 2 {
        plays.into_iter().map(|(v, _)| v).collect()
    } else {
        Vec::new()
    }
}

/// The paragraphs of one play of a volume, from after its title page.
pub fn play_range(src: &Source, at: usize) -> Range<usize> {
    let body = src
        .paras
        .iter()
        .position(|p| p.foot)
        .unwrap_or(src.paras.len());
    let to = src.plays.get(at + 1).map_or(body, |next| next.start);
    src.plays[at].from..to
}

/// What each paragraph is with one play of a volume in view: the rest is
/// the book's.
pub fn labels_of_play(src: &Source, at: usize) -> Vec<Page> {
    let Range {
        start: from,
        end: to,
    } = play_range(src, at);
    let body = src
        .paras
        .iter()
        .position(|p| p.foot)
        .unwrap_or(src.paras.len());
    let pages = src.paras[from].page..=src.paras[to.saturating_sub(1)].page;
    let mut taken: Vec<usize> = (from..to).collect();
    taken.extend((body..src.paras.len()).filter(|&i| pages.contains(&src.paras[i].page)));
    let sub = Source {
        paras: taken.iter().map(|&i| src.paras[i].clone()).collect(),
        body_page: None,
        plays: Vec::new(),
        ..src.clone()
    };
    let mut out = vec![Page::Other; src.paras.len()];
    for (i, l) in taken.into_iter().zip(labels(&sub)) {
        out[i] = l;
    }
    out
}

/// A cast list with who first played the parts in a column beside it, or
/// wrapped under it. Those are actors' names, not the people of the play.
pub(super) fn premiere(list: &[Para]) -> bool {
    let actor = |t: &str| {
        let words: Vec<&str> = t.split_whitespace().collect();
        (1..=3).contains(&words.len())
            && words.iter().all(|w| {
                w.chars().next().is_some_and(char::is_uppercase)
                    && w.chars().any(char::is_lowercase)
            })
    };
    let lines: Vec<String> = list
        .iter()
        .flat_map(|p| p.text().lines().map(str::to_string).collect::<Vec<_>>())
        .collect();
    let actors = lines
        .iter()
        .filter(|l| l.rsplit("   ").next().is_some_and(actor))
        .count();
    actors >= 5 && actors * 3 >= lines.len()
}
