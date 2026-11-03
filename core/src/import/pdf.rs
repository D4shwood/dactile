//! A PDF's text: through `pdftotext` where it is installed (it keeps the
//! layout, a name column among it), else read in Rust. Running heads and
//! page numbers go, margin line numbers too, and wrapped lines are joined.

use super::{Format, Para, Source, plain};
use anyhow::{Result, bail};
use std::path::Path;
use std::process::Command;

pub fn read(path: &Path, bytes: &[u8]) -> Result<Source> {
    let text = match pdftotext(path) {
        Some(t) => t,
        None => in_rust(bytes)?,
    };
    let letters = text.chars().filter(|c| c.is_alphabetic()).count();
    let pages = text.matches('\u{c}').count().max(1);
    if letters < 40 * pages {
        bail!(crate::lang::l(
            "bu PDF'te metin yok, taranmış sayfalar gibi: önce bir OCR aracından geçmesi gerek",
            "this PDF has no text, as if scanned: it needs OCR first"
        ));
    }
    let lines: Vec<String> = page_lines(&text)
        .into_iter()
        .filter(|l| !page_number(l))
        .collect();
    let lines = name_columns(&strip_numbers(&lines));
    let paras = plain::unwrap(&lines, false)
        .iter()
        .map(|p| Para::plain(p))
        .collect();
    Ok(Source {
        format: Format::Pdf,
        paras,
        title: None,
        author: None,
    })
}

fn pdftotext(path: &Path) -> Option<String> {
    let out = Command::new("pdftotext")
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

/// The lines of every page, without the line repeated at the top or the
/// bottom of most pages (a running head, a page number) and with a blank
/// line where a page ends.
fn page_lines(text: &str) -> Vec<String> {
    let pages: Vec<Vec<&str>> = text
        .split('\u{c}')
        .map(|p| p.lines().collect::<Vec<_>>())
        .filter(|p| p.iter().any(|l| !l.trim().is_empty()))
        .collect();
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
    for p in &pages {
        let drop: Vec<usize> = edge(p, true)
            .into_iter()
            .chain(edge(p, false))
            .filter(|&i| running(&key(p[i])))
            .collect();
        for (i, l) in p.iter().enumerate() {
            if !drop.contains(&i) {
                out.push(l.trim_end().to_string());
            }
        }
        out.push(String::new());
    }
    out
}

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
            if w.is_empty() || !w.chars().any(char::is_alphabetic) {
                return None;
            }
            if crate::writing::is_caps_name(w) && words < 3 {
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
