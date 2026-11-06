//! Where a play's things are: its folder, its PDF, the actors' copies, the
//! role table and its drafts. And whether a place is one the system empties.

use crate::library::{self, Play};
use crate::{build, casting, file, lang, timing};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// A file a play made.
#[derive(Debug, Clone, PartialEq)]
pub struct Made {
    pub path: PathBuf,
    pub modified: Option<SystemTime>,
}

/// The actors' copies.
#[derive(Debug, Clone, PartialEq)]
pub struct Copies {
    pub path: PathBuf,
    pub pdfs: usize,
    /// Of the newest PDF: a copy built again leaves the folder's own time.
    pub modified: Option<SystemTime>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Draft {
    pub path: PathBuf,
    pub title: String,
    /// `\draft{…}`.
    pub draft: Option<String>,
    pub modified: Option<SystemTime>,
}

/// What belongs to a play; what isn't there yet is `None` or empty.
#[derive(Debug, Clone, PartialEq)]
pub struct Outputs {
    pub folder: PathBuf,
    pub pdf: Option<Made>,
    pub copies: Option<Copies>,
    /// The role table's PDF (`oyun-tablo.pdf`).
    pub table: Option<Made>,
    /// Its drafts and theirs, the newest first.
    pub drafts: Vec<Draft>,
}

pub fn of(play: &Path) -> Outputs {
    Outputs {
        folder: folder(play),
        pdf: made(build::pdf_path(play)),
        copies: copies(play),
        table: named(|tr| casting::table_path_in(play, "pdf", tr))
            .into_iter()
            .find_map(made),
        drafts: drafts(play),
    }
}

fn folder(play: &Path) -> PathBuf {
    play.parent()
        .filter(|d| !d.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
        .to_path_buf()
}

fn made(path: PathBuf) -> Option<Made> {
    path.is_file().then(|| Made {
        modified: file::modified(&path),
        path,
    })
}

/// A name in the current language, then in the other. The play's language
/// named it, and the start screen has no play open to ask.
fn named(name: impl Fn(bool) -> PathBuf) -> [PathBuf; 2] {
    let tr = lang::is_tr();
    [name(tr), name(!tr)]
}

fn copies(play: &Path) -> Option<Copies> {
    let path = named(|tr| casting::copies_dir_in(play, tr))
        .into_iter()
        .find(|d| d.is_dir())?;
    let pdfs: Vec<PathBuf> = fs::read_dir(&path)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.extension().is_some_and(|x| x == "pdf"))
        .collect();
    Some(Copies {
        modified: pdfs
            .iter()
            .filter_map(|p| file::modified(p))
            .max()
            .or_else(|| file::modified(&path)),
        pdfs: pdfs.len(),
        path,
    })
}

/// A draft is always beside what it comes from, so its folder has them all,
/// wherever the play is.
fn drafts(play: &Path) -> Vec<Draft> {
    let mut all: Vec<Play> = fs::read_dir(folder(play))
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "tex"))
        .filter_map(|p| library::read(&p, timing::WPM))
        .collect();
    library::link_old_drafts(&mut all);
    let root = folder(play).join(play.file_name().unwrap_or_default());
    all.retain(|p| p.path != root);
    let mut found: Vec<Play> = Vec::new();
    let mut ups = vec![root];
    // Taken out as they are found: a loop of links ends by itself.
    while let Some(up) = ups.pop() {
        let (kids, rest): (Vec<Play>, Vec<Play>) = all
            .into_iter()
            .partition(|p| p.parent.as_deref() == Some(up.as_path()));
        all = rest;
        ups.extend(kids.iter().map(|p| p.path.clone()));
        found.extend(kids);
    }
    found.sort_by_key(|p| std::cmp::Reverse(p.modified));
    found
        .into_iter()
        .map(|p| Draft {
            path: p.path,
            title: p.title,
            draft: p.draft,
            modified: p.modified,
        })
        .collect()
}

/// Whether a path, there or still to be made, is somewhere the system
/// empties: the temporary folders, or a user's runtime folder.
pub fn is_temporary(path: &Path) -> bool {
    under(path, &temporary_roots())
}

fn temporary_roots() -> Vec<PathBuf> {
    let mut roots = vec![
        std::env::temp_dir(),
        PathBuf::from("/tmp"),
        PathBuf::from("/var/tmp"),
        PathBuf::from("/dev/shm"),
    ];
    // `/run/user/<id>`, anyone's: no need for a user id here.
    roots.extend(
        fs::read_dir("/run/user")
            .into_iter()
            .flatten()
            .flatten()
            .filter(|e| {
                e.file_name()
                    .to_string_lossy()
                    .bytes()
                    .all(|b| b.is_ascii_digit())
            })
            .map(|e| e.path()),
    );
    roots
}

