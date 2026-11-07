//! Plays from elsewhere: a .docx, .odt, PDF with text, Markdown or plain
//! text, read into paragraphs (with their italics and bold where the format
//! keeps them), then shaped into a play (`shape`).

mod docx;
pub mod ocr;
mod odt;
pub mod pages;
pub mod pdf;
pub mod plain;
pub mod shape;
mod xml;

#[cfg(test)]
mod tests;

use anyhow::{Context, Result, bail};
use std::path::Path;

/// A piece of a paragraph in one style. A line break inside a paragraph
/// (Shift+Enter in a word processor) is a `'\n'` in the text.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Run {
    pub text: String,
    pub italic: bool,
    pub bold: bool,
    /// A footnote where it is marked, not part of the words.
    pub note: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Para {
    pub runs: Vec<Run>,
    /// In a heading style, or a Markdown `#` line.
    pub heading: bool,
    /// In the document's title style.
    pub title: bool,
    /// The PDF's page it is on, from 0.
    pub page: Option<usize>,
    /// A footnote, gathered after the text.
    pub foot: bool,
    /// The same speaker going on after a direction.
    pub resumes: bool,
}

impl Para {
    pub fn plain(text: &str) -> Para {
        Para {
            runs: vec![Run {
                text: text.to_string(),
                ..Run::default()
            }],
            ..Para::default()
        }
    }

    pub fn text(&self) -> String {
        self.runs
            .iter()
            .filter(|r| !r.note)
            .map(|r| r.text.as_str())
            .collect()
    }

    /// Every letter in it in italics (and there are some).
    pub fn all_italic(&self) -> bool {
        let mut any = false;
        for r in &self.runs {
            if !r.note && r.text.chars().any(char::is_alphanumeric) {
                if !r.italic {
                    return false;
                }
                any = true;
            }
        }
        any
    }

    /// Its runs, the spaces around it trimmed and touching runs of the same
    /// style made one.
    fn tidy(mut self) -> Para {
        let mut runs: Vec<Run> = Vec::new();
        for r in self.runs.drain(..) {
            let text = r.text.replace(['\t', '\u{a0}'], " ").replace('\r', "");
            if text.is_empty() {
                continue;
            }
            match runs.last_mut() {
                Some(last) if last.italic == r.italic && last.bold == r.bold => {
                    last.text.push_str(&text)
                }
                _ => runs.push(Run { text, ..r }),
            }
        }
        if let Some(first) = runs.first_mut() {
            first.text = first.text.trim_start().to_string();
        }
        if let Some(last) = runs.last_mut() {
            last.text = last.text.trim_end().to_string();
        }
        runs.retain(|r| !r.text.is_empty());
        self.runs = runs;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Docx,
    Odt,
    Pdf,
    Markdown,
    Text,
}

impl Format {
    pub fn of(path: &Path) -> Option<Format> {
        let ext = path.extension()?.to_str()?.to_lowercase();
        Some(match ext.as_str() {
            "docx" => Format::Docx,
            "odt" => Format::Odt,
            "pdf" => Format::Pdf,
            "md" | "markdown" => Format::Markdown,
            "txt" | "text" => Format::Text,
            _ => return None,
        })
    }

