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
    /// Characters declared, and words spoken.
    pub characters: usize,
    pub words: usize,
    /// `\draft{…}`, and whether the class is in final mode.
    pub draft: Option<String>,
    pub final_mode: bool,
    /// The starred characters, as the cover names them.
    pub starred: Vec<String>,
    pub modified: Option<SystemTime>,
    /// Only in the recent list, not in the plays folder.
    pub elsewhere: bool,
    /// The play this one is a draft of.
    pub parent: Option<PathBuf>,
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
        characters: cast.characters.len(),
        words: o.scenes.iter().map(|s| s.words).sum(),
        draft: info::get(&text, "draft").filter(|d| !d.trim().is_empty()),
        final_mode: info::is_final(&text),
        starred: cast
            .characters
            .iter()
            .filter(|c| c.star)
            .map(|c| c.display_name().to_string())
            .collect(),
        modified: crate::file::modified(path),
        elsewhere: false,
        parent: crate::sidecar::Sidecar::load(path)
            .ok()
            .and_then(|s| s.parent)
            .map(|name| path.with_file_name(name)),
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
    // Drafts made before the link was kept: `oyun-0.2.tex` of the same
    // title next to `oyun.tex`.
    for i in 0..out.len() {
        if out[i].parent.is_some() {
            continue;
        }
        let stem = out[i]
            .path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        let base = draft_base(&stem);
        if base == stem {
            continue;
        }
        let up = out[i].path.with_file_name(format!("{base}.tex"));
        if out.iter().any(|p| p.path == up && p.title == out[i].title) {
            out[i].parent = Some(up);
        }
    }
    out
}

/// A file name's stem without a draft number at its end:
/// `kar-masali-0.2` → `kar-masali`.
pub fn draft_base(stem: &str) -> &str {
    stem.rsplit_once('-')
        .filter(|(b, tail)| {
            !b.is_empty()
                && !tail.is_empty()
                && tail.chars().all(|c| c.is_ascii_digit() || c == '.')
        })
        .map_or(stem, |(b, _)| b)
}

/// A draft number as a file name ends with it: letters, digits and dots
/// stay, the rest becomes a dash (`İkinci el` → `ikinci-el`).
pub fn draft_tag(draft: &str) -> String {
    let tag: String = draft
        .trim()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' {
                c.to_ascii_lowercase().to_string()
            } else {
                let f = crate::cast::fold_ascii(&c.to_string());
                if f.is_empty() { "-".into() } else { f }
            }
        })
        .collect();
    tag.trim_matches('-').to_string()
}

/// `kar-masali.tex` with draft 0.2 → `kar-masali-0.2.tex`; from
/// `kar-masali-0.2.tex`, 0.3 → `kar-masali-0.3.tex`.
pub fn fork_name(src: &Path, draft: &str) -> String {
    let stem = src
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    // Without an earlier draft's number at its end.
    let base = draft_base(&stem).to_string();
    let draft = draft_tag(draft);
    if draft.is_empty() {
        format!("{base}-2.tex")
    } else {
        format!("{base}-{draft}.tex")
    }
}

/// The play as it is now, kept as a draft of it next to it: the same text
/// under `play-0.2.tex` (its draft number), its cards and words along.
pub fn keep_draft(play: &Path, text: &str) -> anyhow::Result<PathBuf> {
    let draft = info::get(text, "draft").unwrap_or_default();
    let dir = play.parent().map(Path::to_path_buf).unwrap_or_default();
    let path = free_path(&dir, &fork_name(play, &crate::tex::unescape_text(&draft)));
    crate::file::write_atomic(&path, text)?;
    let mut side = crate::sidecar::Sidecar::load(play).unwrap_or_default();
    side.parent = play.file_name().map(|n| n.to_string_lossy().to_string());
    side.save(&path)?;
    Ok(path)
}

