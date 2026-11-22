//! The browser names files by path, as the desktop's window does. Here every
//! such path must stay inside the person's own folder, and names that become
//! part of a path must be plain names.

use serde_json::Value;
use std::path::{Component, Path, PathBuf};

/// `p`, if it is inside `root` once links and `..` are followed. A path that
/// isn't there yet counts by its nearest folder that is.
pub fn inside(root: &Path, p: &str) -> Result<PathBuf, String> {
    let p = Path::new(p);
    if !p.is_absolute() {
        return Err(refused());
    }
    let mut there = p.to_path_buf();
    let mut rest = Vec::new();
    let real = loop {
        match there.canonicalize() {
            Ok(r) => break r,
            Err(_) => {
                let Some(name) = there.file_name() else {
                    return Err(refused());
                };
                rest.push(name.to_owned());
                if !there.pop() {
                    return Err(refused());
                }
            }
        }
    };
    // What isn't there yet is plain names only.
    if rest
        .iter()
        .any(|n| !matches!(Path::new(n).components().next(), Some(Component::Normal(_))))
    {
        return Err(refused());
    }
    let full = rest.iter().rev().fold(real, |p, n| p.join(n));
    if full.starts_with(root) {
        Ok(full)
    } else {
        Err(refused())
    }
}

/// A file or folder name with no way out of its folder in it.
pub fn plain(name: &str) -> Result<(), String> {
    let mut parts = Path::new(name).components();
    match (parts.next(), parts.next()) {
        (Some(Component::Normal(_)), None) if !name.contains(['/', '\\']) => Ok(()),
        _ => Err(refused()),
    }
}

fn refused() -> String {
    dactile::lang::l(
        "Bu dosyaya buradan ulaşılamaz.",
        "That file can't be reached from here.",
    )
    .to_string()
}

/// The folders a person's server reaches: their own, and the plays'
/// folders of the projects they are in, some only to read.
#[derive(Debug, Clone, Default)]
pub struct Roots {
    /// Theirs, canonical.
    pub own: PathBuf,
    /// A project's plays, canonical, and whether they may write there.
    pub shared: Vec<(PathBuf, bool)>,
}

impl Roots {
    pub fn alone(own: PathBuf) -> Roots {
        Roots {
            own,
            shared: Vec::new(),
        }
    }

    /// `p` if it is in one of the folders, and whether it may be written.
    pub fn place(&self, p: &str) -> Result<(PathBuf, bool), String> {
        if let Ok(full) = inside(&self.own, p) {
            return Ok((full, true));
        }
        for (dir, writes) in &self.shared {
            if let Ok(full) = inside(dir, p) {
                return Ok((full, *writes));
            }
        }
        Err(refused())
    }

    /// Whether a file already known to be in one of the folders may be
    /// written.
    pub fn writes_at(&self, p: &Path) -> bool {
        !self
            .shared
            .iter()
            .any(|(dir, writes)| !writes && p.starts_with(dir))
    }
}

/// The commands that write at the path they are given, beside it, or at
/// the open play: what someone who may only read a folder can't run there.
/// Every command is either here or in `READS` (a test says so).
pub const WRITES: &[&str] = &[
    "play_imported",
    "save",
    "build",
    "note_add",
    "note_set",
    "note_reply",
    "notes_paste",
    "note_fix",
    "note_fix_many",
    "learn",
    "character_edit",
    "summary_set",
    "scene_split",
    "scene_join",
    "docx_export",
    "notes_pdf",
    "notes_docx",
    "casting_edit",
    "casting_csv",
    "casting_pdf",
    "casting_copies",
    "memo",
    "book_pdf",
    "fork_file",
    "play_make",
    "play_rename",
    "play_throw",
    "play_restore",
    "play_wpm",
    "play_unlearn",
];

/// Of `WRITES`, those one who only reads a play still runs there: they
/// write its notes (a note, a reply; deleting one's own, which the
/// backend checks), never the play.
pub const NOTE_WRITES: &[&str] = &["note_add", "note_reply", "note_set"];

