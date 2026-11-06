//! A PDF's text: through `pdftotext` where it is installed (it keeps the
//! layout, a name column among it), else read in Rust. Running heads and
//! page numbers go, margin line numbers too, and wrapped lines are joined.

use super::{Format, Para, Source, plain, shape};
use anyhow::{Result, bail};
use std::collections::HashSet;
use std::path::Path;

/// A PDF of pictures: it needs reading from them.
#[derive(Debug)]
pub struct NoText;

impl std::fmt::Display for NoText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(crate::lang::l(
            "bu PDF'te metin yok, taranmış sayfalar gibi: OCR ile okunması gerek",
            "this PDF has no text, as if scanned: it needs OCR",
        ))
    }
}

impl std::error::Error for NoText {}

/// A whole line in italics, marked on its way through the unwrapping: a
/// direction, or the speech that goes on after one.
const DIRECTION: char = '\u{e000}';
const RESUMES: char = '\u{e001}';

pub fn read(path: &Path, bytes: &[u8]) -> Result<Source> {
    let text = match pdftotext(path) {
        Some(t) => t,
        None => in_rust(bytes)?,
    };
    let letters = text.chars().filter(|c| c.is_alphabetic()).count();
    let pages = text.matches('\u{c}').count().max(1);
    if letters < 40 * pages {
        return Err(NoText.into());
    }
    Ok(of_text(&text, &italic_lines(path)))
}

/// A line as `pdftotext` and `pdftohtml` both have it: they space it apart
/// differently.
fn line_key(t: &str) -> String {
    t.chars().filter(|c| !c.is_whitespace()).collect()
}

/// The lines in italics on each page, by `pdftohtml`, the one tool that
/// says; none where it isn't there.
fn italic_lines(path: &Path) -> Vec<HashSet<String>> {
    let out = super::ocr::poppler("pdftohtml")
        .args(["-xml", "-i", "-stdout", "-enc", "UTF-8"])
        .arg(path)
        .output();
    match out {
        Ok(o) if o.status.success() => italics_of(&String::from_utf8_lossy(&o.stdout)),
        _ => Vec::new(),
    }
}

/// A line of `pdftohtml -xml`, which comes in pieces, one to each font.
#[derive(Default)]
struct Pieces {
    top: i32,
    height: i32,
    text: String,
    /// Small letters in italics, and in all: a direction's names aren't.
    italic: usize,
    small: usize,
    /// How it starts: the end of a speech can run into a long note.
    starts_italic: Option<bool>,
}

impl Pieces {
    fn italic(&self) -> bool {
        self.starts_italic == Some(true) && self.italic * 10 >= self.small * 8
    }
}

/// From `pdftohtml -xml`.
fn italics_of(xml: &str) -> Vec<HashSet<String>> {
    let unescape = |t: &str| {
        t.replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&#34;", "\"")
            .replace("&#160;", " ")
            .replace("&amp;", "&")
    };
    let number = |l: &str, name: &str| -> Option<i32> {
        let at = l.find(name)? + name.len();
        l[at..].split('"').next()?.parse().ok()
    };
    let mut pages = Vec::new();
    let mut lines: Vec<Pieces> = Vec::new();
    for l in xml.lines() {
        if l.starts_with("</page>") {
            pages.push(
                lines
                    .drain(..)
                    .filter(Pieces::italic)
                    .map(|p| line_key(&p.text))
                    .collect(),
            );
            continue;
        }
        if !l.starts_with("<text ") {
            continue;
        }
        let (Some(top), Some(height), Some(start), Some(end)) = (
            number(l, " top=\""),
            number(l, " height=\""),
            l.find('>'),
            l.rfind("</text>"),
        ) else {
            continue;
        };
        let inner = l[start + 1..end].replace("<b>", "").replace("</b>", "");
        let italic = inner.contains("<i>");
        let bare = unescape(&inner.replace("<i>", "").replace("</i>", ""));
        let small = bare.chars().filter(|c| c.is_lowercase()).count();
        // Small capitals sit a little lower on the line.
        let same = lines
            .last()
            .is_some_and(|p| (p.top - top).abs() * 2 < p.height.max(height));
        if !same {
            lines.push(Pieces {
                top,
                height,
                ..Pieces::default()
            });
        }
        if let Some(p) = lines.last_mut() {
            p.text.push_str(&bare);
            p.small += small;
            if small > 0 {
                p.starts_italic.get_or_insert(italic);
            }
            p.italic += if italic { small } else { 0 };
        }
    }
    pages
}