/// The plays in the order of their tree, growing upward: each play's drafts
/// above it, the newest on top, and the families most recently changed
/// first. With each, whether it and every play it comes from is the topmost
/// of its siblings (empty for a play of its own).
pub fn tree(plays: Vec<Play>) -> Vec<(Play, Vec<bool>)> {
    let n = plays.len();
    let index = |p: &Path| plays.iter().position(|x| x.path == p);
    let mut up: Vec<Option<usize>> = plays
        .iter()
        .map(|p| p.parent.as_deref().and_then(index))
        .collect();
    // A loop of links (a draft of its own draft) breaks where it is found.
    for i in 0..n {
        let (mut at, mut steps) = (i, 0);
        while let Some(u) = up[at] {
            steps += 1;
            if u == i || steps > n {
                up[i] = None;
                break;
            }
            at = u;
        }
    }
    let mut kids: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (i, u) in up.iter().enumerate() {
        if let Some(u) = *u {
            kids[u].push(i);
        }
    }
    // The newest change in a play's family.
    fn newest(i: usize, plays: &[Play], kids: &[Vec<usize>]) -> Option<SystemTime> {
        kids[i]
            .iter()
            .map(|&k| newest(k, plays, kids))
            .fold(plays[i].modified, |a, b| a.max(b))
    }
    let stamp: Vec<Option<SystemTime>> = (0..n).map(|i| newest(i, &plays, &kids)).collect();
    let by_newest = |v: &mut Vec<usize>| v.sort_by_key(|&i| std::cmp::Reverse(stamp[i]));
    for k in kids.iter_mut() {
        by_newest(k);
    }
    let mut roots: Vec<usize> = (0..n).filter(|&i| up[i].is_none()).collect();
    by_newest(&mut roots);
    let mut order: Vec<(usize, Vec<bool>)> = Vec::new();
    fn walk(i: usize, firsts: Vec<bool>, kids: &[Vec<usize>], out: &mut Vec<(usize, Vec<bool>)>) {
        for (j, &k) in kids[i].iter().enumerate() {
            let mut l = firsts.clone();
            l.push(j == 0);
            walk(k, l, kids, out);
        }
        out.push((i, firsts));
    }
    for r in roots {
        walk(r, Vec::new(), &kids, &mut order);
    }
    let mut slots: Vec<Option<Play>> = plays.into_iter().map(Some).collect();
    order
        .into_iter()
        .filter_map(|(i, l)| slots[i].take().map(|p| (p, l)))
        .collect()
}

/// The files that go with a play: its notes, its scene trash, the PDF and
/// what XeLaTeX leaves beside it.
pub fn companions(play: &Path) -> Vec<PathBuf> {
    let mut out = vec![
        crate::sidecar::path_for(play),
        crate::shelf::trash_path(play),
    ];
    for ext in OUTPUTS {
        out.push(play.with_extension(ext));
    }
    out.retain(|p| p.is_file());
    out
}

const OUTPUTS: [&str; 6] = ["pdf", "aux", "log", "toc", "out", "synctex.gz"];

/// Moves a play and the files that go with it to `to`, next to it; its
/// history and its drafts' links follow.
pub fn rename(from: &Path, to: &Path, drafts: &[PathBuf]) -> anyhow::Result<()> {
    if to.exists() {
        anyhow::bail!("{} exists", to.display());
    }
    let old_history = crate::file::history_dir(from);
    let side = companions(from);
    std::fs::rename(from, to)?;
    let lock = crate::file::lock_path(from);
    if lock.is_file() {
        let _ = std::fs::remove_file(lock);
    }
    for f in side {
        let name = f.to_string_lossy();
        let new = if f == crate::sidecar::path_for(from) {
            crate::sidecar::path_for(to)
        } else if f == crate::shelf::trash_path(from) {
            crate::shelf::trash_path(to)
        } else {
            let ext = OUTPUTS
                .iter()
                .find(|e| name.ends_with(&format!(".{e}")))
                .copied()
                .unwrap_or("pdf");
            to.with_extension(ext)
        };
        if !new.exists() {
            let _ = std::fs::rename(&f, &new);
        }
    }
    let new_history = crate::file::history_dir(to);
    if old_history.is_dir() && !new_history.exists() {
        let _ = std::fs::rename(&old_history, &new_history);
    }
    let name = to
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    for d in drafts {
        if let Ok(mut s) = crate::sidecar::Sidecar::load(d) {
            s.parent = Some(name.clone());
            let _ = s.save(d);
        }
    }
    Ok(())
}