/// The commands that write nothing where they are pointed: they read, or
/// write only in the person's own folder.
pub const READS: &[&str] = &[
    "home",
    "settings",
    "set_setting",
    "preview",
    "create",
    "example",
    "open",
    "close",
    "save_template",
    "notes_list",
    "note_authors",
    "misspelt",
    "suggest",
    "summaries",
    "summary_here",
    "casting_roles",
    "import_stop",
    "import_edit",
    "import_doc",
    "import_write",
    "import_volume",
    "import_write_all",
    "import_drop",
    "book_gather",
    "fork_start",
    "copy_start",
    "rename_start",
    "play_drafts",
    "play_settings",
    "files",
    "sync_view",
    "sync_edit",
    "keys_view",
];

fn only_reads() -> String {
    dactile::lang::l(
        "Bu projede okursun: burada bir şey değiştiremezsin.",
        "You are a reader in this project: you can't change anything here.",
    )
    .to_string()
}

/// A command's arguments, checked before it runs: paths stay in the
/// folders, names stay names, the plays' folder is the server's to say,
/// and nothing is written where the person may only read. `open_reads`:
/// the open play is in such a folder.
pub fn check(roots: &Roots, name: &str, args: &Value, open_reads: bool) -> Result<(), String> {
    if name == "set_setting" && args.get("key").and_then(Value::as_str) == Some("plays_dir") {
        return Err(refused());
    }
    let writes = WRITES.contains(&name);
    if writes && open_reads && !NOTE_WRITES.contains(&name) {
        return Err(only_reads());
    }
    let Some(args) = args.as_object() else {
        return Ok(());
    };
    let at = |p: &str| -> Result<(), String> {
        let (_, may) = roots.place(p)?;
        if writes && !may {
            return Err(only_reads());
        }
        Ok(())
    };
    for (key, v) in args {
        match (key.as_str(), v) {
            ("path" | "pdf", Value::String(p)) => at(p)?,
            ("paths", Value::Array(ps)) => {
                for p in ps {
                    at(p.as_str().ok_or_else(refused)?)?;
                }
            }
            // Where a new play goes: somewhere it may be written.
            ("folder", Value::String(p)) => {
                if !roots.place(p)?.1 {
                    return Err(only_reads());
                }
            }
            ("file" | "draft", Value::String(n)) => plain(n)?,
            ("path" | "pdf" | "paths" | "file" | "folder", Value::Null) => {}
            ("path" | "pdf" | "paths" | "file" | "folder", _) => return Err(refused()),
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn root() -> PathBuf {
        let d = std::env::temp_dir().join(format!("dactile-confine-{}", std::process::id()));
        std::fs::create_dir_all(d.join("plays/Kar Masalı")).unwrap();
        std::fs::write(d.join("plays/Kar Masalı/kar-masali.tex"), "x").unwrap();
        d.canonicalize().unwrap()
    }

    #[test]
    fn paths_inside_pass_and_the_rest_do_not() {
        let r = root();
        let play = r.join("plays/Kar Masalı/kar-masali.tex");
        let s = |p: &Path| p.display().to_string();
        assert_eq!(inside(&r, &s(&play)).unwrap(), play);
        // Not there yet, but inside.
        assert!(inside(&r, &s(&r.join("plays/Yeni/yeni.tex"))).is_ok());
        for bad in [
            "/etc/passwd".to_string(),
            "plays/x.tex".to_string(),
            s(&r.join("plays/../../etc/passwd")),
            s(&r.join("plays/yok/../../../etc")),
            format!("{}/../{}", r.display(), "x"),
        ] {
            assert!(inside(&r, &bad).is_err(), "{bad}");
        }
        // A link out of the folder is outside.
        let link = r.join("plays/dışarı");
        let _ = std::fs::remove_file(&link);
        std::os::unix::fs::symlink("/etc", &link).unwrap();
        assert!(inside(&r, &s(&link.join("passwd"))).is_err());
    }

    #[test]
    fn commands_are_checked_by_their_arguments() {
        let r = root();
        let roots = Roots::alone(r.clone());
        let c = |name: &str, args: Value| check(&roots, name, &args, false);
        let play = r
            .join("plays/Kar Masalı/kar-masali.tex")
            .display()
            .to_string();
        assert!(c("open", json!({"path": play})).is_ok());
        assert!(c("open", json!({"path": "/etc/hosts"})).is_err());
        assert!(
            c(
                "book_gather",
                json!({"paths": [play, "/etc"], "title": "K"})
            )
            .is_err()
        );
        assert!(
            c(
                "play_make",
                json!({"path": play, "file": "yeni.tex", "draft": null})
            )
            .is_ok()
        );
        assert!(c("play_make", json!({"path": play, "file": "../yeni.tex"})).is_err());
        assert!(c("fork_file", json!({"path": play, "draft": "a/b"})).is_err());
        assert!(c("set_setting", json!({"key": "plays_dir", "value": "/"})).is_err());
        assert!(c("set_setting", json!({"key": "theme", "value": "paper"})).is_ok());
        assert!(c("view", json!({"text": "/etc/passwd"})).is_ok());
        assert!(c("create", json!({"folder": "/etc"})).is_err());
    }

    #[test]
    fn a_reader_reads_a_project_and_writes_nothing_there() {
        let r = root();
        let base = std::env::temp_dir().join(format!("dactile-shared-{}", std::process::id()));
        for d in ["yazar/plays/Prova", "okur/plays/Prova"] {
            std::fs::create_dir_all(base.join(d)).unwrap();
        }
        let base = base.canonicalize().unwrap();
        let (writes, reads) = (base.join("yazar/plays"), base.join("okur/plays"));
        std::fs::write(reads.join("Prova/prova.tex"), "x").unwrap();
        let roots = Roots {
            own: r.clone(),
            shared: vec![(writes.clone(), true), (reads.clone(), false)],
        };
        let s = |p: PathBuf| p.display().to_string();
        let theirs = s(reads.join("Prova/prova.tex"));
        assert!(check(&roots, "open", &json!({"path": theirs}), false).is_ok());
        assert!(check(&roots, "preview", &json!({"path": theirs}), false).is_ok());
        assert!(check(&roots, "play_throw", &json!({"path": theirs}), false).is_err());
        assert!(
            check(
                &roots,
                "play_make",
                &json!({"path": theirs, "file": "b.tex"}),
                false
            )
            .is_err()
        );
        // With it open, nothing that writes at the open play.
        assert!(check(&roots, "save", &json!({"text": "", "force": false}), true).is_err());
        assert!(check(&roots, "build", &json!({}), true).is_err());
        assert!(check(&roots, "view", &json!({"text": ""}), true).is_ok());
        // New plays go where they may be written.
        assert!(
            check(
                &roots,
                "create",
                &json!({"folder": s(writes.clone())}),
                false
            )
            .is_ok()
        );
        assert!(
            check(
                &roots,
                "create",
                &json!({"folder": s(reads.clone())}),
                false
            )
            .is_err()
        );
        assert!(!roots.writes_at(&reads.join("Prova/prova.tex")));
        assert!(roots.writes_at(&writes.join("x.tex")) && roots.writes_at(&r.join("plays/x.tex")));
        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn every_command_says_whether_it_writes() {
        let names = dactile_backend::commands::NAMES;
        let text = dactile_backend::commands::TEXT_ONLY;
        for n in names.iter().filter(|n| !text.contains(n)) {
            assert!(
                WRITES.contains(n) != READS.contains(n),
                "{n}: in WRITES or READS, and in only one"
            );
        }
        for n in WRITES.iter().chain(READS) {
            assert!(names.contains(n), "{n} is no command");
        }
    }
}