/// Sets the book's lines in italics apart. Few of them are only emphasis.
fn italics_apart(lines: Vec<String>, italics: &[HashSet<String>]) -> Vec<String> {
    let mut page = 0;
    let mut marks = Vec::with_capacity(lines.len());
    for l in &lines {
        if l == PAGE_END {
            page += 1;
        }
        let t = line_key(l);
        marks.push(!t.is_empty() && italics.get(page).is_some_and(|s| s.contains(&t)));
    }
    let many = marks.iter().filter(|&&m| m).count();
    if many < 10 || many * 50 < lines.len() {
        return lines;
    }
    let mut out = Vec::with_capacity(lines.len() + many);
    let mut was = false;
    // Inside `(…)` in a speech that goes over the line: a note of that speech.
    let mut open = 0;
    for (l, italic) in lines.into_iter().zip(marks) {
        let t = l.trim();
        if t.is_empty() || l == PAGE_END {
            out.push(l);
            continue;
        }
        let direction = italic && open == 0 && !shape::starts_item(t);
        if !direction {
            let (a, b) = (t.matches('(').count(), t.matches(')').count());
            open = (open + a).saturating_sub(b);
            if shape::starts_item(t) {
                open = a.saturating_sub(b);
            }
        }
        match (direction, was) {
            (true, false) => {
                out.push(String::new());
                out.push(format!("{DIRECTION}{t}"));
            }
            // Each line, for it not to read as a name's column.
            (true, true) => out.push(format!("{DIRECTION}{t}")),
            (false, true) => {
                out.push(String::new());
                if shape::starts_item(t) {
                    out.push(l);
                } else {
                    out.push(format!("{RESUMES}{t}"));
                }
            }
            _ => out.push(l),
        }
        was = direction;
    }
    out
}

/// A book from its text, pages parted by form feeds, and the lines in
/// italics on each.
pub(super) fn of_text(text: &str, italics: &[HashSet<String>]) -> Source {
    let (pages, body_page) = numbers(text);
    let lines: Vec<String> = page_lines(text)
        .into_iter()
        .filter(|l| !page_number(l))
        .collect();
    let lines = italics_apart(strip_numbers(&lines), italics);
    let lines = names_put_right(&name_columns(&lines));
    let mut paras = Vec::new();
    let mut feet = Vec::new();
    for (page, group) in lines.split(|l| l == PAGE_END).enumerate() {
        let (body, notes) = group.split_at(notes_at(group));
        let tagged = |t: &String, foot: bool| {
            let bare = t.replace([DIRECTION, RESUMES], "");
            let mut p = Para {
                page: Some(page),
                foot,
                resumes: t.starts_with(RESUMES),
                ..Para::plain(&bare)
            };
            if t.starts_with(DIRECTION) {
                p.runs.iter_mut().for_each(|r| r.italic = true);
            }
            p
        };
        paras.extend(plain::unwrap(body, false).iter().map(|p| tagged(p, false)));
        feet.extend(
            plain::unwrap(&notes_of(notes), true)
                .iter()
                .map(|p| tagged(p, true)),
        );
    }
    paras.append(&mut feet);
    Source {
        format: Format::Pdf,
        paras,
        title: None,
        author: None,
        pages,
        body_page,
        plays: Vec::new(),
    }
}

fn pdftotext(path: &Path) -> Option<String> {
    let out = super::ocr::poppler("pdftotext")
        .args(["-layout", "-enc", "UTF-8"])
        .arg(path)
        .arg("-")
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

fn in_rust(bytes: &[u8]) -> Result<String> {
    // The reader panics on some damaged files: an error, not a crash.
    let bytes = bytes.to_vec();
    match std::panic::catch_unwind(move || pdf_extract::extract_text_from_mem(&bytes)) {
        Ok(Ok(t)) => Ok(t),
        Ok(Err(e)) => bail!("{e}"),
        Err(_) => bail!(crate::lang::l(
            "bu PDF okunamadı",
            "this PDF could not be read"
        )),
    }
}

/// Between one page's lines and the next's.
const PAGE_END: &str = "\u{c}";

/// The edit distance of two short lines.
pub(super) fn distance(a: &[char], b: &[char]) -> usize {
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, x) in a.iter().enumerate() {
        let mut diag = row[0];
        row[0] = i + 1;
        for (j, y) in b.iter().enumerate() {
            let up = row[j + 1];
            row[j + 1] = (diag + usize::from(x != y)).min(up + 1).min(row[j] + 1);
            diag = up;
        }
    }
    row[b.len()]
}