/// The play and the files that go with it, to the desktop's trash.
pub fn throw_away(play: &Path, bin: &Path) -> anyhow::Result<Vec<crate::trash::Moved>> {
    let lock = crate::file::lock_path(play);
    if lock.is_file() {
        let _ = std::fs::remove_file(lock);
    }
    let mut all = vec![play.to_path_buf()];
    all.extend(companions(play));
    crate::trash::throw(&all, bin)
}

/// A file name for a title, in plain letters (TeX and every system take
/// them): "Kar Masalı" → "kar-masali.tex".
pub fn file_name(title: &str) -> String {
    let words: Vec<String> = title
        .split_whitespace()
        .map(|w| {
            w.chars()
                .map(|c| {
                    if c.is_ascii_alphanumeric() {
                        c.to_ascii_lowercase().to_string()
                    } else {
                        crate::cast::fold_ascii(&c.to_string())
                    }
                })
                .collect::<String>()
        })
        .filter(|w| !w.is_empty())
        .collect();
    let s = words.join("-");
    format!("{}.tex", if s.is_empty() { "oyun".to_string() } else { s })
}

/// Another play made from `text`: a copy under a new title, or a new draft
/// of the same play. Only the cover changes; the values are as typed in a
/// form and get escaped here.
pub fn derive(text: &str, title: Option<&str>, draft: Option<&str>) -> String {
    let mut t = text.to_string();
    for (cmd, v) in [("title", title), ("draft", draft)] {
        if let Some(v) = v
            && let Some(e) = info::set(&t, cmd, &crate::tex::escape_text(v))
        {
            t = crate::tex::apply(&t, &[e]);
        }
    }
    t
}

/// The draft number after `d`: 0.1 → 0.2, 1.9 → 1.10, 3 → 4; a date
/// (02.08.2024) becomes today's; anything else gets " (2)".
pub fn next_draft(d: &str, today: &str) -> String {
    let d = d.trim();
    if d.is_empty() {
        return "0.2".into();
    }
    let parts: Vec<&str> = d.split('.').collect();
    let date =
        parts.len() == 3 && parts.iter().all(|p| p.parse::<u32>().is_ok()) && parts[2].len() == 4;
    if date {
        return today.to_string();
    }
    if let Some(last) = parts.last()
        && let Ok(n) = last.parse::<u32>()
    {
        let mut out: Vec<String> = parts[..parts.len() - 1]
            .iter()
            .map(|p| p.to_string())
            .collect();
        out.push((n + 1).to_string());
        return out.join(".");
    }
    format!("{d} (2)")
}

/// A file name next to `path` that is not taken: `name.tex`, `name-2.tex`…
pub fn free_path(dir: &Path, name: &str) -> PathBuf {
    let stem = name.strip_suffix(".tex").unwrap_or(name);
    let mut p = dir.join(format!("{stem}.tex"));
    let mut n = 2;
    while p.exists() {
        p = dir.join(format!("{stem}-{n}.tex"));
        n += 1;
    }
    p
}

