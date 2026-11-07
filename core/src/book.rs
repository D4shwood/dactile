//! A book of plays: a file of its own for its title, and anything that
//! stands before the plays, with the plays under it in the list the way
//! drafts are. Its PDF is one document: a cover, the contents, the parts.

use crate::buffer::escape_pasted;
use crate::build::{self, outside};
use crate::import::{plain, shape};
use crate::lang::{self, Lang};
use crate::library;
use crate::sidecar::Sidecar;
use anyhow::{Context, Result, bail};
use std::fs;
use std::path::{Path, PathBuf};

/// A book's own file in `dir`, under its title. `front`, what stands before
/// the plays, is its text.
pub fn make(dir: &Path, title: &str, author: &str, front: &str, lang: Lang) -> Result<PathBuf> {
    let mut d = shape::shape(&plain::read(front, false), lang);
    // An introduction has no speakers, whatever it has before a colon.
    for p in d.people.iter_mut() {
        p.not_character = true;
    }
    d.title = title.trim().to_string();
    d.author = author.trim().to_string();
    fs::create_dir_all(dir)?;
    let path = library::free_path(dir, &library::file_name(title));
    crate::file::write_atomic(&path, &d.tex())?;
    let mut side = Sidecar::load(&path)?;
    side.book = true;
    side.save(&path)?;
    Ok(path)
}

/// This play is part `at` of the book, under it in the list.
pub fn join(book: &Path, play: &Path, at: usize) -> Result<()> {
    let Some(name) = book.file_name() else {
        bail!("{}: no file name", book.display());
    };
    let mut side = Sidecar::load(play)?;
    side.parent = Some(name.to_string_lossy().to_string());
    side.book_at = Some(at);
    side.save(play)
}

/// The book's plays, in their order.
pub fn parts(book: &Path) -> Vec<PathBuf> {
    let Some(name) = book.file_name().map(|n| n.to_string_lossy().to_string()) else {
        return Vec::new();
    };
    let dir = book.parent().unwrap_or(Path::new("."));
    let mut found: Vec<(usize, PathBuf)> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "tex"))
        .filter_map(|p| {
            let side = Sidecar::load(&p).ok()?;
            (side.parent.as_deref() == Some(&name)).then(|| (side.book_at.unwrap_or(usize::MAX), p))
        })
        .collect();
    found.sort();
    found.into_iter().map(|(_, p)| p).collect()
}

/// Whether the book's own file has text of its own, past its cover.
fn has_text(tex: &str) -> bool {
    let body = tex.split("\\begin{document}").nth(1).unwrap_or("");
    let body = body.split("\\end{document}").next().unwrap_or("");
    body.lines()
        .map(str::trim)
        .any(|l| !l.starts_with(['\\', '%', '#']) && l.chars().any(char::is_alphabetic))
}

/// The book's one document in TeX: `parts` are its titles and PDFs' names.
fn master(title: &str, author: &str, parts: &[(String, String)], lang: Lang) -> String {
    let esc = |t: &str| escape_pasted(t, false);
    let contents = match lang {
        Lang::Tr => "İçindekiler",
        Lang::En => "Contents",
    };
    let mut t = format!(
        "\\IfFileExists{{lmroman12-regular.otf}}{{}}{{\\DeclareFontFamily{{TU}}{{lmr}}{{}}\
         \\DeclareFontShape{{TU}}{{lmr}}{{m}}{{n}}{{<->\"Liberation Serif\"}}{{}}\
         \\DeclareFontShape{{TU}}{{lmr}}{{m}}{{it}}{{<->\"Liberation Serif/I\"}}{{}}\
         \\DeclareFontShape{{TU}}{{lmr}}{{bx}}{{n}}{{<->\"Liberation Serif/B\"}}{{}}\
         \\DeclareFontShape{{TU}}{{lmr}}{{bx}}{{it}}{{<->\"Liberation Serif/BI\"}}{{}}}}\n\
         \\documentclass[12pt,a4paper]{{article}}\n\
         \\usepackage{{fontspec}}\n\\IfFontExistsTF{{Times New Roman}}{{\\setmainfont{{Times New Roman}}}}{{\\setmainfont{{Liberation Serif}}}}\n\\usepackage{{pdfpages}}\n\
         \\pagestyle{{empty}}\n\\renewcommand\\contentsname{{{contents}}}\n\
         \\begin{{document}}\n\\begin{{titlepage}}\\centering\n\\vspace*{{.3\\textheight}}\n\
         {{\\Huge\\bfseries {}\\par}}\n\\vspace{{1.5em}}\n{{\\Large {}\\par}}\n\\end{{titlepage}}\n\
         \\setcounter{{page}}{{2}}\n\\renewcommand\\numberline[1]{{}}\n\\tableofcontents\n\\thispagestyle{{empty}}\n\\clearpage\n",
        esc(title),
        esc(author)
    );
    for (i, (name, pdf)) in parts.iter().enumerate() {
        t.push_str(&format!(
            "\\includepdf[pages=-,addtotoc={{1,section,1,{{{}}},part{i}}}]{{{pdf}}}\n",
            esc(name)
        ));
    }
    t.push_str("\\end{document}\n");
    t
}