/// The pages whose first line is a running head: on the left pages the
/// author, on the right the title, each read a little differently by OCR.
/// A name at the top of a page falls on both sides.
fn alternating_heads(pages: &[Vec<&str>]) -> Vec<bool> {
    let tops: Vec<Vec<char>> = pages
        .iter()
        .map(|p| {
            let first = p.iter().find(|l| !l.trim().is_empty()).unwrap_or(&"");
            first
                .chars()
                .filter(|c| c.is_alphanumeric())
                .flat_map(char::to_lowercase)
                .collect()
        })
        .collect();
    let like = |a: &[char], b: &[char]| {
        a.len() >= 4 && a.len().abs_diff(b.len()) <= a.len() / 3 && distance(a, b) * 3 <= a.len()
    };
    (0..pages.len())
        .map(|i| {
            let near = i.saturating_sub(HEAD_REACH)..pages.len().min(i + HEAD_REACH);
            let (mut same, mut other) = (0, 0);
            for j in near.filter(|&j| j != i && like(&tops[i], &tops[j])) {
                if j % 2 == i % 2 {
                    same += 1;
                } else {
                    other += 1;
                }
            }
            same >= 3 && same >= 3 * other
        })
        .collect()
}

/// As many pages either way as a running head is compared with.
const HEAD_REACH: usize = 24;

/// The lines of every page, without the line repeated at the top or the
/// bottom of most pages (a running head, a page number) and with a blank
/// line where a page ends.
fn page_lines(text: &str) -> Vec<String> {
    let pages: Vec<Vec<&str>> = text
        .split('\u{c}')
        .map(|p| p.lines().collect::<Vec<_>>())
        .collect();
    let heads = alternating_heads(&pages);
    // A line with its numbers taken out: "Sayfa 3" and "Sayfa 4" are one.
    let key = |l: &str| -> String {
        l.chars()
            .filter(|c| !c.is_ascii_digit())
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    };
    let edge = |p: &Vec<&str>, top: bool| -> Vec<usize> {
        let idx: Vec<usize> = (0..p.len()).filter(|&i| !p[i].trim().is_empty()).collect();
        if top {
            idx.into_iter().take(2).collect()
        } else {
            idx.into_iter().rev().take(2).collect()
        }
    };
    let mut seen: std::collections::HashMap<String, usize> = Default::default();
    for p in &pages {
        let mut keys: Vec<String> = edge(p, true)
            .into_iter()
            .chain(edge(p, false))
            .map(|i| key(p[i]))
            .collect();
        keys.sort();
        keys.dedup();
        for k in keys {
            *seen.entry(k).or_default() += 1;
        }
    }
    let running = |k: &str| {
        k.is_empty() || (pages.len() >= 3 && seen.get(k).is_some_and(|&n| n * 2 >= pages.len()))
    };
    let mut out = Vec::new();
    for (n, p) in pages.iter().enumerate() {
        let mut drop: Vec<usize> = edge(p, true)
            .into_iter()
            .chain(edge(p, false))
            .filter(|&i| running(&key(p[i])))
            .collect();
        drop.extend(edge(p, true).first().filter(|_| heads[n]));
        drop.extend(
            edge(p, false)
                .first()
                .filter(|&&i| foot_number(p[i]).is_some()),
        );
        for (i, l) in p.iter().enumerate() {
            if !drop.contains(&i) {
                out.push(l.trim_end().to_string());
            }
        }
        out.push(PAGE_END.to_string());
    }
    out
}

/// What a page's last line says, if it is a number. OCR reads 1 as ı or l
/// and puts spaces in numbers.
#[derive(Debug, PartialEq)]
enum Foot {
    Arabic(usize),
    Roman,
}

fn foot_number(l: &str) -> Option<Foot> {
    let t: String = l.chars().filter(|c| !c.is_whitespace()).collect();
    if t.is_empty() || t.chars().count() > 9 {
        return None;
    }
    let digits: String = t
        .chars()
        .map(|c| match c {
            'ı' | 'l' | 'I' | '|' => '1',
            'O' | 'o' => '0',
            c => c,
        })
        .collect();
    if digits.len() <= 4 && digits.chars().all(|c| c.is_ascii_digit()) {
        return digits.parse().ok().map(Foot::Arabic);
    }
    t.chars()
        .all(|c| "IVXLCUivxlcıl)('\\.,".contains(c))
        .then_some(Foot::Roman)
        .filter(|_| t.chars().any(|c| "IVXivx".contains(c)))
}

