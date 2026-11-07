//! Where a play's things are: its folder, its PDF, the actors' copies, the
//! role table and its drafts. And whether a place is one the system empties.

use crate::sidecar::Sidecar;
use crate::{build, casting, file, info, lang, library};
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
            // `kar-tablo.tex` is a play of its own, with its own PDF.
            .filter(|p| !p.with_extension("tex").exists())
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

/// The folder with PDFs in it: an empty one in the other language doesn't
/// hide them.
fn copies(play: &Path) -> Option<Copies> {
    let mut found: Vec<Copies> = named(|tr| casting::copies_dir_in(play, tr))
        .into_iter()
        .filter(|d| d.is_dir())
        .map(copies_in)
        .collect();
    let best = found.iter().position(|c| c.pdfs > 0).unwrap_or(0);
    (!found.is_empty()).then(|| found.swap_remove(best))
}

fn copies_in(path: PathBuf) -> Copies {
    let pdfs: Vec<PathBuf> = fs::read_dir(&path)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.extension().is_some_and(|x| x == "pdf"))
        .collect();
    Copies {
        modified: pdfs
            .iter()
            .filter_map(|p| file::modified(p))
            .max()
            .or_else(|| file::modified(&path)),
        pdfs: pdfs.len(),
        path,
    }
}

/// What a `.tex` beside the play is linked to: by its notes file, or, for a
/// draft older than the link, by its name alone (false).
fn link(path: &Path) -> Option<(PathBuf, bool)> {
    let kept = Sidecar::load(path).ok().and_then(|s| s.parent);
    match kept {
        Some(name) => Some((path.with_file_name(name), true)),
        None => library::old_parent(path).map(|up| (up, false)),
    }
}

/// A draft's row, from its preamble: no need to read its scenes.
fn row(path: &Path) -> Option<Draft> {
    let text = fs::read_to_string(path).ok()?;
    library::is_play(&text).then(|| Draft {
        title: library::title_of(&text, path),
        draft: info::get(&text, "draft").filter(|d| !d.trim().is_empty()),
        modified: file::modified(path),
        path: path.to_path_buf(),
    })
}

/// A draft is always beside what it comes from, so its folder has them all,
/// wherever the play is. The names and links tell which they are: a big
/// play nearby that is no kin is never opened.
fn drafts(play: &Path) -> Vec<Draft> {
    let dir = folder(play);
    let root = dir.join(play.file_name().unwrap_or_default());
    let mut links: Vec<(PathBuf, PathBuf, bool)> = fs::read_dir(&dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "tex") && *p != root)
        .filter_map(|p| link(&p).map(|(up, kept)| (p, up, kept)))
        .collect();
    let mut found: Vec<Draft> = Vec::new();
    let mut ups = vec![root];
    // Taken out as they are found: a loop of links ends by itself.
    while let Some(up) = ups.pop() {
        let (kids, rest): (Vec<_>, Vec<_>) = links.into_iter().partition(|l| l.1 == up);
        links = rest;
        let title = kids
            .iter()
            .any(|k| !k.2)
            .then(|| row(&up).map(|d| d.title))
            .flatten();
        for (path, _, kept) in kids {
            let Some(d) = row(&path) else { continue };
            if kept || title.as_ref() == Some(&d.title) {
                ups.push(path);
                found.push(d);
            }
        }
    }
    found.sort_by_key(|d| std::cmp::Reverse(d.modified));
    found
}

/// Whether a path, there or still to be made, is somewhere the system
/// empties: the temporary folders, or a user's runtime folder.
pub fn is_temporary(path: &Path) -> bool {
    let (roots, mounts) = places(Path::new("/run/user"));
    under(path, &roots, &mounts)
}

/// The temporary folders, and what is mounted inside them and stays: in
/// each `/run/user/<id>` (`run` is `/run/user`), the network shares of
/// `gvfs` and the documents of `doc`.
fn places(run: &Path) -> (Vec<PathBuf>, Vec<PathBuf>) {
    let mut roots = vec![
        std::env::temp_dir(),
        PathBuf::from("/tmp"),
        PathBuf::from("/var/tmp"),
        PathBuf::from("/dev/shm"),
    ];
    let mut mounts = Vec::new();
    // Anyone's: no need for a user id here.
    for e in fs::read_dir(run).into_iter().flatten().flatten() {
        let id = e.file_name();
        if id.to_string_lossy().bytes().all(|b| b.is_ascii_digit()) {
            mounts.push(e.path().join("gvfs"));
            mounts.push(e.path().join("doc"));
            roots.push(e.path());
        }
    }
    (roots, mounts)
}

/// `path` as it really is: its nearest parent that exists, resolved, and
/// the rest as written.
fn real(path: &Path) -> Option<PathBuf> {
    path.ancestors().find_map(|a| {
        let at = if a.as_os_str().is_empty() {
            Path::new(".")
        } else {
            a
        };
        let rest = path.strip_prefix(a).ok()?;
        at.canonicalize().ok().map(|r| r.join(rest))
    })
}

