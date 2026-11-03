//! Paragraphs into a play: the title and author, a cast list, act and
//! scene headings, lines of dialogue (`NAME: …`, `NAME — …`, `NAME. …`, or
//! the name on a line of its own above the words), directions (in
//! parentheses, or all in italics), and the rest as it is. The names found
//! wait in a `Draft` to be checked (renamed, merged, or not a character
//! after all) before the play is written out.

use super::{Para, Run, Source};
use crate::buffer::escape_pasted;
use crate::cast::{Cast, upper_tr};
use crate::lang::Lang;
use crate::writing::is_caps_name;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Person {
    /// As the play writes it over their lines: "SELİN".
    pub name: String,
    /// A fuller name from a cast list: "Selin Aydın".
    pub full: Option<String>,
    pub desc: Option<String>,
    pub alias: String,
    pub lines: usize,
    /// The start of their first line.
    pub sample: String,
    /// Caught as a name, but is not one ("NOT: …"): their lines stay text.
    pub not_character: bool,
    /// The same person under another name (index into `people`).
    pub merged_into: Option<usize>,
}

#[derive(Debug, Clone)]
pub enum Item {
    Act(String),
    Scene(String),
    Preface,
    Notes,
    Speech {
        who: usize,
        said: Vec<Run>,
        /// The paragraphs as they were, for a name that is not a character.
        raw: Vec<Run>,
    },
    Direction(Vec<Run>),
    Plain(Vec<Run>),
}

#[derive(Debug, Clone)]
pub struct Draft {
    pub title: String,
    pub author: String,
    pub people: Vec<Person>,
    pub items: Vec<Item>,
    pub lang: Lang,
}

/// What a draft holds, for the preview.
#[derive(Debug, Default, PartialEq)]
pub struct Counts {
    pub lines: usize,
    pub directions: usize,
    pub acts: usize,
    pub scenes: usize,
    pub plain: usize,
}

// ------------------------------------------------------------------ headings

#[derive(Clone, Copy, PartialEq)]
enum Level {
    Act,
    Scene,
    Preface,
    Notes,
}

const ORDINALS_TR: [&str; 12] = [
    "BİRİNCİ",
    "İKİNCİ",
    "ÜÇÜNCÜ",
    "DÖRDÜNCÜ",
    "BEŞİNCİ",
    "ALTINCI",
    "YEDİNCİ",
    "SEKİZİNCİ",
    "DOKUZUNCU",
    "ONUNCU",
    "İLK",
    "SON",
];
const NUMBERS_TR: [&str; 10] = [
    "BİR", "İKİ", "ÜÇ", "DÖRT", "BEŞ", "ALTI", "YEDİ", "SEKİZ", "DOKUZ", "ON",
];
const ORDINALS_EN: [&str; 11] = [
    "FIRST", "SECOND", "THIRD", "FOURTH", "FIFTH", "SIXTH", "SEVENTH", "EIGHTH", "NINTH", "TENTH",
    "LAST",
];
const NUMBERS_EN: [&str; 10] = [
    "ONE", "TWO", "THREE", "FOUR", "FIVE", "SIX", "SEVEN", "EIGHT", "NINE", "TEN",
];

fn is_number(w: &str) -> bool {
    let w = w.trim_end_matches(['.', ':']);
    !w.is_empty()
        && (w.chars().all(|c| c.is_ascii_digit())
            || w.chars().all(|c| matches!(c, 'I' | 'V' | 'X' | 'L'))
            || NUMBERS_TR.contains(&w)
            || NUMBERS_EN.contains(&w))
}

fn is_ordinal(w: &str) -> bool {
    let w = w.trim_end_matches(['.', ':']);
    ORDINALS_TR.contains(&w) || ORDINALS_EN.contains(&w) || is_number(w)
}

fn level_word(w: &str) -> Option<Level> {
    match w.trim_end_matches(['.', ':', ',']) {
        "PERDE" | "ACT" | "BÖLÜM" | "KISIM" => Some(Level::Act),
        "SAHNE" | "SCENE" | "TABLO" | "MEŞHED" => Some(Level::Scene),
        _ => None,
    }
}