/// Both sides as they really are: `path`, or the nearest parent of it that
/// exists, and each root.
fn under(path: &Path, roots: &[PathBuf]) -> bool {
    let Some(real) = path.ancestors().find_map(|a| {
        let a = if a.as_os_str().is_empty() {
            Path::new(".")
        } else {
            a
        };
        a.canonicalize().ok()
    }) else {
        return false;
    };
    roots
        .iter()
        .any(|r| real.starts_with(r.canonicalize().unwrap_or_else(|_| r.clone())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sidecar::Sidecar;

    fn play(title: &str, draft: &str) -> String {
        format!(
            "\\documentclass{{tiyatro}}\n\\title{{{title}}}\n\\draft{{{draft}}}\n\\begin{{document}}\n\\end{{document}}\n"
        )
    }

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("dactile-out-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    fn draft_of(path: &Path, parent: &str) {
        let side = Sidecar {
            parent: Some(parent.into()),
            ..Sidecar::default()
        };
        side.save(path).unwrap();
    }

    #[test]
    fn a_new_play_has_only_its_folder() {
        let d = tmp("new");
        let p = d.join("oyun.tex");
        fs::write(&p, play("Oyun", "")).unwrap();
        let o = of(&p);
        assert_eq!(o.folder, d);
        assert_eq!(
            (o.pdf, o.copies, o.table, o.drafts.len()),
            (None, None, None, 0)
        );
        let _ = fs::remove_dir_all(d);
    }

    #[test]
    fn what_a_play_made() {
        let d = tmp("made");
        let p = d.join("oyun.tex");
        fs::write(&p, play("Oyun", "")).unwrap();
        fs::write(d.join("oyun.pdf"), "x").unwrap();
        // Whatever language the play or the screen is in.
        fs::write(casting::table_path_in(&p, "pdf", false), "x").unwrap();
        let dir = casting::copies_dir_in(&p, true);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("Ayşe Yılmaz.pdf"), "x").unwrap();
        fs::write(dir.join("Can Demir.pdf"), "x").unwrap();
        fs::write(dir.join("not.txt"), "x").unwrap();
        let o = of(&p);
        let pdf = o.pdf.unwrap();
        assert_eq!(pdf.path, d.join("oyun.pdf"));
        assert!(pdf.modified.is_some());
        let c = o.copies.unwrap();
        assert_eq!((c.path, c.pdfs), (dir, 2));
        assert!(c.modified.is_some());
        assert_eq!(o.table.unwrap().path, d.join("oyun-table.pdf"));
        let _ = fs::remove_dir_all(d);
    }

    #[test]
    fn drafts_by_link_by_name_and_of_drafts() {
        let d = tmp("drafts");
        let p = d.join("oyun.tex");
        fs::write(&p, play("Oyun", "")).unwrap();
        // Linked, under any name.
        let linked = d.join("deneme.tex");
        fs::write(&linked, play("Başka Ad", "0.3")).unwrap();
        draft_of(&linked, "oyun.tex");
        // A draft of that draft.
        let deeper = d.join("deneme-2.tex");
        fs::write(&deeper, play("Başka Ad", "0.4")).unwrap();
        draft_of(&deeper, "deneme.tex");
        // From before the link was kept: the name and the title tell.
        let old = d.join("oyun-0.2.tex");
        fs::write(&old, play("Oyun", "0.2")).unwrap();
        // The same kind of name, another play.
        fs::write(d.join("oyun-0.5.tex"), play("Öbür Oyun", "")).unwrap();
        fs::write(d.join("yabanci.tex"), play("Yabancı", "")).unwrap();
        let mut got: Vec<PathBuf> = of(&p).drafts.into_iter().map(|x| x.path).collect();
        got.sort();
        assert_eq!(got, [deeper, linked.clone(), old]);
        // A draft's own list, and a play that is nobody's.
        assert_eq!(of(&linked).drafts.len(), 1);
        assert!(of(&d.join("yabanci.tex")).drafts.is_empty());
        let _ = fs::remove_dir_all(d);
    }

    #[test]
    fn a_loop_of_drafts_ends() {
        let d = tmp("loop");
        let (a, b) = (d.join("a.tex"), d.join("b.tex"));
        fs::write(&a, play("A", "")).unwrap();
        fs::write(&b, play("B", "")).unwrap();
        draft_of(&a, "b.tex");
        draft_of(&b, "a.tex");
        let got: Vec<PathBuf> = of(&a).drafts.into_iter().map(|x| x.path).collect();
        assert_eq!(got, [b]);
        let _ = fs::remove_dir_all(d);
    }

    #[test]
    fn temporary_places() {
        let d = tmp("temp");
        let (temp, home) = (d.join("temp"), d.join("home"));
        fs::create_dir_all(temp.join("oyunlar")).unwrap();
        fs::create_dir_all(&home).unwrap();
        let roots = [temp.clone()];
        assert!(under(&temp.join("oyunlar"), &roots));
        assert!(!under(&home, &roots));
        // Not there yet: its nearest parent says.
        assert!(under(&temp.join("yok/daha/oyun.tex"), &roots));
        assert!(!under(&home.join("yok/daha/oyun.tex"), &roots));
        // Only a whole folder's name counts.
        fs::create_dir_all(d.join("temporary")).unwrap();
        assert!(!under(&d.join("temporary"), &roots));
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            // A link in a safe place to a temporary one, and the other way.
            let link = home.join("baglanti");
            symlink(temp.join("oyunlar"), &link).unwrap();
            assert!(under(&link, &roots));
            assert!(under(&link.join("yeni.tex"), &roots));
            let out = temp.join("disari");
            symlink(&home, &out).unwrap();
            assert!(!under(&out, &roots));
            // A root that is itself a link.
            let root = d.join("root");
            symlink(&temp, &root).unwrap();
            assert!(under(&temp.join("oyunlar"), &[root]));
        }
        assert!(is_temporary(&std::env::temp_dir().join("yok/oyun.tex")));
        assert!(is_temporary(Path::new("/tmp/yok/oyun.tex")));
        assert!(is_temporary(Path::new("/var/tmp")));
        let _ = fs::remove_dir_all(d);
    }
}
