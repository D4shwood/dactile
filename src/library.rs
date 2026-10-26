//! The plays the home screen shows, and new plays.

use crate::cast::Cast;
use crate::config::Config;
use crate::info;
use crate::lang::Lang;
use crate::lex;
use crate::outline::Outline;
use crate::source;
use crate::timing;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Clone)]
pub struct Play {
    pub path: PathBuf,
    pub title: String,
    pub author: String,
    pub acts: usize,
    pub scenes: usize,
    pub secs: u32,
    pub modified: Option<SystemTime>,
    /// Only in the recent list, not in the plays folder.
    pub elsewhere: bool,
}

/// Whether a file is a play for the class.
pub fn is_play(text: &str) -> bool {
    text.contains("{tiyatro}")
}

pub fn read(path: &Path, wpm: u32) -> Option<Play> {
    let text = std::fs::read_to_string(path).ok()?;
    if !is_play(&text) {
        return None;
    }
    let cast = Cast::of_play(&text);
    let blocks = lex::blocks(&text, source::split(&text).body, &cast);
    let o = Outline::of(&text, &blocks, &cast);
    let title = info::get(&text, "title")
        .map(|t| t.split("\\\\").next().unwrap_or("").trim().to_string())
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| {
            path.file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default()
        });
    Some(Play {
        path: path.to_path_buf(),
        title,
        author: info::get(&text, "author").unwrap_or_default(),
        acts: o.acts.len(),
        scenes: o.scenes.len(),
        secs: o.scenes.iter().map(|s| timing::scene_secs(s, wpm)).sum(),
        modified: crate::file::modified(path),
        elsewhere: false,
    })
}

/// The plays folder (one level of subfolders too) and the recent ones,
/// most recently changed first.
pub fn list(cfg: &Config) -> Vec<Play> {
    let mut paths: Vec<PathBuf> = Vec::new();
    let dir = cfg.plays_dir();
    for e in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
        let p = e.path();
        if p.is_dir()
            && !p
                .file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with('.'))
        {
            for f in std::fs::read_dir(&p).into_iter().flatten().flatten() {
                paths.push(f.path());
            }
        } else {
            paths.push(p);
        }
    }
    paths.retain(|p| p.extension().is_some_and(|x| x == "tex"));
    let mut out: Vec<Play> = paths.iter().filter_map(|p| read(p, cfg.wpm)).collect();
    for r in &cfg.recent {
        let p = PathBuf::from(r);
        let canon = |x: &Path| x.canonicalize().unwrap_or_else(|_| x.to_path_buf());
        if out.iter().any(|x| canon(&x.path) == canon(&p)) {
            continue;
        }
        if let Some(mut play) = read(&p, cfg.wpm) {
            play.elsewhere = true;
            out.push(play);
        }
    }
    out.sort_by_key(|p| std::cmp::Reverse(p.modified));
    out
}

/// A file name for a title, in plain letters (TeX and every system take
/// them): "Kar Masalı" → "kar-masali.tex".
pub fn file_name(title: &str) -> String {
    let words: Vec<String> = title
        .split_whitespace()
        .map(crate::cast::fold_ascii)
        .filter(|w| !w.is_empty())
        .collect();
    let s = words.join("-");
    format!("{}.tex", if s.is_empty() { "oyun".to_string() } else { s })
}

/// A new play: the class, the cover, one act and one scene, and three lines
/// of help as comments.
pub fn template(title: &str, author: &str, lang: Lang) -> String {
    let (help, words, act, place, first) = match lang {
        Lang::Tr => (
            "% Dactile ile yazılıyor. \\ yazınca öneriler açılır; \\kisaltma ve boşluk bir\n\
             % replik başlatır, \\kisaltmado bir yönerge. F1 bütün tuşları gösterir.\n\
             % Derlemek için: xelatex iki kez.\n",
            "",
            "Birinci Perde",
            "İç — Gece",
            "\\action{Işıklar yavaşça açılır.}",
        ),
        Lang::En => (
            "% Written with Dactile. A backslash opens suggestions; \\alias and a space\n\
             % starts a line, \\aliasdo a direction. F1 shows every key.\n\
             % To compile: xelatex twice.\n",
            "\\renewcommand{\\casttitlename}{CAST}\n\\renewcommand{\\acttitlename}{Act}\n\
             \\renewcommand{\\scenetitlename}{Scene}\n\\renewcommand{\\curtainlocalizedtext}{Curtain}\n\
             \\renewcommand{\\scriptendlocalizedtext}{The End}\n\\renewcommand{\\draftlocalizedtext}{DRAFT}\n\
             \\renewcommand{\\beattext}{Pause.}\n\n",
            "Act One",
            "Interior — Night",
            "\\action{The lights come up slowly.}",
        ),
    };
    format!(
        "{help}\n\\documentclass[draft]{{tiyatro}}\n\n\\title{{{title}}}\n\\author{{{author}}}\n\\draft{{0.1}}\n\n{words}\
         \\begin{{document}}\n\n\\maketitle\n\\kisiler\n\n\\perde[{act}]\n\n\\sahne[{place}]\n\n{first}\n\n\\end{{document}}\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::Doc;

    #[test]
    fn new_play() {
        let t = template("Kar Masalı", "Örnek Yazar", Lang::Tr);
        let doc = Doc::new(&t);
        assert_eq!(doc.title(&t).as_deref(), Some("Kar Masalı"));
        assert_eq!(doc.outline.acts.len(), 1);
        assert_eq!(doc.outline.scenes.len(), 1);
        assert!(is_play(&t));
        let t = template("Snow", "A. Writer", Lang::En);
        assert!(t.contains("\\renewcommand{\\acttitlename}{Act}"));
        assert_eq!(Doc::new(&t).outline.scenes[0].place, "Interior — Night");
        assert_eq!(file_name("Kar Masalı"), "kar-masali.tex");
        assert_eq!(file_name("  "), "oyun.tex");
    }
}