/// The page labels of a PDF, and the page its text starts on: where page 1
/// follows pages in Roman numerals. Most pages agree on how far the printed
/// numbers are from the file's.
fn numbers(text: &str) -> (Vec<String>, Option<usize>) {
    let pages: Vec<&str> = text.split('\u{c}').collect();
    let feet: Vec<Option<Foot>> = pages
        .iter()
        .map(|p| {
            p.lines()
                .rev()
                .find(|l| !l.trim().is_empty())
                .and_then(foot_number)
        })
        .collect();
    let mut votes: std::collections::HashMap<usize, usize> = Default::default();
    for (i, f) in feet.iter().enumerate() {
        if let Some(Foot::Arabic(n)) = f
            && *n > 0
            && *n <= i + 1
        {
            *votes.entry(i + 1 - n).or_default() += 1;
        }
    }
    let first = votes
        .into_iter()
        .filter(|&(_, n)| n >= 3)
        .max_by_key(|&(at, n)| (n, at))
        .map(|(at, _)| at);
    // The Roman numerals that came out clean say where those start.
    let mut votes: std::collections::HashMap<usize, usize> = Default::default();
    for (i, p) in pages.iter().enumerate() {
        let foot = p.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("");
        if let Some(n) = (1..60).find(|&n| roman(n) == foot.trim())
            && n <= i + 1
        {
            *votes.entry(i + 1 - n).or_default() += 1;
        }
    }
    let front = votes
        .into_iter()
        .max_by_key(|&(at, n)| (n, at))
        .map_or(0, |(at, _)| at);
    let label = |i: usize| match first {
        Some(at) if i >= at => (i + 1 - at).to_string(),
        Some(_) => roman((i + 1).saturating_sub(front)),
        None => (i + 1).to_string(),
    };
    let labels = (0..pages.len()).map(label).collect();
    let body = first.filter(|&at| {
        feet[..at.min(feet.len())]
            .iter()
            .flatten()
            .filter(|f| **f == Foot::Roman)
            .count()
            >= 2
    });
    (labels, body)
}

/// A number as Roman numerals.
fn roman(mut n: usize) -> String {
    let mut out = String::new();
    for (v, s) in [
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ] {
        while n >= v {
            out.push_str(s);
            n -= v;
        }
    }
    out
}

/// Where a page's footnotes start: a line of marks alone (OCR's `*`), in
/// the page's lower part.
fn notes_at(group: &[String]) -> usize {
    // OCR reads the mark as `*`, `•`, `,,` or `"'`.
    let mark = |l: &String| {
        let t = l.trim();
        !t.is_empty()
            && t.chars().count() <= 12
            && t.chars().any(|c| "*•\"',".contains(c))
            && t.chars().filter(|c| c.is_alphanumeric()).count() <= 1
    };
    let lead = |l: &String| l.trim_start().starts_with(['*', '•']);
    let half = group.len() / 2;
    (half..group.len())
        .find(|&i| mark(&group[i]) || lead(&group[i]))
        .unwrap_or(group.len())
}

/// Footnotes without their marks, a blank line between them.
fn notes_of(lines: &[String]) -> Vec<String> {
    lines
        .iter()
        .map(|l| {
            let t = l
                .trim()
                .trim_start_matches(|c: char| !c.is_alphanumeric() && c != '(');
            t.to_string()
        })
        .collect()
}

/// A name on its line that OCR got a letter wrong in (`BERfANT`), put as
/// the same name is spelled far more often.
fn names_put_right(lines: &[String]) -> Vec<String> {
    use super::shape::{fold, like_name};
    // Mostly capitals, a few letters, no sentence.
    let name_like = |t: &str| {
        let letters: Vec<char> = t.chars().filter(|c| c.is_alphabetic()).collect();
        let caps = letters.iter().filter(|c| c.is_uppercase()).count();
        (3..=24).contains(&letters.len())
            && caps * 10 >= letters.len() * 6
            && t.split_whitespace().count() <= 3
            && !t.ends_with(['.', ',', ';', '?', '!', ':'])
    };
    let mut count: std::collections::HashMap<String, (String, usize)> = Default::default();
    for l in lines {
        let t = l.trim();
        if name_like(t) {
            count.entry(fold(t)).or_insert_with(|| (t.to_string(), 0)).1 += 1;
        }
    }
    let often: Vec<(&String, &(String, usize))> =
        count.iter().filter(|(_, (_, n))| *n >= OFTEN).collect();
    lines
        .iter()
        .map(|l| {
            let t = l.trim();
            if !name_like(t) {
                return l.clone();
            }
            let k = fold(t);
            let n = count.get(&k).map_or(0, |c| c.1);
            let better: Vec<&String> = often
                .iter()
                .filter(|(o, (_, m))| **o != k && *m > n && like_name(o, &k))
                .map(|(_, (name, _))| name)
                .collect();
            match better[..] {
                [name] => name.clone(),
                _ => l.clone(),
            }
        })
        .collect()
}