/// `BİRİNCİ PERDE`, `Perde 2: Kar`, `II. Sahne — Ahır`, `ACT ONE`, and
/// `#sahne Ahır`: the level and what follows it (a title, a place).
fn heading(text: &str) -> Option<(Level, String)> {
    let t = text.trim();
    if let Some((word, _, rest)) = crate::lex::hash_line(t) {
        use crate::lex::{Heading, Section};
        return match crate::lex::heading(word)? {
            Heading::Act => Some((Level::Act, rest.trim().to_string())),
            Heading::Scene => Some((Level::Scene, rest.trim().to_string())),
            Heading::Section(Section::Preface) => Some((Level::Preface, String::new())),
            Heading::Section(Section::Notes) => Some((Level::Notes, String::new())),
            Heading::Prologue => None,
        };
    }
    match upper_tr(t.trim_end_matches([':', '.'])).as_str() {
        "ÖNSÖZ" | "PREFACE" | "SUNUŞ" => return Some((Level::Preface, String::new())),
        "NOTLAR"
        | "REJİSÖRE NOTLAR"
        | "YÖNETMENE NOTLAR"
        | "NOTES"
        | "NOTES FOR THE DIRECTOR"
        | "DIRECTOR'S NOTES" => return Some((Level::Notes, String::new())),
        _ => {}
    }
    if t.contains('\n') || t.split_whitespace().count() > 12 {
        return None;
    }
    let words: Vec<&str> = t.split_whitespace().collect();
    let up: Vec<String> = words.iter().map(|w| upper_tr(w)).collect();
    // "BİRİNCİ PERDE …", "2. SAHNE …" or "PERDE 2 …", "SCENE ONE …".
    let (level, used) = match up.as_slice() {
        [a, b, ..] if is_ordinal(a) && level_word(b).is_some() => (level_word(b)?, 2),
        [a, b, ..] if level_word(a).is_some() && is_number(b) => (level_word(a)?, 2),
        [a] if level_word(a).is_some() && a.chars().all(|c| !c.is_lowercase()) => {
            (level_word(a)?, 1)
        }
        _ => return None,
    };
    // The words after it, past a separator; in the original case.
    let mut at = 0;
    for w in words.iter().take(used) {
        at = t[at..].find(w).map_or(at, |i| at + i + w.len());
    }
    let rest = t[at..]
        .trim_start_matches(|c: char| c.is_whitespace() || "-–—:.,".contains(c))
        .trim();
    Some((level, rest.to_string()))
}

/// A few words, no sentence, nothing else: an act's title.
fn act_title(t: &str) -> bool {
    let t = t.trim();
    !t.is_empty()
        && !t.contains('\n')
        && t.split_whitespace().count() <= 6
        && !t.ends_with(['.', '!', '?', ':', ')'])
        && heading(t).is_none()
        && name_alone(t).is_none_or(|_| !is_caps_name(t))
        && speech_head(t).is_none()
}

/// The line that starts a cast list.
fn is_cast_heading(text: &str) -> bool {
    let t = upper_tr(text.trim().trim_end_matches([':', '.']));
    matches!(
        t.as_str(),
        "KİŞİLER"
            | "OYUNUN KİŞİLERİ"
            | "OYUNDAKİ KİŞİLER"
            | "KARAKTERLER"
            | "OYUNCULAR"
            | "ŞAHIS KADROSU"
            | "ŞAHISLAR"
            | "KADRO"
            | "CHARACTERS"
            | "CAST"
            | "CAST OF CHARACTERS"
            | "DRAMATIS PERSONAE"
    )
}

// ------------------------------------------------------------------ names

/// `SELİN`, `Ozan Usta`, `Selin (gülerek)`: a name, and a note in
/// parentheses after it.
fn split_name(s: &str) -> Option<(String, Option<String>)> {
    let s = s.trim().trim_matches(|c| c == '*' || c == '_').trim();
    let (name, note) = match s.find('(') {
        Some(i) if s.ends_with(')') => (s[..i].trim(), Some(s[i + 1..s.len() - 1].trim())),
        _ => (s, None),
    };
    let ok = !name.is_empty()
        && name.chars().count() <= 40
        && name.split_whitespace().count() <= 3
        && name.chars().any(char::is_alphabetic)
        && name
            .chars()
            .all(|c| c.is_alphabetic() || c == ' ' || c == '.' || c == '\'' || c == '’');
    ok.then(|| (name.to_string(), note.map(str::to_string)))
}

