//! Reading and writing a play safely: atomic saves, a lock against a second
//! Dactile editing the same file, and a history of earlier versions.

use anyhow::{Context, Result};
use chrono::Local;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
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
/// so a crash never leaves half a file, and waits for the disk before the
/// rename, so a power cut doesn't leave an empty one. Keeps the file's
/// permissions; a link is written through, not replaced.
pub fn write_atomic(path: &Path, text: &str) -> Result<()> {
    let real = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let path = real.as_path();
    let dir = path
        .parent()
        .filter(|d| !d.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let name = path.file_name().context("no file name")?.to_string_lossy();
    // A name of its own for each write, made new: two writes of one file at
    // once (a build and a save) never share it, and a link left in its place
    // is never followed.
    static WRITES: AtomicUsize = AtomicUsize::new(0);
    let n = WRITES.fetch_add(1, Ordering::Relaxed);
    let tmp = dir.join(format!(".{name}.{}-{n}.dactile-tmp", std::process::id()));
    let written = (|| {
        let mut f = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)?;
        f.write_all(text.as_bytes())?;
        f.sync_all()
    })();
    if let Err(e) = written {
        let _ = fs::remove_file(&tmp);
        return Err(e).with_context(|| format!("writing {}", tmp.display()));
    }
    if let Ok(meta) = fs::metadata(path) {
        let _ = fs::set_permissions(&tmp, meta.permissions());
    }
    if let Err(e) = fs::rename(&tmp, path) {
        let _ = fs::remove_file(&tmp);
        return Err(e).with_context(|| format!("saving {}", path.display()));
    }
    // The rename itself, on disk.
    if let Ok(d) = fs::File::open(dir) {
        let _ = d.sync_all();
    }
    Ok(())
}

pub fn modified(path: &Path) -> Option<SystemTime> {
    fs::metadata(path).and_then(|m| m.modified()).ok()
}

// ------------------------------------------------------------------ lock

/// `.oyun.tex.dactile-lock` next to the play while it is open.
pub struct Lock {
    path: PathBuf,
    /// What we wrote in it: while it still says this, it is ours.
    line: String,
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
        // The number alone may have gone to another program since a crash.
        fs::read_to_string(format!("/proc/{pid}/comm"))
            .is_ok_and(|name| name.trim().starts_with("dactile"))
    } else {
        true
    }
}

/// Who holds the play open, when another living Dactile does.
pub fn holder(play: &Path) -> Option<String> {
    let held = fs::read_to_string(lock_path(play)).ok()?;
    let mut parts = held.split_whitespace();
    let pid: u32 = parts.next()?.parse().ok()?;
    let lock_host = parts.next().unwrap_or("");
    let since = parts.next().unwrap_or("");
    (pid != std::process::id() && alive(pid, lock_host))
        .then(|| format!("{pid} {lock_host} {since}").trim().to_string())
}

/// "pid host since n": the last, a count of this run's locks, tells two of
/// them apart.
fn lock_line() -> String {
    static LOCKS: AtomicUsize = AtomicUsize::new(0);
    format!(
        "{} {} {} {}\n",
        std::process::id(),
        host(),
        Local::now().format("%Y-%m-%dT%H:%M"),
        LOCKS.fetch_add(1, Ordering::Relaxed)
    )
}

/// Takes the lock, unless a living Dactile holds it. A stale lock (its
/// process gone) is taken over.
pub fn lock(play: &Path) -> Result<LockState> {
    let path = lock_path(play);
    // Made new, so of two Dactiles opening the play at once only one gets it.
    for _ in 0..2 {
        let made = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path);
        match made {
            Ok(mut f) => {
                let line = lock_line();
                f.write_all(line.as_bytes())
                    .with_context(|| format!("writing {}", path.display()))?;
                return Ok(LockState::Ours(Lock { path, line }));
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                if let Some(held) = holder(play) {
                    return Ok(LockState::Taken(held));
                }
                // Stale, or ours from before: away with it, and once more.
                let _ = fs::remove_file(&path);
            }
            Err(e) => return Err(e).with_context(|| format!("writing {}", path.display())),
        }
    }
    Ok(LockState::Taken(
        fs::read_to_string(&path)
            .unwrap_or_default()
            .trim()
            .to_string(),
    ))
}

/// Takes the lock whoever holds it: the writer asked for it twice.
pub fn take_over(play: &Path) -> Result<Lock> {
    let path = lock_path(play);
    let _ = fs::remove_file(&path);
    match lock(play)? {
        LockState::Ours(l) => Ok(l),
        LockState::Taken(who) => anyhow::bail!("{who}"),
    }
}

impl Drop for Lock {
    fn drop(&mut self) {
        // Only while it is still ours: another Dactile may have taken it over.
        if fs::read_to_string(&self.path).is_ok_and(|held| held == self.line) {
            let _ = fs::remove_file(&self.path);
        }
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
    // To the millisecond: two at once (theirs and ours, on a conflict) keep
    // both, and the names still sort by time.
    let mut name;
    loop {
        name = dir.join(format!("{}.tex", Local::now().format("%Y%m%d-%H%M%S%.3f")));
        if !name.exists() {
            break;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    write_atomic(&name, text)?;
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
    fn atomic_write_through_a_link() {
        let d = tmp("link");
        let real = d.join("oyun.tex");
        let link = d.join("kisayol.tex");
        fs::write(&real, "eski").unwrap();
        std::os::unix::fs::symlink(&real, &link).unwrap();
        write_atomic(&link, "yeni").unwrap();
        assert!(
            fs::symlink_metadata(&link)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(fs::read_to_string(&real).unwrap(), "yeni");
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn atomic_write_keeps_bytes() {
        let d = tmp("write");
        let p = d.join("oyun.tex");
        write_atomic(&p, "a\r\nb").unwrap();
        assert_eq!(fs::read(&p).unwrap(), b"a\r\nb");
        assert_eq!(fs::read_dir(&d).unwrap().count(), 1);
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
        // Another living Dactile's lock is respected: a program of that name.
        let other = d.join("dactile-other");
        fs::copy("/bin/sleep", &other).unwrap();
        let mut child = std::process::Command::new(&other)
            .arg("30")
            .spawn()
            .unwrap();
        std::thread::sleep(Duration::from_millis(100));
        let living = format!("{} {} now", child.id(), host());
        fs::write(lock_path(&p), &living).unwrap();
        assert!(matches!(lock(&p).unwrap(), LockState::Taken(_)));
        // Any other program with the number is not one.
        fs::write(lock_path(&p), format!("1 {} now", host())).unwrap();
        assert!(matches!(lock(&p).unwrap(), LockState::Ours(_)));
        // A dead process's lock is taken over.
        fs::write(lock_path(&p), format!("4000000000 {} now", host())).unwrap();
        let LockState::Ours(l2) = lock(&p).unwrap() else {
            panic!()
        };
        // Taken over by hand from a living one, and the old holder's going
        // leaves it alone.
        fs::write(lock_path(&p), &living).unwrap();
        let l3 = take_over(&p).unwrap();
        let _ = child.kill();
        let _ = child.wait();
        drop(l);
        drop(l2);
        assert!(lock_path(&p).exists());
        drop(l3);
        assert!(!lock_path(&p).exists());
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
        // The same second: a second name, not over the first.
        assert!(snapshot(&d, "üç", true).unwrap());
        assert!(snapshot(&d, "dört", true).unwrap());
        assert_eq!(versions(&d).len(), 4);
        fs::remove_dir_all(d).unwrap();
    }
}