    /// Whether the format keeps italics and bold.
    pub fn styled(self) -> bool {
        matches!(self, Format::Docx | Format::Odt | Format::Markdown)
    }
}

/// What a file says, before it is shaped into a play.
#[derive(Debug, Clone)]
pub struct Source {
    pub format: Format,
    pub paras: Vec<Para>,
    /// From the document's properties.
    pub title: Option<String>,
    pub author: Option<String>,
    /// A PDF's pages by the numbers printed on them.
    pub pages: Vec<String>,
    /// The page a book's text starts on, where the pages before it are
    /// numbered in Roman numerals and this one is page 1.
    pub body_page: Option<usize>,
    /// The plays of a volume of several, in order; else none.
    pub plays: Vec<pages::Volume>,
}

impl Source {
    /// What a volume has before its first play, after its contents: an
    /// introduction, a chronology.
    pub fn front_matter(&self) -> String {
        let end = self.plays.first().map_or(0, |p| p.start);
        let contents = self.paras[..end]
            .iter()
            .position(|p| pages::is_contents(&p.text()))
            .and_then(|i| self.paras[i].page);
        self.paras[..end]
            .iter()
            .filter(|p| !p.foot && p.page > contents)
            .map(Para::text)
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}

/// The extensions `read` takes.
pub const EXTENSIONS: [&str; 7] = ["docx", "odt", "pdf", "md", "markdown", "txt", "text"];

pub fn read(path: &Path) -> Result<Source> {
    read_with(path, None)
}

/// A PDF read from its page images.
pub fn read_ocr(path: &Path, job: &ocr::Job) -> Result<Source> {
    read_with(path, Some(job))
}

/// A document read the way it asks to be: from its page images with `force`
/// or where a PDF has no text and the tools are there, else as it is.
pub fn read_as(path: &Path, force: bool, job: &ocr::Job) -> Result<Source> {
    if force {
        return read_ocr(path, job);
    }
    match read(path) {
        Err(e) if e.downcast_ref::<pdf::NoText>().is_some() && ocr::ready() => read_ocr(path, job),
        other => other,
    }
}

fn read_with(path: &Path, job: Option<&ocr::Job>) -> Result<Source> {
    let Some(format) = Format::of(path) else {
        bail!(
            "{}: {}",
            path.display(),
            crate::lang::l(
                "bu biçim içe aktarılamıyor (.docx, .odt, .pdf, .md, .txt olur)",
                "this format can't be imported (.docx, .odt, .pdf, .md, .txt can)"
            )
        );
    };
    let bytes = std::fs::read(path)
        .with_context(|| crate::lang::f("{} okunamadı", "Couldn't read {}", &[&path.display()]))?;
    let mut src = match format {
        Format::Docx => docx::read(&bytes),
        Format::Odt => odt::read(&bytes),
        Format::Pdf => match job {
            Some(job) => ocr::text(path, job).map(|t| pdf::of_text(&t, &[])),
            None => pdf::read(path, &bytes),
        },
        Format::Markdown => Ok(plain::read(&text_of(&bytes), true)),
        Format::Text => Ok(plain::read(&text_of(&bytes), false)),
    }
    .with_context(|| crate::lang::f("{} okunamadı", "Couldn't read {}", &[&path.display()]))?;
    src.paras = src
        .paras
        .into_iter()
        .map(Para::tidy)
        .filter(|p| !p.runs.is_empty())
        .collect();
    let blank = |s: &Option<String>| s.as_deref().is_none_or(|s| s.trim().is_empty());
    if blank(&src.title) {
        src.title = None;
    }
    if blank(&src.author) {
        src.author = None;
    }
    src.plays = pages::volume(&src.paras);
    // A PDF says neither; its file often does: "Yazar - Oyun.pdf", and in a
    // volume whichever side is one of its plays is the title.
    if format == Format::Pdf
        && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
        && let stem = stem.replace('_', " ")
        && let Some((author, title)) = stem.split_once(" - ")
    {
        let (author, title) = (author.trim(), title.trim());
        let names_play = |t: &str| {
            src.plays
                .iter()
                .any(|p| shape::fold(&p.title) == shape::fold(t))
        };
        let (author, title) = if names_play(author) && !names_play(title) {
            (title, author)
        } else {
            (author, title)
        };
        src.title = Some(title.to_string());
        src.author = Some(author.to_string());
    }
    Ok(src)
}

/// A text file's text: UTF-8, or else the Turkish Windows code page that
/// older files come in.
fn text_of(bytes: &[u8]) -> String {
    let bytes = bytes.strip_prefix(b"\xef\xbb\xbf").unwrap_or(bytes);
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => bytes.iter().map(|&b| windows_1254(b)).collect(),
    }
}

fn windows_1254(b: u8) -> char {
    match b {
        0xd0 => 'Ğ',
        0xdd => 'İ',
        0xde => 'Ş',
        0xf0 => 'ğ',
        0xfd => 'ı',
        0xfe => 'ş',
        0x80 => '€',
        0x85 => '…',
        0x91 => '‘',
        0x92 => '’',
        0x93 => '“',
        0x94 => '”',
        0x96 => '–',
        0x97 => '—',
        b => b as char,
    }
}

/// A table row as paragraphs: two cells, a short name and the words, as
/// `NAME: words` (scripts are often set out so); anything else cell by cell.
fn table_row(cells: Vec<Vec<Para>>) -> Vec<Para> {
    if let [name, words] = cells.as_slice()
        && let [who] = name.as_slice()
        && !words.is_empty()
    {
        let n = who.text();
        let n = n.trim();
        if !n.is_empty() && n.chars().count() <= 40 && !n.contains(':') {
            let mut p = Para {
                runs: who.runs.clone(),
                ..Para::default()
            };
            p.runs.push(Run {
                text: ": ".into(),
                ..Run::default()
            });
            for (i, w) in words.iter().enumerate() {
                if i > 0 {
                    p.runs.push(Run {
                        text: "\n".into(),
                        ..Run::default()
                    });
                }
                p.runs.extend(w.runs.iter().cloned());
            }
            return vec![p];
        }
    }
    cells.into_iter().flatten().collect()
}

/// A zip member as text, if it is there.
fn member(zip: &mut zip::ZipArchive<std::io::Cursor<&[u8]>>, name: &str) -> Result<Option<String>> {
    use std::io::Read;
    let mut f = match zip.by_name(name) {
        Ok(f) => f,
        Err(zip::result::ZipError::FileNotFound) => return Ok(None),
        Err(e) => {
            return Err(e)
                .with_context(|| crate::lang::f("{} açılamadı", "Couldn't open {}", &[&name]));
        }
    };
    let mut s = String::new();
    f.read_to_string(&mut s)
        .with_context(|| crate::lang::f("{} okunamadı", "Couldn't read {}", &[&name]))?;
    Ok(Some(s))
}

fn open_zip(bytes: &[u8]) -> Result<zip::ZipArchive<std::io::Cursor<&[u8]>>> {
    zip::ZipArchive::new(std::io::Cursor::new(bytes)).context(crate::lang::l(
        "bozuk ya da şifreli bir belge",
        "a broken or encrypted document",
    ))
}