/// The book as one PDF beside its file: `<book>-kitap.pdf`. Each play is
/// built as it is on disk; where one stops, the book says which.
pub fn pdf(book: &Path, lang: Lang) -> Result<PathBuf> {
    let plays = parts(book);
    if plays.is_empty() {
        bail!(
            "{}",
            lang::l("Bu kitapta oyun yok.", "This book has no plays.")
        );
    }
    let read = |p: &Path| library::read(p, 150).with_context(|| p.display().to_string());
    let own = read(book)?;
    let text = fs::read_to_string(book)?;
    let mut sources: Vec<(String, PathBuf)> = Vec::new();
    if has_text(&text) {
        sources.push((
            lang::l("Giriş", "Introduction").to_string(),
            book.to_path_buf(),
        ));
    }
    for p in &plays {
        sources.push((read(p)?.title, p.clone()));
    }
    let work = std::env::temp_dir().join(format!("dactile-book-{}", std::process::id()));
    fs::create_dir_all(&work)?;
    let done = (|| -> Result<PathBuf> {
        let mut parts = Vec::new();
        for (i, (title, play)) in sources.iter().enumerate() {
            let built =
                build::build(play).map_err(|e| anyhow::anyhow!("{}: {}", title, e.message))?;
            let name = format!("part{i}.pdf");
            fs::copy(&built.pdf, work.join(&name))?;
            parts.push((title.clone(), name));
        }
        fs::write(
            work.join("book.tex"),
            master(&own.title, &own.author, &parts, lang),
        )?;
        for _ in 0..2 {
            let run = outside("xelatex")
                .args(["-interaction=nonstopmode", "-halt-on-error", "book.tex"])
                .current_dir(&work)
                .output()
                .context("xelatex")?;
            if !run.status.success() {
                let log = String::from_utf8_lossy(&run.stdout);
                let why = log.lines().find(|l| l.starts_with('!')).unwrap_or("");
                bail!(
                    "{} {why}",
                    lang::l("Kitap derlenemedi:", "The book didn't build:")
                );
            }
        }
        let stem = book.file_stem().unwrap_or_default().to_string_lossy();
        let out = book.with_file_name(format!("{stem}-kitap.pdf"));
        fs::copy(work.join("book.pdf"), &out)?;
        Ok(out)
    })();
    let _ = fs::remove_dir_all(&work);
    done
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folder(name: &str) -> PathBuf {
        let d =
            std::env::temp_dir().join(format!("dactile-book-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn a_books_parts_come_in_their_order() {
        let d = folder("parts");
        let book = make(&d, "Toplu Oyunlar", "Ada Yazar", "", Lang::Tr).unwrap();
        assert!(library::read(&book, 150).unwrap().book);
        for (at, name) in [(1, "b.tex"), (0, "a.tex")] {
            let p = d.join(name);
            fs::write(&p, library::template(name, "Ada Yazar", Lang::Tr)).unwrap();
            join(&book, &p, at).unwrap();
        }
        let names: Vec<String> = parts(&book)
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert_eq!(names, ["a.tex", "b.tex"]);
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn a_book_has_text_only_when_it_is_given_some() {
        let d = folder("text");
        let bare = make(&d, "Boş", "", "", Lang::Tr).unwrap();
        assert!(!has_text(&fs::read_to_string(bare).unwrap()));
        let full = make(&d, "Dolu", "", "Bir giriş yazısı.\n\nKronoloji.", Lang::Tr).unwrap();
        assert!(has_text(&fs::read_to_string(full).unwrap()));
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn the_master_lists_each_part_for_the_contents() {
        let parts = vec![("Yol & Kar".to_string(), "part0.pdf".to_string())];
        let t = master("Toplu", "Ada", &parts, Lang::Tr);
        assert!(
            t.contains("addtotoc={1,section,1,{Yol \\& Kar},part0}"),
            "{t}"
        );
        assert!(t.contains("İçindekiler") && t.contains("\\tableofcontents"));
    }
}