/// Where a line of dialogue's name ends and its words start: `NAME:`,
/// `NAME —`, `NAME –`, `NAME -`, `NAME.` The separator and whether only a
/// name in capitals may take it.
fn separators(t: &str) -> Vec<(usize, usize, bool)> {
    let mut out = Vec::new();
    let head: String = t.chars().take(60).collect();
    if let Some(i) = head.find(':') {
        out.push((i, i + 1, false));
    }
    for d in [" — ", " – ", " - ", "— ", "– "] {
        if let Some(i) = head.find(d) {
            out.push((i, i + d.len(), false));
        }
    }
    if let Some(i) = head.find(". ") {
        out.push((i, i + 2, true));
    }
    out.sort_by_key(|o| o.0);
    out
}

/// `NAME: words` and the like: the name, a note, and where the words start.
fn speech_head(t: &str) -> Option<(String, Option<String>, usize, bool)> {
    for (at, after, caps_only) in separators(t) {
        let Some((name, note)) = split_name(&t[..at]) else {
            continue;
        };
        if caps_only && (!is_caps_name(&name) || name.chars().count() < 2) {
            continue;
        }
        return Some((name, note, after, caps_only));
    }
    None
}

/// A name alone on its paragraph, in capitals (a note may follow it).
fn name_alone(t: &str) -> Option<(String, Option<String>)> {
    if t.contains('\n') {
        return None;
    }
    let (name, note) = split_name(t.trim_end_matches([':', '.']))?;
    is_caps_name(&name).then_some((name, note))
}

/// Words in capitals on a line of their own that are no one.
const NOT_NAMES: [&str; 14] = [
    "SON",
    "SONSÖZ",
    "KARANLIK",
    "PERDE",
    "PERDE KAPANIR",
    "IŞIKLAR",
    "KAPANIŞ",
    "FİNAL",
    "END",
    "THE END",
    "CURTAIN",
    "BLACKOUT",
    "FINIS",
    "FİNİS",
];

