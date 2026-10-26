//! The desktop's trash, as the freedesktop.org spec keeps it: a file goes to
//! `Trash/files/`, and a note of where it came from to `Trash/info/`, so the
//! file manager can put it back too.

use anyhow::{Context, Result};
use chrono::Local;
use std::fs;
use std::path::{Path, PathBuf};

/// A file in the trash, and where it was.
#[derive(Debug, Clone, PartialEq)]
pub struct Moved {
    pub from: PathBuf,
    pub file: PathBuf,
    pub info: PathBuf,
}

/// `~/.local/share/Trash`.
pub fn bin() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Trash")
}

/// Puts each file in the trash `bin`. Stops at the first that fails, with
/// the ones already moved put back.
pub fn throw(paths: &[PathBuf], bin: &Path) -> Result<Vec<Moved>> {
    let (files, infos) = (bin.join("files"), bin.join("info"));
    fs::create_dir_all(&files)?;
    fs::create_dir_all(&infos)?;
    let mut done = Vec::new();
    for p in paths {
        match throw_one(p, &files, &infos) {
            Ok(m) => done.push(m),
            Err(e) => {
                let _ = restore(&done);
                return Err(e);
            }
        }
    }
    Ok(done)
}

fn throw_one(path: &Path, files: &Path, infos: &Path) -> Result<Moved> {
    let from = std::path::absolute(path)?;
    let name = from
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .context("no file name")?;
    let (stem, ext) = match name.rsplit_once('.') {
        Some((s, e)) if !s.is_empty() => (s.to_string(), format!(".{e}")),
        _ => (name.clone(), String::new()),
    };
    // The info file is made first and alone: it keeps the name taken.
    let mut n = 1;
    let (file, info) = loop {
        let here = if n == 1 {
            name.clone()
        } else {
            format!("{stem} ({n}){ext}")
        };
        let info = infos.join(format!("{here}.trashinfo"));
        let made = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&info);
        match made {
            Ok(_) if !files.join(&here).exists() => break (files.join(&here), info),
            Ok(_) => {
                let _ = fs::remove_file(&info);
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e.into()),
        }
        n += 1;
    };
    let note = format!(
        "[Trash Info]\nPath={}\nDeletionDate={}\n",
        encode(&from.to_string_lossy()),
        Local::now().format("%Y-%m-%dT%H:%M:%S")
    );
    let moved = fs::write(&info, note).and_then(|_| move_file(&from, &file));
    if let Err(e) = moved {
        let _ = fs::remove_file(&info);
        return Err(e).with_context(|| format!("trashing {}", from.display()));
    }
    Ok(Moved { from, file, info })
}

/// Puts them back where they were, unless something has taken a place.
pub fn restore(moved: &[Moved]) -> Result<()> {
    if let Some(m) = moved.iter().find(|m| m.from.exists()) {
        anyhow::bail!("{} exists", m.from.display());
    }
    for m in moved {
        move_file(&m.file, &m.from)?;
        let _ = fs::remove_file(&m.info);
    }
    Ok(())
}

/// A rename, or a copy and a removal across disks.
fn move_file(from: &Path, to: &Path) -> std::io::Result<()> {
    match fs::rename(from, to) {
        Err(e) if e.kind() == std::io::ErrorKind::CrossesDevices => {
            fs::copy(from, to)?;
            fs::remove_file(from)
        }
        r => r,
    }
}

/// A path as a URL writes it: `/a b/ç` → `/a%20b/%C3%A7`.
fn encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"/-_.~".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn into_the_trash_and_back() {
        let d = std::env::temp_dir().join(format!("dactile-test-trash-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(d.join("oyunlar")).unwrap();
        let bin = d.join("Trash");
        let play = d.join("oyunlar/kar masalı.tex");
        fs::write(&play, "bir").unwrap();
        let moved = throw(std::slice::from_ref(&play), &bin).unwrap();
        assert!(!play.exists());
        assert_eq!(fs::read_to_string(&moved[0].file).unwrap(), "bir");
        let note = fs::read_to_string(&moved[0].info).unwrap();
        assert!(note.contains("/oyunlar/kar%20masal%C4%B1.tex\n"), "{note}");
        // The same name again: a second place.
        fs::write(&play, "iki").unwrap();
        let again = throw(std::slice::from_ref(&play), &bin).unwrap();
        assert!(again[0].file.ends_with("kar masalı (2).tex"));
        restore(&again).unwrap();
        assert_eq!(fs::read_to_string(&play).unwrap(), "iki");
        assert!(!again[0].info.exists());
        // Its place taken: the first one stays in the trash.
        assert!(restore(&moved).is_err());
        assert!(moved[0].file.exists());
        let _ = fs::remove_dir_all(&d);
    }
}