/// A new play: the class, the cover, one act and one scene, and three lines
/// of help as comments.
pub fn template(title: &str, author: &str, lang: Lang) -> String {
    let title = crate::tex::escape_text(title);
    let author = crate::tex::escape_text(author);
    let (help, words, act, place, first, kw) = match lang {
        Lang::Tr => (
            "% Dactile ile yazılıyor. \\ yazınca öneriler açılır; \\kisaltma ve boşluk bir\n\
             % replik başlatır, \\kisaltmado bir yönerge; satır başında # başlıklar\n\
             % (#sahne, #perde, #önsöz, #notlar). F1 bütün tuşları gösterir.\n\
             % PDF: F5 (ya da elle xelatex; Dactile dışında yazılınca iki kez).\n",
            "",
            "Birinci Perde",
            "İç — Gece",
            "\\action{Işıklar yavaşça açılır.}",
            ("perde", "sahne"),
        ),
        Lang::En => (
            "% Written with Dactile. A backslash opens suggestions; \\alias and a space\n\
             % starts a line, \\aliasdo a direction; # at a line's start a heading\n\
             % (#scene, #act, #preface, #notes). F1 shows every key.\n\
             % PDF: F5 (or xelatex by hand; twice after editing outside Dactile).\n",
            "\\renewcommand{\\casttitlename}{CAST}\n\\renewcommand{\\acttitlename}{Act}\n\
             \\renewcommand{\\scenetitlename}{Scene}\n\\renewcommand{\\curtainlocalizedtext}{Curtain}\n\
             \\renewcommand{\\scriptendlocalizedtext}{The End}\n\\renewcommand{\\draftlocalizedtext}{DRAFT}\n\
             \\renewcommand{\\beattext}{Pause.}\n\\renewcommand{\\prefacename}{PREFACE}\n\
             \\renewcommand{\\notesname}{NOTES FOR THE DIRECTOR}\n\n",
            "Act One",
            "Interior — Night",
            "\\action{The lights come up slowly.}",
            ("act", "scene"),
        ),
    };
    format!(
        "{help}\n\\documentclass[draft]{{tiyatro}}\n\n\\title{{{title}}}\n\\author{{{author}}}\n\\draft{{0.1}}\n\n{words}\
         \\begin{{document}}\n\n\\maketitle\n\\kisiler\n\n#{} {act}\n\n#{} {place}\n\n{first}\n\n\\end{{document}}\n",
        kw.0, kw.1
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::Doc;

    #[test]
    fn copies_and_drafts() {
        let play = include_str!("../examples/ornek-oyun.tex");
        let copy = derive(play, Some("Kar & Masal"), None);
        assert_eq!(info::get(&copy, "title").as_deref(), Some("Kar \\& Masal"));
        assert_eq!(info::get(&copy, "draft").as_deref(), Some("0.1"));
        let fork = derive(play, None, Some("0.2"));
        assert_eq!(info::get(&fork, "title").as_deref(), Some("Kar Masalı"));
        assert_eq!(info::get(&fork, "draft").as_deref(), Some("0.2"));
        // Only the cover line changed.
        assert_eq!(fork.lines().count(), play.lines().count());
        assert_eq!(next_draft("0.1", "26.10.2026"), "0.2");
        assert_eq!(next_draft("1.9", "26.10.2026"), "1.10");
        assert_eq!(next_draft("3", "26.10.2026"), "4");
        assert_eq!(next_draft("02.08.2024", "26.10.2026"), "26.10.2026");
        assert_eq!(next_draft("ilk", "26.10.2026"), "ilk (2)");
        assert_eq!(next_draft("", "26.10.2026"), "0.2");
    }

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
        assert_eq!(file_name("Kar Masalı 2"), "kar-masali-2.tex");
        assert_eq!(file_name("Çay & Şeker"), "cay-seker.tex");
    }

    fn play(dir: &Path, name: &str, title: &str, age: u64) -> Play {
        Play {
            path: dir.join(name),
            title: title.into(),
            author: String::new(),
            acts: 0,
            scenes: 0,
            secs: 0,
            characters: 0,
            words: 0,
            draft: None,
            final_mode: false,
            starred: Vec::new(),
            modified: Some(SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1000 - age)),
            elsewhere: false,
            parent: None,
        }
    }

    #[test]
    fn drafts_stack_above_their_play() {
        let d = Path::new("/x");
        let mut a2 = play(d, "a-0.2.tex", "A", 1);
        a2.parent = Some(d.join("a.tex"));
        let mut a3 = play(d, "a-0.3.tex", "A", 5);
        a3.parent = Some(d.join("a.tex"));
        let mut a21 = play(d, "a-0.2.1.tex", "A", 2);
        a21.parent = Some(d.join("a-0.2.tex"));
        // A loop of links, and a parent gone.
        let mut l1 = play(d, "l1.tex", "L", 8);
        l1.parent = Some(d.join("l2.tex"));
        let mut l2 = play(d, "l2.tex", "L", 9);
        l2.parent = Some(d.join("l1.tex"));
        let mut lost = play(d, "lost-2.tex", "Z", 7);
        lost.parent = Some(d.join("gone.tex"));
        let all = vec![
            play(d, "b.tex", "B", 3),
            a3,
            l1,
            play(d, "a.tex", "A", 10),
            a21,
            lost,
            l2,
            a2,
        ];
        let t: Vec<(String, Vec<bool>)> = tree(all)
            .into_iter()
            .map(|(p, l)| (p.path.file_name().unwrap().to_string_lossy().into(), l))
            .collect();
        let names: Vec<&str> = t.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(
            names,
            [
                "a-0.2.1.tex",
                "a-0.2.tex",
                "a-0.3.tex",
                "a.tex",
                "b.tex",
                "lost-2.tex",
                "l2.tex",
                "l1.tex"
            ]
        );
        assert_eq!(t[0].1, [true, true]);
        assert_eq!(t[1].1, [true]);
        assert_eq!(t[2].1, [false]);
        assert!(t[3].1.is_empty() && t[4].1.is_empty());
        // The loop broke somewhere: one of the two is a root.
        assert!(t[7].1.is_empty());
        assert_eq!(draft_base("kar-masali-0.2"), "kar-masali");
        assert_eq!(draft_base("kar-masali"), "kar-masali");
        assert_eq!(draft_base("-2"), "-2");
    }

    #[test]
    fn renaming_and_trashing_take_everything() {
        let d = std::env::temp_dir().join(format!("dactile-test-lib-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        let a = d.join("a.tex");
        std::fs::write(&a, "x").unwrap();
        std::fs::write(d.join("a.pdf"), "pdf").unwrap();
        std::fs::write(d.join("a.synctex.gz"), "s").unwrap();
        std::fs::write(crate::shelf::trash_path(&a), "cop").unwrap();
        let mut side = crate::sidecar::Sidecar::default();
        side.words.push("söz".into());
        side.save(&a).unwrap();
        let draft = d.join("a-0.2.tex");
        std::fs::write(&draft, "y").unwrap();
        let b = d.join("b.tex");
        rename(&a, &b, std::slice::from_ref(&draft)).unwrap();
        assert!(!a.exists() && b.exists());
        assert_eq!(std::fs::read_to_string(d.join("b.pdf")).unwrap(), "pdf");
        assert!(d.join("b.synctex.gz").exists());
        assert!(crate::shelf::trash_path(&b).exists());
        assert_eq!(crate::sidecar::Sidecar::load(&b).unwrap().words, ["söz"]);
        assert_eq!(
            crate::sidecar::Sidecar::load(&draft)
                .unwrap()
                .parent
                .as_deref(),
            Some("b.tex")
        );
        assert!(rename(&b, &draft, &[]).is_err());
        let bin = d.join("Trash");
        let moved = throw_away(&b, &bin).unwrap();
        assert_eq!(moved.len(), 5);
        assert!(std::fs::read_dir(&d).unwrap().flatten().all(|e| {
            let n = e.file_name().to_string_lossy().to_string();
            n == "Trash" || n.starts_with("a-0.2")
        }));
        crate::trash::restore(&moved).unwrap();
        assert!(b.exists() && d.join("b.pdf").exists());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn kept_as_a_draft() {
        let d = std::env::temp_dir().join(format!("dactile-keep-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let play = d.join("kar.tex");
        let text = "\\title{Kar}\n\\draft{0.2}\n\\begin{document}\n\\end{document}\n";
        std::fs::write(&play, text).unwrap();
        let mut side = crate::sidecar::Sidecar::default();
        side.summaries.insert("k1".into(), "Kar yağar.".into());
        side.save(&play).unwrap();
        let kept = keep_draft(&play, text).unwrap();
        assert_eq!(kept, d.join("kar-0.2.tex"));
        assert_eq!(std::fs::read_to_string(&kept).unwrap(), text);
        let s = crate::sidecar::Sidecar::load(&kept).unwrap();
        assert_eq!(s.parent.as_deref(), Some("kar.tex"));
        assert_eq!(s.summaries["k1"], "Kar yağar.");
        // Once more: a name of its own.
        assert_eq!(keep_draft(&play, text).unwrap(), d.join("kar-0.2-2.tex"));
        let _ = std::fs::remove_dir_all(&d);
    }
}
