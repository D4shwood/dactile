//! Reading a book from its page images, where a PDF has no text or poor
//! text: each page drawn by `pdftoppm` and read by Tesseract, several at a
//! time. The desktop app carries both, and the models, in a folder of its
//! own (`packaging/ocr-bundle.sh` makes it); else they are the system's.

use crate::lang::l;
use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

/// Turkish reads capital I in foreign names badly on its own; the Latin
/// script model doesn't, and together they read a Turkish book best.
const BOTH: &str = "tur+Latin";
const TURKISH: &str = "tur";
/// Scans are mostly made near this, and Tesseract reads book type well at it.
const DPI: &str = "320";

/// A long reading: how far it is, and a way to stop it.
pub struct Job<'a> {
    /// Pages done, of how many.
    pub progress: &'a (dyn Fn(usize, usize) + Sync),
    pub stop: &'a AtomicBool,
}

#[derive(Debug, Clone)]
struct Tools {
    /// The folder the programs came in; none, they are the system's.
    bundle: Option<PathBuf>,
    langs: &'static str,
}

impl Tools {
    fn command(&self, program: &str) -> Command {
        match &self.bundle {
            Some(dir) => {
                let mut c = crate::build::outside(&dir.join("bin").join(program).to_string_lossy());
                c.env("LD_LIBRARY_PATH", dir.join("lib"));
                c.env("TESSDATA_PREFIX", dir.join("tessdata"));
                c
            }
            None => crate::build::outside(program),
        }
    }
}

/// Where a bundle may be, by the program that looks for it.
fn places(exe: &Path, appdir: Option<&Path>, asked: Option<&Path>) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = asked.map(Path::to_path_buf).into_iter().collect();
    if let Some(dir) = appdir {
        out.push(dir.join("usr/lib/Dactile/ocr"));
    }
    if let Some(bin) = exe.parent() {
        out.push(bin.join("ocr"));
        // In /usr/bin, and in target/debug while it is being worked on.
        out.push(bin.join("../lib/dactile/ocr"));
        out.push(bin.join("../ocr"));
    }
    out
}

fn is_bundle(dir: &Path) -> bool {
    [
        "bin/tesseract",
        "bin/pdftoppm",
        "bin/pdftotext",
        "tessdata/tur.traineddata",
    ]
    .iter()
    .all(|f| dir.join(f).is_file())
}

fn system() -> Option<Tools> {
    let out = crate::build::outside("tesseract")
        .arg("--list-langs")
        .output()
        .ok()?;
    let list = String::from_utf8_lossy(&out.stdout);
    let has = |lang: &str| list.lines().any(|l| l.trim() == lang);
    let there = |p: &str| crate::build::outside(p).arg("-v").output().is_ok();
    (there("pdftoppm") && there("pdftotext") && has("tur")).then(|| Tools {
        bundle: None,
        langs: if has("Latin") { BOTH } else { TURKISH },
    })
}

fn tools() -> Option<&'static Tools> {
    static TOOLS: OnceLock<Option<Tools>> = OnceLock::new();
    TOOLS
        .get_or_init(|| {
            let exe = std::env::current_exe().unwrap_or_default();
            let appdir = std::env::var_os("APPDIR").map(PathBuf::from);
            let asked = std::env::var_os("DACTILE_OCR").map(PathBuf::from);
            places(&exe, appdir.as_deref(), asked.as_deref())
                .into_iter()
                .find(|d| is_bundle(d))
                .map(|dir| Tools {
                    langs: if dir.join("tessdata/Latin.traineddata").is_file() {
                        BOTH
                    } else {
                        TURKISH
                    },
                    bundle: Some(dir),
                })
                .or_else(system)
        })
        .as_ref()
}

/// Whether a book can be read from its images here.
pub fn ready() -> bool {
    tools().is_some()
}

/// A program for reading PDFs: the one carried along, else the system's.
pub(super) fn poppler(program: &str) -> Command {
    match tools() {
        Some(t)
            if t.bundle
                .as_ref()
                .is_some_and(|d| d.join("bin").join(program).is_file()) =>
        {
            t.command(program)
        }
        _ => crate::build::outside(program),
    }
}

/// Removed when the reading ends, however it ends: the pages are the
/// user's book.
struct Scratch(PathBuf);

/// This process's page images.
fn scratch_dir() -> PathBuf {
    std::env::temp_dir().join(format!("dactile-ocr-{}", std::process::id()))
}

impl Scratch {
    fn new() -> Result<Scratch> {
        let dir = scratch_dir();
        std::fs::create_dir_all(&dir)?;
        Ok(Scratch(dir))
    }
}

/// On quitting: a reading under way doesn't get to clean up after itself.
pub fn remove_scratch() {
    let _ = std::fs::remove_dir_all(scratch_dir());
}

/// On starting: what readings left behind in processes that are gone.
pub fn sweep_scratch() {
    // Without `/proc` no one can be told gone.
    if Path::new("/proc/self").exists() {
        sweep_in(&std::env::temp_dir(), |pid| {
            Path::new("/proc").join(pid.to_string()).exists()
        });
    }
}