/// As many times as a name on its line is said to be sure of its spelling.
const OFTEN: usize = 4;

/// A line with a number and nothing else: a page's.
fn page_number(l: &str) -> bool {
    let t = l.trim().trim_matches(['-', '–', '—', ' ']);
    !t.is_empty() && t.len() <= 4 && t.chars().all(|c| c.is_ascii_digit())
}

/// Margin line numbers (a draft's "12   SELİN …"), when many lines have one.
fn strip_numbers(lines: &[String]) -> Vec<String> {
    let numbered = |l: &str| -> Option<usize> {
        let t = l.trim_start();
        let digits = t.chars().take_while(char::is_ascii_digit).count();
        let rest = &t[digits..];
        (digits > 0 && digits <= 5 && rest.starts_with(' ') && !rest.trim().is_empty())
            .then(|| l.len() - rest.len())
    };
    let full: Vec<&String> = lines.iter().filter(|l| !l.trim().is_empty()).collect();
    let n = full.iter().filter(|l| numbered(l).is_some()).count();
    if full.is_empty() || n * 10 < full.len() * 3 {
        return lines.to_vec();
    }
    lines
        .iter()
        .map(|l| match numbered(l) {
            Some(at) => format!("{}{}", " ".repeat(at), &l[at..]),
            None => l.clone(),
        })
        .collect()
}