fn key(name: &str) -> String {
    upper_tr(name)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// An act or scene heading, a preface's or notes' title: a line to itself.
pub fn is_heading(line: &str) -> bool {
    heading(line).is_some() || is_cast_heading(line)
}

/// Whether a line starts something (for joining wrapped lines): a name,
/// a heading, a parenthesis.
pub fn starts_item(line: &str) -> bool {
    let t = line.trim().trim_start_matches(['*', '_']);
    t.starts_with(['(', '[', '#'])
        || heading(t).is_some()
        || is_cast_heading(t)
        || speech_head(t).is_some_and(|(name, ..)| is_caps_name(&name))
        || name_alone(t).is_some()
}

// ------------------------------------------------------------------ runs

/// The runs from byte `at` of their joined text on.
fn runs_from(runs: &[Run], at: usize) -> Vec<Run> {
    let mut out = Vec::new();
    let mut pos = 0;
    for r in runs {
        let end = pos + r.text.len();
        if end > at {
            let from = at.saturating_sub(pos);
            out.push(Run {
                text: r.text[from..].to_string(),
                ..r.clone()
            });
        }
        pos = end;
    }
    if let Some(first) = out.first_mut() {
        first.text = first.text.trim_start().to_string();
    }
    out.retain(|r| !r.text.is_empty());
    out
}

/// The name's runs are bold.
fn bold_name(runs: &[Run], name_end: usize) -> bool {
    let mut pos = 0;
    for r in runs {
        if pos >= name_end {
            break;
        }
        if !r.bold
            && r.text[..r.text.len().min(name_end - pos)]
                .trim()
                .chars()
                .count()
                > 0
        {
            return false;
        }
        pos += r.text.len();
    }
    true
}

/// A paragraph all in parentheses or brackets, or all in italics.
fn direction(p: &Para) -> Option<Vec<Run>> {
    let t = p.text();
    let t = t.trim();
    let wrapped = (t.starts_with('(') && t.ends_with(')') && balanced(t, '(', ')'))
        || (t.starts_with('[') && t.ends_with(']') && balanced(t, '[', ']'));
    if wrapped {
        let inner = &t[1..t.len() - 1];
        return Some(vec![Run {
            text: inner.trim().to_string(),
            ..Run::default()
        }]);
    }
    p.all_italic().then(|| {
        p.runs
            .iter()
            .map(|r| Run {
                italic: false,
                ..r.clone()
            })
            .collect()
    })
}

/// The first opening parenthesis closes at the very end.
fn balanced(t: &str, open: char, close: char) -> bool {
    let mut depth = 0;
    for (i, c) in t.char_indices() {
        if c == open {
            depth += 1;
        } else if c == close {
            depth -= 1;
            if depth == 0 && i + c.len_utf8() < t.len() {
                return false;
            }
        }
    }
    depth == 0
}

// ------------------------------------------------------------------ shaping

struct Shaper {
    people: Vec<Person>,
    index: HashMap<String, usize>,
}

impl Shaper {
    fn person(&mut self, name: &str) -> usize {
        let k = key(name);
        if let Some(&i) = self.index.get(&k) {
            return i;
        }
        // "OZAN" for "Ozan Usta" of a cast list: the first word, if only one
        // listed name starts so.
        let first: Vec<usize> = self
            .people
            .iter()
            .enumerate()
            .filter(|(_, p)| p.lines == 0 && key(&p.name).split(' ').next() == Some(k.as_str()))
            .map(|(i, _)| i)
            .collect();
        if let [i] = first[..] {
            // Their lines' name is the one over them.
            self.people[i].name = upper_tr(name.trim());
            self.index.insert(k, i);
            return i;
        }
        self.people.push(Person {
            name: upper_tr(name.trim()),
            full: None,
            desc: None,
            alias: String::new(),
            lines: 0,
            sample: String::new(),
            not_character: false,
            merged_into: None,
        });
        self.index.insert(k, self.people.len() - 1);
        self.people.len() - 1
    }

    fn listed(&mut self, name: &str, desc: Option<String>) {
        let i = self.person(name);
        let p = &mut self.people[i];
        if name.chars().any(char::is_lowercase) {
            p.full = Some(name.trim().to_string());
        }
        if desc.is_some() {
            p.desc = desc;
        }
    }
}

/// A cast list's entry: `SELİN — köy öğretmeni`, `Ozan Usta, değirmenci`,
/// `İHTİYAR (yaşlı bir yolcu)`, or a name alone.
fn cast_entry(t: &str) -> Option<(String, Option<String>)> {
    if t.contains('\n') || t.split_whitespace().count() > 25 {
        return None;
    }
    if let Some((name, note)) = split_name(t) {
        return Some((name, note.filter(|n| !n.is_empty())));
    }
    let cut = t
        .char_indices()
        .find(|&(_, c)| ",:—–(".contains(c) || c == '-')
        .map(|(i, _)| i)?;
    let (name, _) = split_name(&t[..cut])?;
    let desc = t[cut..]
        .trim_start_matches(|c: char| c.is_whitespace() || ",:—–-(".contains(c))
        .trim_end_matches(')')
        .trim();
    Some((name, (!desc.is_empty()).then(|| desc.to_string())))
}

/// The paragraphs a draft is made of: who speaks is sure only once all of
/// them are counted (a name written once before a colon may be a word).
pub fn shape(src: &Source, lang: Lang) -> Draft {
    let paras = &src.paras;
    // Names before a colon or alone, counted; names in bold count as sure.
    let mut seen: HashMap<String, usize> = HashMap::new();
    let mut alone = 0;
    for p in paras {
        let t = p.text();
        if let Some((name, ..)) = speech_head(&t) {
            *seen.entry(key(&name)).or_default() += 1;
        } else if let Some((name, _)) = name_alone(&t) {
            *seen.entry(key(&name)).or_default() += 1;
            alone += 1;
        }
    }
    // Names over their lines, the play's way: then a name once is enough.
    let alone_style = alone >= 3;
    let mut sh = Shaper {
        people: Vec::new(),
        index: HashMap::new(),
    };
    let mut items: Vec<Item> = Vec::new();
    let mut front: Vec<&Para> = Vec::new();
    let mut started = false;
    let mut in_list = false;
    let mut i = 0;
    while i < paras.len() {
        let p = &paras[i];
        let t = p.text();
        let t = t.trim();
        i += 1;
        if let Some((level, mut rest)) = heading(t) {
            started = true;
            in_list = false;
            // "BİRİNCİ PERDE", and its title on the line under it.
            if level == Level::Act
                && rest.is_empty()
                && let Some(n) = paras.get(i)
                && act_title(&n.text())
            {
                rest = n.text().trim().to_string();
                i += 1;
            }
            items.push(match level {
                Level::Act => Item::Act(rest),
                Level::Scene => Item::Scene(rest),
                Level::Preface => Item::Preface,
                Level::Notes => Item::Notes,
            });
            continue;
        }
        if is_cast_heading(t) {
            started = true;
            in_list = true;
            continue;
        }
        if in_list {
            if let Some((name, desc)) = cast_entry(t) {
                sh.listed(&name, desc);
                continue;
            }
            in_list = false;
        }
        if let Some(runs) = direction(p) {
            started = true;
            items.push(Item::Direction(runs));
            continue;
        }
        // NAME: words.
        if let Some((name, note, after, _)) = speech_head(t) {
            let k = key(&name);
            let sure = is_caps_name(&name)
                || sh.index.contains_key(&k)
                || bold_name(&p.runs, t.find(&name).map_or(0, |s| s + name.len()))
                || seen.get(&k).is_some_and(|&n| n >= 3);
            if sure {
                started = true;
                let lead = p.text().len() - p.text().trim_start().len();
                let mut said = note_run(note);
                said.extend(runs_from(&p.runs, lead + after));
                let who = sh.person(&name);
                items.push(Item::Speech {
                    who,
                    said,
                    raw: p.runs.clone(),
                });
                continue;
            }
        }
        // NAME, then the words under it.
        if let Some((name, note)) = name_alone(t) {
            let k = key(&name);
            let next_ok = paras.get(i).is_some_and(|n| {
                let nt = n.text();
                heading(&nt).is_none() && name_alone(&nt).is_none() && !is_cast_heading(&nt)
            });
            let often = seen.get(&k).is_some_and(|&n| n >= 2) || alone_style;
            if next_ok && !NOT_NAMES.contains(&k.as_str()) && (often || sh.index.contains_key(&k)) {
                started = true;
                let mut said = note_run(note);
                let mut raw = p.runs.clone();
                // A note in parentheses on its own line before the words.
                if let Some(n) = paras.get(i)
                    && let Some(d) = direction(n)
                    && n.text().trim().starts_with('(')
                    && paras.get(i + 1).is_some_and(|w| direction(w).is_none())
                {
                    let note: String = d.iter().map(|r| r.text.as_str()).collect();
                    said.extend(note_run(Some(note)));
                    i += 1;
                }
                if let Some(w) = paras.get(i) {
                    raw.push(Run {
                        text: "\n".into(),
                        ..Run::default()
                    });
                    raw.extend(w.runs.iter().cloned());
                    if !said.is_empty() {
                        said.push(Run {
                            text: " ".into(),
                            ..Run::default()
                        });
                    }
                    said.extend(w.runs.iter().cloned());
                    i += 1;
                }
                let who = sh.person(&name);
                items.push(Item::Speech { who, said, raw });
                continue;
            }
        }
        if !started {
            front.push(p);
            continue;
        }
        // A word in capitals alone ("SON", "KARANLIK") is not more words.
        if name_alone(t).is_some() {
            items.push(Item::Plain(p.runs.clone()));
            continue;
        }
        // A paragraph after a line of dialogue goes on it.
        if let Some(Item::Speech { said, raw, .. }) = items.last_mut() {
            for v in [said, raw] {
                v.push(Run {
                    text: "\n".into(),
                    ..Run::default()
                });
                v.extend(p.runs.iter().cloned());
            }
            continue;
        }
        items.push(Item::Plain(p.runs.clone()));
    }
    // Counts and samples.
    for it in &items {
        if let Item::Speech { who, said, .. } = it {
            let p = &mut sh.people[*who];
            p.lines += 1;
            if p.sample.is_empty() {
                let s: String = said.iter().map(|r| r.text.as_str()).collect();
                p.sample = s.replace('\n', " ").chars().take(70).collect();
            }
        }
    }
    let (title, author, rest) = front_matter(src, &front);
    let mut items_all: Vec<Item> = rest
        .into_iter()
        .map(|p| Item::Plain(p.runs.clone()))
        .collect();
    items_all.extend(items);
    let mut d = Draft {
        title,
        author,
        people: sh.people,
        items: items_all,
        lang,
    };
    d.give_aliases();
    d
}

fn note_run(note: Option<String>) -> Vec<Run> {
    note.filter(|n| !n.is_empty())
        .map(|n| {
            vec![Run {
                text: format!("({n})"),
                italic: true,
                bold: false,
            }]
        })
        .unwrap_or_default()
}

/// The title and author: the document's, or the first lines before the
/// play starts (`Yazan: …`, `by …`), and what is left of those lines.
fn front_matter<'a>(src: &Source, front: &[&'a Para]) -> (String, String, Vec<&'a Para>) {
    let mut rest: Vec<&Para> = front.to_vec();
    let short = |p: &Para| p.text().split_whitespace().count() <= 12 && !p.text().contains('\n');
    let mut title = None;
    if let Some(i) = rest.iter().position(|p| p.title) {
        title = Some(rest.remove(i).text());
    } else if let Some(t) = src.title.clone().filter(|t| !odd_title(t)) {
        title = Some(t);
        if let Some(i) = rest
            .iter()
            .position(|p| key(&p.text()) == key(&title.clone().unwrap_or_default()))
        {
            rest.remove(i);
        }
    } else if rest.first().is_some_and(|p| short(p)) {
        title = Some(rest.remove(0).text());
    }
    let mut author = None;
    for (i, p) in rest.iter().enumerate() {
        let t = p.text();
        let low = crate::cast::lower_tr(t.trim());
        for lead in ["yazan", "yazar", "by", "written by"] {
            if let Some(after) = low.strip_prefix(lead)
                && (after.starts_with(':') || after.starts_with(' '))
            {
                let cut = t.trim().len() - after.len();
                author = Some(
                    t.trim()[cut..]
                        .trim_start_matches([':', ' '])
                        .trim()
                        .to_string(),
                );
                break;
            }
        }
        if author.is_some() {
            rest.remove(i);
            break;
        }
    }
    if author.is_none()
        && let Some(p) = rest.first()
        && p.text().split_whitespace().count() <= 4
        && !p.text().trim_end().ends_with(['.', '!', '?', ':'])
    {
        author = Some(rest.remove(0).text());
    }
    let author = author.or_else(|| src.author.clone()).unwrap_or_default();
    (
        title.unwrap_or_default().trim().to_string(),
        author.trim().to_string(),
        rest,
    )
}