/// Both sides as they really are.
fn under(path: &Path, roots: &[PathBuf], mounts: &[PathBuf]) -> bool {
    let Some(path) = real(path) else {
        return false;
    };
    let inside = |places: &[PathBuf]| {
        places
            .iter()
            .any(|p| path.starts_with(real(p).unwrap_or_else(|| p.clone())))
    };
    inside(roots) && !inside(mounts)
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn another_plays_pdf_is_not_the_table() {
        let d = tmp("table");
        let p = d.join("kar.tex");
        fs::write(&p, play("Kar", "")).unwrap();
        for tr in [true, false] {
            let t = casting::table_path_in(&p, "pdf", tr);
            fs::write(&t, "x").unwrap();
            fs::write(t.with_extension("tex"), play("Başka Oyun", "")).unwrap();
        }
        assert_eq!(of(&p).table, None);
        // In one language it is another play's, in the other the table.
        let table = casting::table_path_in(&p, "pdf", false);
        fs::remove_file(table.with_extension("tex")).unwrap();
        assert_eq!(of(&p).table.unwrap().path, table);
        let _ = fs::remove_dir_all(d);
    }

    #[test]
    fn an_empty_folder_does_not_hide_the_copies() {
        for empty_tr in [true, false] {
            let d = tmp(&format!("copies-{empty_tr}"));
            let p = d.join("kar.tex");
            let (empty, full) = (
                casting::copies_dir_in(&p, empty_tr),
                casting::copies_dir_in(&p, !empty_tr),
            );
            fs::create_dir_all(&empty).unwrap();
            fs::create_dir_all(&full).unwrap();
            fs::write(full.join("Can Demir.pdf"), "x").unwrap();
            let c = of(&p).copies.unwrap();
            assert_eq!((c.path, c.pdfs), (full.clone(), 1));
            // Only the empty one: still shown, with nothing in it.
            fs::remove_dir_all(&full).unwrap();
            let c = of(&p).copies.unwrap();
            assert_eq!((c.path, c.pdfs), (empty, 0));
            let _ = fs::remove_dir_all(d);
        }
    }

    #[test]
    fn a_big_neighbour_is_not_read() {
        let d = tmp("big");
        let p = d.join("kar.tex");
        fs::write(&p, play("Kar", "")).unwrap();
        let body = "\\replik{ay}{Bir gün daha geçti, kar yağdı.}\n".repeat(500_000);
        let big = play("Komşu", "").replace("\\end{document}", &body);
        // No kin, and kin with a big body: neither reads its scenes.
        fs::write(d.join("komsu.tex"), &big).unwrap();
        fs::write(d.join("kar-0.2.tex"), big.replace("Komşu", "Kar")).unwrap();
        let start = std::time::Instant::now();
        assert_eq!(of(&p).drafts.len(), 1);
        assert!(start.elapsed() < std::time::Duration::from_secs(5));
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
    fn mounts_in_the_runtime_folder_stay() {
        let d = tmp("run");
        let user = d.join("1000");
        fs::create_dir_all(user.join("gvfs/smb-share:server=ev,share=oyunlar")).unwrap();
        fs::create_dir_all(user.join("doc/abc")).unwrap();
        fs::create_dir_all(d.join("yabanci")).unwrap();
        let (roots, mounts) = places(&d);
        let temporary = |p: &Path| under(p, &roots, &mounts);
        assert!(temporary(&user));
        assert!(temporary(&user.join("oyun.tex")));
        // A share, there or not mounted yet, and a document.
        assert!(!temporary(
            &user.join("gvfs/smb-share:server=ev,share=oyunlar/kar.tex")
        ));
        assert!(!temporary(&user.join("doc/abc/kar.tex")));
        fs::remove_dir_all(user.join("gvfs")).unwrap();
        assert!(!temporary(
            &user.join("gvfs/smb-share:server=ev,share=oyunlar/kar.tex")
        ));
        // Only a whole name, and only under a user's number.
        assert!(temporary(&user.join("gvfs-eski/kar.tex")));
        assert_eq!(roots.iter().filter(|r| r.starts_with(&d)).count(), 1);
        let _ = fs::remove_dir_all(d);
    }

    #[test]
    fn temporary_places() {
        let d = tmp("temp");
        let (temp, home) = (d.join("temp"), d.join("home"));
        fs::create_dir_all(temp.join("oyunlar")).unwrap();
        fs::create_dir_all(&home).unwrap();
        let roots = [temp.clone()];
        assert!(under(&temp.join("oyunlar"), &roots, &[]));
        assert!(!under(&home, &roots, &[]));
        // Not there yet: its nearest parent says.
        assert!(under(&temp.join("yok/daha/oyun.tex"), &roots, &[]));
        assert!(!under(&home.join("yok/daha/oyun.tex"), &roots, &[]));
        // Only a whole folder's name counts.
        fs::create_dir_all(d.join("temporary")).unwrap();
        assert!(!under(&d.join("temporary"), &roots, &[]));
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            // A link in a safe place to a temporary one, and the other way.
            let link = home.join("baglanti");
            symlink(temp.join("oyunlar"), &link).unwrap();
            assert!(under(&link, &roots, &[]));
            assert!(under(&link.join("yeni.tex"), &roots, &[]));
            let out = temp.join("disari");
            symlink(&home, &out).unwrap();
            assert!(!under(&out, &roots, &[]));
            // A root that is itself a link.
            let root = d.join("root");
            symlink(&temp, &root).unwrap();
            assert!(under(&temp.join("oyunlar"), &[root], &[]));
        }
        assert!(is_temporary(&std::env::temp_dir().join("yok/oyun.tex")));
        assert!(is_temporary(Path::new("/tmp/yok/oyun.tex")));
        assert!(is_temporary(Path::new("/var/tmp")));
        let _ = fs::remove_dir_all(d);
    }
}
