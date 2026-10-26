//! Reading and writing a play safely: atomic saves, a lock against a second
//! Dactile editing the same file, and a history of earlier versions.

use anyhow::{Context, Result};
use chrono::Local;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// Keep this many versions of each play.
const HISTORY_KEEP: usize = 100;
/// A version is kept at most this often while writing.
const HISTORY_EVERY: Duration = Duration::from_secs(5 * 60);

pub fn read(path: &Path) -> Result<String> {
    let bytes = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    String::from_utf8(bytes).with_context(|| format!("{} is not UTF-8 text", path.display()))
}

/// Writes through a temporary file next to the play and renames it over,
/// so a crash never leaves half a file. Keeps the file's permissions.
pub fn write_atomic(path: &Path, text: &str) -> Result<()> {
    let dir = path
        .parent()
        .filter(|d| !d.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let name = path.file_name().context("no file name")?.to_string_lossy();
    let tmp = dir.join(format!(".{name}.dactile-tmp"));
    fs::write(&tmp, text).with_context(|| format!("writing {}", tmp.display()))?;
    if let Ok(meta) = fs::metadata(path) {
        let _ = fs::set_permissions(&tmp, meta.permissions());
    }
    fs::rename(&tmp, path).with_context(|| format!("saving {}", path.display()))?;
    Ok(())
}

pub fn modified(path: &Path) -> Option<SystemTime> {
    fs::metadata(path).and_then(|m| m.modified()).ok()
}

// ------------------------------------------------------------------ lock

/// `.oyun.tex.dactile-lock` next to the play while it is open.
pub struct Lock {
    path: PathBuf,
}

#[derive(Debug, PartialEq)]
pub enum LockState {
    /// We hold the lock.
    Ours(Lock),
    /// Another Dactile has the file open: "pid host since".
    Taken(String),
}

impl std::fmt::Debug for Lock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Lock({})", self.path.display())
    }
}

impl PartialEq for Lock {
    fn eq(&self, other: &Lock) -> bool {
        self.path == other.path
    }
}

pub fn lock_path(play: &Path) -> PathBuf {
    let name = play
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    play.with_file_name(format!(".{name}.dactile-lock"))
}

fn host() -> String {
    fs::read_to_string("/etc/hostname")
        .map(|h| h.trim().to_string())
        .unwrap_or_else(|_| std::env::var("HOSTNAME").unwrap_or_default())
}

/// Whether a lock left by `pid` on `host` is still held.
fn alive(pid: u32, lock_host: &str) -> bool {
    if lock_host != host() {
        // Another machine (a shared folder): trust it.
        return true;
    }
    if cfg!(target_os = "linux") {
        Path::new(&format!("/proc/{pid}")).exists()
    } else {
        true
    }
}

/// Takes the lock, unless a living Dactile holds it. A stale lock (its
/// process gone) is taken over.
pub fn lock(play: &Path) -> Result<LockState> {
    let path = lock_path(play);
    if let Ok(held) = fs::read_to_string(&path) {
        let mut parts = held.split_whitespace();
        let pid: Option<u32> = parts.next().and_then(|p| p.parse().ok());
        let lock_host = parts.next().unwrap_or("").to_string();
        if let Some(pid) = pid
            && pid != std::process::id()
            && alive(pid, &lock_host)
        {
            return Ok(LockState::Taken(held.trim().to_string()));
        }
    }
    let line = format!(
        "{} {} {}\n",
        std::process::id(),
        host(),
        Local::now().format("%Y-%m-%dT%H:%M")
    );
    fs::write(&path, line).with_context(|| format!("writing {}", path.display()))?;
    Ok(LockState::Ours(Lock { path }))
}

impl Drop for Lock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

// ------------------------------------------------------------------ history

/// Earlier versions of a play: `~/.local/share/dactile/history/<name>-<hash>/`.
pub fn history_dir(play: &Path) -> PathBuf {
    let full = play.canonicalize().unwrap_or_else(|_| play.to_path_buf());
    let stem = play
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    // FNV-1a: the same folder for the same path on every run.
    let hash = full
        .to_string_lossy()
        .bytes()
        .fold(0xcbf29ce484222325u64, |h, b| {
            (h ^ b as u64).wrapping_mul(0x100000001b3)
        });
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("dactile/history")
        .join(format!("{stem}-{:08x}", hash as u32))
}

/// The versions kept, oldest first.
pub fn versions(dir: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "tex"))
        .collect();
    out.sort();
    out
}

/// Keeps a version of `text` unless it is the newest one already, or
/// (unless `force`) the newest is younger than a few minutes. Returns
/// whether one was written.
pub fn snapshot(dir: &Path, text: &str, force: bool) -> Result<bool> {
    let kept = versions(dir);
    if let Some(last) = kept.last() {
        if fs::read_to_string(last).is_ok_and(|t| t == text) {
            return Ok(false);
        }
        let young = modified(last)
            .and_then(|m| m.elapsed().ok())
            .is_some_and(|age| age < HISTORY_EVERY);
        if young && !force {
            return Ok(false);
        }
    }
    fs::create_dir_all(dir)?;
    let name = format!("{}.tex", Local::now().format("%Y%m%d-%H%M%S"));
    fs::write(dir.join(name), text)?;
    for old in kept.iter().rev().skip(HISTORY_KEEP - 1) {
        let _ = fs::remove_file(old);
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("dactile-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn atomic_write_keeps_bytes() {
        let d = tmp("write");
        let p = d.join("oyun.tex");
        write_atomic(&p, "a\r\nb").unwrap();
        assert_eq!(fs::read(&p).unwrap(), b"a\r\nb");
        assert!(!d.join(".oyun.tex.dactile-tmp").exists());
        fs::remove_dir_all(d).unwrap();
    }

    #[test]
    fn locking() {
        let d = tmp("lock");
        let p = d.join("oyun.tex");
        fs::write(&p, "x").unwrap();
        let LockState::Ours(l) = lock(&p).unwrap() else {
            panic!()
        };
        assert!(lock_path(&p).exists());
        // Another living process's lock is respected: pid 1 always runs.
        fs::write(lock_path(&p), format!("1 {} now", host())).unwrap();
        assert!(matches!(lock(&p).unwrap(), LockState::Taken(_)));
        // A dead process's lock is taken over.
        fs::write(lock_path(&p), format!("4000000000 {} now", host())).unwrap();
        assert!(matches!(lock(&p).unwrap(), LockState::Ours(_)));
        drop(l);
        fs::remove_dir_all(d).unwrap();
    }

    #[test]
    fn history_keeps_changes_only() {
        let d = tmp("history");
        assert!(snapshot(&d, "bir", false).unwrap());
        assert!(!snapshot(&d, "bir", true).unwrap());
        // Too soon after the last one, unless forced.
        assert!(!snapshot(&d, "iki", false).unwrap());
        std::thread::sleep(Duration::from_millis(1100));
        assert!(snapshot(&d, "iki", true).unwrap());
        assert_eq!(versions(&d).len(), 2);
        fs::remove_dir_all(d).unwrap();
    }
}