/// `SELİN      Text` (a name column, as the tiyatro class prints) read as
/// `SELİN: Text`; and `SELİN Text` too, where the columns were lost, for a
/// name in capitals that starts two lines or more so.
fn name_columns(lines: &[String]) -> Vec<String> {
    // "SELİN" of "SELİN Amca, yine…": capitals up to a word that is not.
    let lead = |t: &str| -> Option<usize> {
        let mut end = 0;
        let mut words = 0;
        for (i, w) in t.split(' ').scan(0, |at, w| {
            let i = *at;
            *at += w.len() + 1;
            Some((i, w))
        }) {
            // `NAME. …` needs no column, and shape reads it as it is.
            if w.is_empty() || !w.chars().any(char::is_alphabetic) || w.ends_with('.') {
                return None;
            }
            // A lone capital starts a sentence ("O gün…").
            if crate::writing::is_caps_name(w) && words < 3 && (words > 0 || w.chars().count() > 1)
            {
                end = i + w.len();
                words += 1;
            } else {
                return (words > 0 && (w.chars().any(char::is_lowercase) || w.starts_with('(')))
                    .then_some(end);
            }
        }
        None
    };
    let mut count: std::collections::HashMap<String, usize> = Default::default();
    for l in lines {
        let t = l.trim_start();
        if let Some(end) = lead(t) {
            *count.entry(t[..end].to_string()).or_default() += 1;
        }
    }
    // The play is written so: then a name once is enough.
    let common = count.values().filter(|&&n| n >= 2).sum::<usize>() >= 5;
    lines
        .iter()
        .map(|l| {
            let t = l.trim_start();
            if super::shape::heading(t).is_some() {
                return l.clone();
            }
            if let Some(gap) = t.find("  ") {
                let (name, rest) = (&t[..gap], t[gap..].trim_start());
                if !rest.is_empty() && crate::writing::is_caps_name(name) {
                    return format!("{name}: {rest}");
                }
            }
            if let Some(end) = lead(t)
                && (common || count.get(&t[..end]).is_some_and(|&n| n >= 2))
            {
                return format!("{}: {}", &t[..end], t[end..].trim_start());
            }
            l.clone()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A book as OCR reads it: Roman numerals, then 1 read as ı; a head on
    /// each side, differently each time.
    fn book() -> String {
        let foot = [
            "V", "VI", "VII", "VIII", "ıx", "ı", "2", "3", "4", "5", "6", "7", "8",
        ];
        let head = ["Kar Masalı", "Kar Masa1ı", "Kar Masah", "Kar Masalı"];
        foot.iter()
            .enumerate()
            .map(|(i, f)| {
                let top = if i % 2 == 0 {
                    "Deniz Örnek"
                } else {
                    head[i / 2 % 4]
                };
                format!("{top}\n\nMetin {i}.\nSELİN\n  Metin.\nSon {i}.\n\n{f}\n")
            })
            .collect::<Vec<_>>()
            .join("\u{c}")
    }

    #[test]
    fn roman_numerals_then_page_one() {
        let (labels, body) = numbers(&book());
        assert_eq!(body, Some(5));
        assert_eq!(labels[5], "1");
        assert_eq!(labels[12], "8");
    }

    #[test]
    fn heads_on_both_sides_go_and_names_stay() {
        let lines = page_lines(&book());
        let all = lines.join("\n");
        assert!(!all.contains("Deniz") && !all.contains("Masa"), "{all}");
        assert!(all.contains("SELİN") && !all.contains("VIII"), "{all}");
    }

    #[test]
    fn lines_in_italics() {
        let xml = "<page number=\"1\">\n<text top=\"10\" left=\"1\" width=\"1\" height=\"20\" font=\"0\"><i>Selin girer</i></text>\n<text top=\"13\" left=\"1\" width=\"1\" height=\"20\" font=\"0\">.</text>\n<text top=\"40\" left=\"1\" width=\"1\" height=\"20\" font=\"0\">OZAN. Bir söz, </text>\n<text top=\"40\" left=\"1\" width=\"1\" height=\"20\" font=\"0\"><i>iki</i></text>\n</page>\n";
        let pages = italics_of(xml);
        assert!(pages[0].contains("Selingirer."));
        assert_eq!(pages[0].len(), 1);
    }

    #[test]
    fn a_direction_parts_a_speech() {
        let mut lines: Vec<String> = vec!["OZAN. Bir.".into(); 12];
        let mut set = HashSet::new();
        for i in 0..12 {
            let t = format!("Selin girer {i}.");
            set.insert(line_key(&t));
            lines.push(t);
            lines.push("Ve sürer.".into());
        }
        let out = italics_apart(lines, &[set]);
        assert!(
            out.iter()
                .any(|l| l == &format!("{DIRECTION}Selin girer 0."))
        );
        assert!(out.iter().any(|l| l == &format!("{RESUMES}Ve sürer.")));
    }

    #[test]
    fn a_name_misread_once() {
        let mut lines: Vec<String> = ["BERKANT", "  Bir.", "SELİN", "  İki."]
            .repeat(4)
            .iter()
            .map(|s| s.to_string())
            .collect();
        lines.extend(["BERfANT".to_string(), "  Üç.".to_string()]);
        let out = names_put_right(&lines);
        assert_eq!(out[16], "BERKANT");
        assert_eq!(out[17], "  Üç.");
    }

    #[test]
    fn notes_at_a_page_foot() {
        let lines: Vec<String> = [
            "SELİN",
            "  Bir.",
            "  İki*",
            "  Üç.",
            "",
            "* *",
            "   Açıklama.",
        ]
        .map(String::from)
        .to_vec();
        assert_eq!(notes_at(&lines), 5);
    }

    #[test]
    fn heads_numbers_and_columns() {
        let page = |n: u32, body: &str| {
            format!("Kar Masalı                    {n}\n\n{body}\n\n       {n}\n")
        };
        let text = [
            page(
                1,
                "  1   SELİN      Gel buraya,\n               hemen.\n  2   OZAN       Geliyorum.",
            ),
            page(2, "  3   SELİN      Kar yağıyor.\n  4   OZAN       Yağsın."),
            page(3, "  5   SELİN      Yarın?\n  6   OZAN       Yarın."),
        ]
        .join("\u{c}");
        let lines = name_columns(&strip_numbers(&page_lines(&text)));
        let full: Vec<&str> = lines
            .iter()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .collect();
        assert_eq!(
            full,
            [
                "SELİN: Gel buraya,",
                "hemen.",
                "OZAN: Geliyorum.",
                "SELİN: Kar yağıyor.",
                "OZAN: Yağsın.",
                "SELİN: Yarın?",
                "OZAN: Yarın."
            ]
        );
    }
}