/// Titles a word processor fills in on its own.
fn odd_title(t: &str) -> bool {
    let l = t.to_lowercase();
    l.starts_with("microsoft word")
        || l.ends_with(".doc")
        || l.ends_with(".docx")
        || l == "untitled"
        || l == "adsız"
}

// ------------------------------------------------------------------ the draft

impl Draft {
    /// Who a person's lines go to, through merges.
    pub fn owner(&self, mut i: usize) -> usize {
        let mut n = 0;
        while let Some(j) = self.people[i].merged_into {
            i = j;
            n += 1;
            if n > self.people.len() {
                break;
            }
        }
        i
    }

    /// The people the play declares: speaking, not merged away, characters.
    pub fn cast(&self) -> Vec<usize> {
        (0..self.people.len())
            .filter(|&i| {
                let p = &self.people[i];
                p.merged_into.is_none()
                    && !p.not_character
                    && (self.lines_of(i) > 0 || p.desc.is_some() || p.full.is_some())
            })
            .collect()
    }

    /// Fresh aliases for everyone, in order, each unlike the others.
    fn give_aliases(&mut self) {
        let mut decls = String::new();
        for i in 0..self.people.len() {
            let cast = Cast::of_play(&decls);
            let a = cast.suggest_alias(&self.people[i].name);
            decls.push_str(&format!("\\karakter{{X}}{{{a}}}\n"));
            self.people[i].alias = a;
        }
    }