fn sweep_in(dir: &Path, alive: impl Fn(u32) -> bool) {
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let name = entry.file_name();
        let pid = name
            .to_str()
            .and_then(|n| n.strip_prefix("dactile-ocr-"))
            .and_then(|n| n.parse().ok());
        if pid.is_some_and(|p| !alive(p)) {
            let _ = std::fs::remove_dir_all(entry.path());
        }
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn page_count(t: &Tools, pdf: &Path) -> Result<usize> {
    let out = t.command("pdfinfo").arg(pdf).output().context("pdfinfo")?;
    pages_said(&String::from_utf8_lossy(&out.stdout)).context("pdfinfo")
}

fn pages_said(info: &str) -> Option<usize> {
    info.lines()
        .find_map(|l| l.strip_prefix("Pages:"))
        .and_then(|n| n.trim().parse().ok())
}

fn page(t: &Tools, pdf: &Path, dir: &Path, n: usize) -> Result<String> {
    let image = dir.join(format!("p{n}"));
    let number = n.to_string();
    let drawn = t
        .command("pdftoppm")
        .args([
            "-r",
            DPI,
            "-gray",
            "-png",
            "-singlefile",
            "-f",
            &number,
            "-l",
            &number,
        ])
        .arg(pdf)
        .arg(&image)
        .output()
        .context("pdftoppm")?;
    if !drawn.status.success() {
        bail!(
            "pdftoppm: {}",
            String::from_utf8_lossy(&drawn.stderr).trim()
        );
    }
    let png = image.with_extension("png");
    // Through a PDF of the words alone: `pdftotext` then keeps where they
    // stood, which a cast page's columns need, as with any other PDF.
    let read = t
        .command("tesseract")
        .arg(&png)
        .arg(&image)
        .args(["-l", t.langs, "--psm", "6", "--dpi", DPI])
        .args(["-c", "tessedit_create_pdf=1", "-c", "textonly_pdf=1"])
        // The pages are read side by side already.
        .env("OMP_THREAD_LIMIT", "1")
        .output()
        .context("tesseract")?;
    let _ = std::fs::remove_file(&png);
    if !read.status.success() {
        bail!(
            "tesseract: {}",
            String::from_utf8_lossy(&read.stderr).trim()
        );
    }
    let words = image.with_extension("pdf");
    let out = t
        .command("pdftotext")
        .args(["-layout", "-enc", "UTF-8"])
        .arg(&words)
        .arg("-")
        .output()
        .context("pdftotext")?;
    let _ = std::fs::remove_file(&words);
    let _ = std::fs::remove_file(image.with_extension("txt"));
    let text = String::from_utf8_lossy(&out.stdout);
    Ok(text.trim_end_matches(['\u{c}', '\n']).to_string())
}

/// The book's text, its pages parted as `pdftotext` parts them.
pub(super) fn text(pdf: &Path, job: &Job) -> Result<String> {
    let Some(t) = tools() else {
        bail!(l(
            "OCR için Tesseract bulunamadı",
            "Tesseract was not found for OCR"
        ));
    };
    let pages = page_count(t, pdf)?;
    let dir = Scratch::new()?;
    let next = AtomicUsize::new(1);
    let done = AtomicUsize::new(0);
    let read: Mutex<Vec<String>> = Mutex::new(vec![String::new(); pages]);
    let failed: Mutex<Option<anyhow::Error>> = Mutex::new(None);
    let failing = AtomicBool::new(false);
    let cores = std::thread::available_parallelism().map_or(2, |n| n.get());
    (job.progress)(0, pages);
    std::thread::scope(|s| {
        for _ in 0..cores.saturating_sub(1).clamp(1, pages.max(1)) {
            s.spawn(|| {
                loop {
                    if job.stop.load(Ordering::Relaxed) || failing.load(Ordering::Relaxed) {
                        break;
                    }
                    let n = next.fetch_add(1, Ordering::Relaxed);
                    if n > pages {
                        break;
                    }
                    match page(t, pdf, &dir.0, n) {
                        Ok(text) => {
                            read.lock().unwrap()[n - 1] = text;
                            let d = done.fetch_add(1, Ordering::Relaxed) + 1;
                            (job.progress)(d, pages);
                        }
                        Err(e) => {
                            failing.store(true, Ordering::Relaxed);
                            *failed.lock().unwrap() = Some(e);
                        }
                    }
                }
            });
        }
    });
    if let Some(e) = failed.into_inner().unwrap() {
        return Err(e);
    }
    if job.stop.load(Ordering::Relaxed) {
        bail!(l("okuma durduruldu", "the reading was stopped"));
    }
    Ok(read.into_inner().unwrap().join("\u{c}\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_gone_are_swept() {
        let root = std::env::temp_dir().join(format!("dactile-sweep-{}", std::process::id()));
        for name in ["dactile-ocr-1", "dactile-ocr-2", "dactile-ocr-x", "oyunlar"] {
            std::fs::create_dir_all(root.join(name)).unwrap();
        }
        sweep_in(&root, |pid| pid == 2);
        assert!(!root.join("dactile-ocr-1").exists());
        assert!(root.join("dactile-ocr-2").exists());
        assert!(root.join("dactile-ocr-x").exists());
        assert!(root.join("oyunlar").exists());
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn where_a_bundle_is_looked_for() {
        let at = places(
            Path::new("/usr/bin/dactile"),
            Some(Path::new("/tmp/.mount")),
            Some(Path::new("/asked")),
        );
        assert_eq!(at[0], Path::new("/asked"));
        assert_eq!(at[1], Path::new("/tmp/.mount/usr/lib/Dactile/ocr"));
        assert!(at.contains(&PathBuf::from("/usr/bin/../lib/dactile/ocr")));
    }

    #[test]
    fn pages_as_pdfinfo_says() {
        assert_eq!(pages_said("Title: x\nPages:           138\n"), Some(138));
        assert_eq!(pages_said("Title: x\n"), None);
    }
}