    /// Why `alias` can't be person `i`'s, if it can't.
    pub fn alias_problem(&self, i: usize, alias: &str) -> Option<&'static str> {
        let others: String = self
            .cast()
            .into_iter()
            .filter(|&j| j != i)
            .map(|j| format!("\\karakter{{X}}{{{}}}\n", self.people[j].alias))
            .collect();
        if alias.is_empty() || !alias.chars().all(|c| c.is_ascii_lowercase()) {
            return Some(crate::lang::l(
                "yalnız küçük İngilizce harfler (a–z)",
                "lowercase English letters only (a–z)",
            ));
        }
        Cast::of_play(&others)
            .alias_taken(alias)
            .then_some(crate::lang::l(
                "bu kısaltma alınmış ya da bir LaTeX komutu",
                "this alias is taken or a LaTeX command",
            ))
    }

    /// `from`'s lines go to `into`; their cast-list details too, where
    /// `into` has none.
    pub fn merge(&mut self, from: usize, into: usize) {
        let into = self.owner(into);
        if from == into {
            return;
        }
        for p in &mut self.people {
            if p.merged_into == Some(from) {
                p.merged_into = Some(into);
            }
        }
        let (full, desc) = {
            let f = &self.people[from];
            (f.full.clone(), f.desc.clone())
        };
        let t = &mut self.people[into];
        t.full = t.full.take().or(full);
        t.desc = t.desc.take().or(desc);
        self.people[from].merged_into = Some(into);
    }

    /// A merge undone: `i` speaks their own lines again.
    pub fn unmerge(&mut self, i: usize) {
        self.people[i].merged_into = None;
    }

    /// The lines a person speaks, with those of the people merged into them.
    pub fn lines_of(&self, i: usize) -> usize {
        (0..self.people.len())
            .filter(|&j| self.owner(j) == i)
            .map(|j| self.people[j].lines)
            .sum()
    }

    pub fn counts(&self) -> Counts {
        let mut c = Counts::default();
        for it in &self.items {
            match it {
                Item::Act(_) => c.acts += 1,
                Item::Preface | Item::Notes => {}
                Item::Scene(_) => c.scenes += 1,
                Item::Speech { who, .. } if !self.people[self.owner(*who)].not_character => {
                    c.lines += 1
                }
                Item::Speech { .. } | Item::Plain(_) => c.plain += 1,
                Item::Direction(_) => c.directions += 1,
            }
        }
        c
    }

    /// The play, ready to be saved.
    pub fn tex(&self) -> String {
        let t = crate::library::template(&self.title, &self.author, self.lang);
        let mut decls = String::new();
        for i in self.cast() {
            let p = &self.people[i];
            let script = escape_pasted(&p.name, false);
            let desc = escape_pasted(p.desc.as_deref().unwrap_or(""), false);
            let full = p
                .full
                .as_deref()
                .filter(|f| key(f) != key(&p.name))
                .map(|f| escape_pasted(f, false));
            match full {
                Some(f) => decls.push_str(&format!(
                    "\\karakter[{f}][{desc}]{{{script}}}{{{}}}\n",
                    p.alias
                )),
                None => decls.push_str(&format!("\\karakter[{desc}]{{{script}}}{{{}}}\n", p.alias)),
            }
        }
        let (act, scene) = match self.lang {
            Lang::Tr => ("perde", "sahne"),
            Lang::En => ("act", "scene"),
        };
        let mut body = String::new();
        if !self.items.iter().any(|i| matches!(i, Item::Scene(_))) {
            let first = self
                .items
                .iter()
                .position(|i| !matches!(i, Item::Plain(_)))
                .unwrap_or(self.items.len());
            for it in &self.items[..first] {
                self.item_tex(it, act, scene, &mut body);
            }
            body.push_str(&format!("#{scene}\n\n"));
            for it in &self.items[first..] {
                self.item_tex(it, act, scene, &mut body);
            }
        } else {
            for it in &self.items {
                self.item_tex(it, act, scene, &mut body);
            }
        }
        let begin = t.find("\\begin{document}").unwrap_or(t.len());
        let from = t
            .find("\\kisiler\n\n")
            .map_or(begin, |i| i + "\\kisiler\n\n".len());
        let to = t.find("\\end{document}").unwrap_or(t.len());
        let decls = if decls.is_empty() {
            decls
        } else {
            format!("{decls}\n")
        };
        format!(
            "{}{decls}{}{body}{}",
            &t[..begin],
            &t[begin..from],
            &t[to..]
        )
    }

    fn item_tex(&self, it: &Item, act: &str, scene: &str, out: &mut String) {
        match it {
            Item::Act(t) => out.push_str(&heading_line(act, t)),
            Item::Preface => out.push_str(match self.lang {
                Lang::Tr => "#önsöz\n\n",
                Lang::En => "#preface\n\n",
            }),
            Item::Notes => out.push_str(match self.lang {
                Lang::Tr => "#notlar\n\n",
                Lang::En => "#notes\n\n",
            }),
            Item::Scene(t) => out.push_str(&heading_line(scene, t)),
            Item::Direction(runs) => out.push_str(&format!(
                "\\action{{{}}}\n\n",
                inline(runs, false).replace('\n', " ")
            )),
            Item::Plain(runs) => {
                out.push_str(&inline(runs, false).replace('\n', "\\\\\n"));
                out.push_str("\n\n");
            }
            Item::Speech { who, said, raw } => {
                let p = &self.people[self.owner(*who)];
                if p.not_character {
                    out.push_str(&inline(raw, false).replace('\n', "\\\\\n"));
                } else {
                    let words = inline(said, true);
                    // A line that starts with a command would end the speech.
                    let words = words
                        .split('\n')
                        .enumerate()
                        .map(|(i, l)| {
                            if i > 0 && l.starts_with('\\') {
                                format!("{{}}{l}")
                            } else {
                                l.to_string()
                            }
                        })
                        .collect::<Vec<_>>()
                        .join("\n");
                    if words.is_empty() {
                        out.push_str(&format!("\\{}", p.alias));
                    } else {
                        out.push_str(&format!("\\{} {words}", p.alias));
                    }
                }
                out.push_str("\n\n");
            }
        }
    }
}

fn heading_line(word: &str, rest: &str) -> String {
    let rest = escape_pasted(rest, false).replace('\n', " ");
    if rest.trim().is_empty() {
        format!("#{word}\n\n")
    } else {
        format!("#{word} {}\n\n", rest.trim())
    }
}

/// Runs as TeX: in a line of dialogue a run in italics, or words in
/// parentheses, are an inline direction (`\ic{…}`); one word in italics is
/// emphasis.
fn inline(runs: &[Run], dialogue: bool) -> String {
    let mut out = String::new();
    for r in runs {
        if r.italic && dialogue {
            let t = r.text.trim();
            let inner = t
                .strip_prefix('(')
                .and_then(|s| s.strip_suffix(')'))
                .unwrap_or(t)
                .trim();
            let lead = &r.text[..r.text.len() - r.text.trim_start().len()];
            let trail = &r.text[r.text.trim_end().len()..];
            if inner.is_empty() {
                out.push_str(&r.text);
            } else if t.starts_with('(') || inner.split_whitespace().count() > 1 {
                out.push_str(&format!(
                    "{lead}\\ic{{{}}}{trail}",
                    escape_pasted(inner, false)
                ));
            } else {
                out.push_str(&format!(
                    "{lead}\\emph{{{}}}{trail}",
                    escape_pasted(inner, false)
                ));
            }
        } else if r.italic && !r.text.trim().is_empty() {
            let lead = &r.text[..r.text.len() - r.text.trim_start().len()];
            let trail = &r.text[r.text.trim_end().len()..];
            out.push_str(&format!(
                "{lead}\\emph{{{}}}{trail}",
                escape_pasted(r.text.trim(), false)
            ));
        } else if dialogue {
            out.push_str(&parens(&r.text));
        } else {
            out.push_str(&escape_pasted(&r.text, false));
        }
    }
    out.trim().to_string()
}

/// Plain words in a line of dialogue, `(gülerek)` as `\ic{gülerek}`.
fn parens(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(i) = rest.find('(') {
        let Some(j) = rest[i..].find(')').map(|j| i + j) else {
            break;
        };
        out.push_str(&escape_pasted(&rest[..i], false));
        let inner = rest[i + 1..j].trim();
        if inner.is_empty() || inner.contains('(') {
            out.push_str(&escape_pasted(&rest[i..=j], false));
        } else {
            out.push_str(&format!("\\ic{{{}}}", escape_pasted(inner, false)));
        }
        rest = &rest[j + 1..];
    }
    out.push_str(&escape_pasted(rest, false));
    out
}
