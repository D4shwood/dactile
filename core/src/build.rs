//! Building a play's PDF with XeLaTeX, in one run.
//!
//! The class shows who is in each scene under its heading, but only learns it
//! at the scene's end, so it keeps the casts in the `.aux` file for the next
//! run. Dactile knows them already, from the outline: it writes them into the
//! `.aux` before the run, and one run is enough. Should the class still find
//! another cast than its heading showed, it says "Rerun", and the build runs
//! once more.

use crate::cast::Cast;
use crate::outline::Outline;
use crate::{class, config, file, lang, lex, source, trash};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Never more runs than this, whatever the log says.
const MAX_RUNS: usize = 3;
/// A run that takes longer than this is stuck.
const TIMEOUT: Duration = Duration::from_secs(180);

/// The `\tiyatrocast` lines the class writes to the `.aux`, one per scene
/// with someone in it.
pub fn casts(outline: &Outline) -> String {
    outline
        .scenes
        .iter()
        .filter(|s| !s.cast.is_empty())
        .map(|s| {
            format!(
                "\\tiyatrocast{{{}}}{{{}}}\n",
                s.key(&outline.acts),
                s.cast.join(",")
            )
        })
        .collect()
}

/// The outline of a play's text.
pub fn outline_of(text: &str) -> Outline {
    let cast = Cast::of_play(text);
    let blocks = lex::blocks(text, source::split(text).body, &cast);
    Outline::of(text, &blocks, &cast)
}

pub fn aux_path(play: &Path) -> PathBuf {
    play.with_extension("aux")
}

pub fn pdf_path(play: &Path) -> PathBuf {
    play.with_extension("pdf")
}

/// Writes `casts` into the play's `.aux`, in place of the ones there, keeping
/// everything else in it. Without an `.aux`, makes one only if `create`.
pub fn seed(play: &Path, casts: &str, create: bool) -> anyhow::Result<()> {
    let aux = aux_path(play);
    let old = match fs::read_to_string(&aux) {
        Ok(t) => t,
        Err(_) if !create => return Ok(()),
        Err(_) => String::new(),
    };
    let new = seeded(&old, casts);
    if new != old {
        file::write_atomic(&aux, &new)?;
    }
    Ok(())
}

fn seeded(old: &str, casts: &str) -> String {
    let mut out: String = old
        .lines()
        .filter(|l| !l.starts_with("\\tiyatrocast{"))
        .map(|l| format!("{l}\n"))
        .collect();
    if out.is_empty() {
        out.push_str("\\relax\n");
    }
    out.push_str(casts);
    out
}

/// A finished build.
#[derive(Debug, Clone, PartialEq)]
pub struct Built {
    pub pdf: PathBuf,
    pub pages: Option<usize>,
    pub runs: usize,
    pub took: Duration,
}

/// A build that didn't make a PDF: what went wrong, and at which line of
/// the play, when it was the play.
#[derive(Debug, Clone, PartialEq)]
pub struct Failed {
    pub line: Option<usize>,
    pub message: String,
}

impl fmt::Display for Failed {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.line {
            Some(n) => write!(
                f,
                "{}",
                lang::f("{}. satır: {}", "line {}: {}", &[&n, &self.message])
            ),
            None => write!(f, "{}", self.message),
        }
    }
}

fn failed(message: String) -> Failed {
    Failed {
        line: None,
        message,
    }
}

/// Builds the play's PDF next to it, from the file as it is on disk, with
/// the class that came with this Dactile (a `tiyatro.cls` beside the play
/// still comes first).
pub fn build(play: &Path) -> Result<Built, Failed> {
    let start = Instant::now();
    let text = fs::read_to_string(play).map_err(|e| failed(format!("{}: {e}", play.display())))?;
    seed(play, &casts(&outline_of(&text)), true).map_err(|e| failed(format!("{e:#}")))?;
    let classes = class_dir().map_err(|e| failed(format!("{e:#}")))?;
    let dir = play
        .parent()
        .filter(|d| !d.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let name = play
        .file_name()
        .ok_or_else(|| failed("no file name".into()))?;
    let log_path = play.with_extension("log");
    // A table of contents is one run behind, like the casts without Dactile.
    let toc = text.contains("\\tableofcontents");
    let path = format!(".:{}:", classes.display());
    let mut runs = 0;
    loop {
        runs += 1;
        let log = xelatex(dir, name.as_ref(), &path, &log_path)?;
        let again = log.contains("Rerun to get") || (toc && runs == 1);
        if !again || runs >= MAX_RUNS {
            let pdf = pdf_path(play);
            if !pdf.exists() || log.contains("No pages of output") {
                return Err(failed(
                    lang::l(
                        "PDF çıkmadı: oyunda basılacak bir şey yok.",
                        "No PDF: nothing in the play to print.",
                    )
                    .into(),
                ));
            }
            return Ok(Built {
                pdf,
                pages: pages_in(&log),
                runs,
                took: start.elapsed(),
            });
        }
    }
}

/// One XeLaTeX run of `name` in `dir`, with TEXINPUTS `path`: its log, or
/// what stopped it.
fn xelatex(dir: &Path, name: &Path, path: &str, log_path: &Path) -> Result<String, Failed> {
    let mut child = outside("xelatex")
        .args([
            "-interaction=nonstopmode",
            "-halt-on-error",
            "-file-line-error",
            // For going between the play's lines and the PDF.
            "-synctex=1",
            // A play from someone else runs no programs, not even the few
            // TeX Live allows by default; the class needs none.
            "-no-shell-escape",
        ])
        .arg(name)
        .current_dir(dir)
        .env("TEXINPUTS", path)
        // The class looks for fonts that may not be installed (Aptos,
        // Calibri); without this each miss runs METAFONT for nothing, and a
        // short play builds in twice the time.
        .env("MKTEXTFM", "0")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| {
            failed(if e.kind() == std::io::ErrorKind::NotFound {
                lang::l(
                    "XeLaTeX bulunamadı: TeX Live'ı kur (xelatex komutu).",
                    "XeLaTeX not found: install TeX Live (the xelatex command).",
                )
                .into()
            } else {
                format!("xelatex: {e}")
            })
        })?;
    let begun = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break s,
            Ok(None) if begun.elapsed() > TIMEOUT => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(failed(
                    lang::l(
                        "XeLaTeX takıldı; durduruldu.",
                        "XeLaTeX got stuck; stopped.",
                    )
                    .into(),
                ));
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(50)),
            Err(e) => return Err(failed(format!("xelatex: {e}"))),
        }
    };
    let log = fs::read(log_path)
        .map(|b| String::from_utf8_lossy(&b).into_owned())
        .unwrap_or_default();
    if !status.success() {
        return Err(error_in(&log, &name.to_string_lossy()).unwrap_or_else(|| {
            failed(
                lang::l(
                    "XeLaTeX durdu; ayrıntı .log dosyasında.",
                    "XeLaTeX stopped; the .log has the details.",
                )
                .into(),
            )
        }));
    }
    Ok(log)
}

/// Builds a document made from a play (an actor's copy, the scene table) in
/// a folder of its own, and puts its PDF at `out`. The play's folder is
/// searched after it, for what the play takes from there; `aux` seeds its
/// `.aux`.
pub fn build_aside(play: &Path, tex: &str, aux: &str, out: &Path) -> Result<Built, Failed> {
    let start = Instant::now();
    let classes = class_dir().map_err(|e| failed(format!("{e:#}")))?;
    let work = cache()
        .map_err(|e| failed(format!("{e:#}")))?
        .join(format!("work-{}", std::process::id()));
    let _ = fs::remove_dir_all(&work);
    let made = (|| -> anyhow::Result<PathBuf> {
        fs::create_dir_all(&work)?;
        let src = work.join("dactile-aside.tex");
        fs::write(&src, tex)?;
        seed(&src, aux, true)?;
        Ok(src)
    })()
    .map_err(|e| failed(format!("{e:#}")));
    let result = made.and_then(|src| {
        let home = play
            .parent()
            .filter(|d| !d.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let home = fs::canonicalize(home).unwrap_or(home.to_path_buf());
        let path = format!(".:{}:{}:", home.display(), classes.display());
        let name = Path::new("dactile-aside.tex");
        let log_path = src.with_extension("log");
        let mut runs = 0;
        loop {
            runs += 1;
            let log = xelatex(&work, name, &path, &log_path)?;
            if !(log.contains("Rerun") && runs < MAX_RUNS) {
                if let Some(d) = out.parent() {
                    fs::create_dir_all(d).map_err(|e| failed(format!("{}: {e}", d.display())))?;
                }
                fs::copy(src.with_extension("pdf"), out)
                    .map_err(|_| failed(lang::l("PDF çıkmadı.", "No PDF came out.").into()))?;
                return Ok(Built {
                    pdf: out.to_path_buf(),
                    pages: pages_in(&log),
                    runs,
                    took: start.elapsed(),
                });
            }
        }
    });
    let _ = fs::remove_dir_all(&work);
    result
}

/// The first error in a log written with `-file-line-error`: `./play.tex:42:
/// Undefined control sequence.`, or a plain `! …` one.
pub fn error_in(log: &str, play_name: &str) -> Option<Failed> {
    for l in log.lines() {
        if let Some((file, n, msg)) = file_line(l) {
            let ours = Path::new(file).file_name() == Some(play_name.as_ref());
            return Some(Failed {
                line: ours.then_some(n),
                message: msg.trim().to_string(),
            });
        }
        if let Some(msg) = l.strip_prefix("! ") {
            return Some(failed(msg.trim().to_string()));
        }
    }
    None
}

/// `./a.tex:42: what` → ("./a.tex", 42, "what").
fn file_line(l: &str) -> Option<(&str, usize, &str)> {
    let mut from = 0;
    while let Some(i) = l[from..].find(':') {
        let at = from + i;
        let rest = &l[at + 1..];
        let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
        if at > 0 && digits > 0 && rest[digits..].starts_with(": ") {
            return Some((&l[..at], rest[..digits].parse().ok()?, &rest[digits + 2..]));
        }
        from = at + 1;
    }
    None
}

/// "Output written on play.pdf (12 pages, …)" → 12.
fn pages_in(log: &str) -> Option<usize> {
    let at = log.find("Output written on ")?;
    let rest = &log[at..];
    let open = rest.find('(')?;
    rest[open + 1..].split_whitespace().next()?.parse().ok()
}

/// `~/.cache/dactile`. Never a shared folder like `/tmp`, where someone
/// else could put a class of their own first.
fn cache() -> anyhow::Result<PathBuf> {
    dirs::cache_dir()
        .map(|d| d.join("dactile"))
        .ok_or_else(|| anyhow::anyhow!("no cache folder (HOME is not set)"))
}

/// The class files of this Dactile, written once into the cache.
fn class_dir() -> anyhow::Result<PathBuf> {
    let dir = cache()?.join(format!("tex-{}", class::version()));
    for (name, body) in class::FILES {
        let p = dir.join(name);
        if fs::read_to_string(&p).ok().as_deref() != Some(body) {
            fs::create_dir_all(&dir)?;
            file::write_atomic(&p, body)?;
        }
    }
    Ok(dir)
}

/// Opens the PDF in the desktop's viewer, without waiting for it.
pub fn open(pdf: &Path) -> anyhow::Result<()> {
    if !pdf.exists() {
        anyhow::bail!(
            "{}",
            lang::l(
                "Henüz PDF yok: önce F5 ile derle.",
                "No PDF yet: build it with F5 first."
            )
        );
    }
    launch(pdf)
}

/// Opens the last build's log, where XeLaTeX says what stopped it.
pub fn open_log(play: &Path) -> anyhow::Result<()> {
    let log = play.with_extension("log");
    if !log.exists() {
        anyhow::bail!(
            "{}",
            lang::l(
                "Henüz log yok: önce F5 ile derle.",
                "No log yet: build with F5 first."
            )
        );
    }
    launch(&log)
}

/// Why a file that was there can't be opened any more.
fn still_there(path: &Path) -> anyhow::Result<()> {
    if !path.exists() {
        anyhow::bail!(
            "{}",
            lang::f(
                "{} yok: taşınmış ya da silinmiş olabilir.",
                "{} is not there: it may have moved or been deleted.",
                &[&config::tilde(path)]
            )
        );
    }
    Ok(())
}

/// The desktop's own program for the file, without waiting for it.
/// Opens a file or a folder with the desktop's own program for it.
pub fn launch(path: &Path) -> anyhow::Result<()> {
    still_there(path)?;
    let mut child = outside("xdg-open")
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    std::thread::spawn(move || child.wait());
    Ok(())
}

/// Shows a file selected in the desktop's file manager; if none answers,
/// opens its folder. Waits for the answer, a few seconds at worst: a window
/// calls this off its own thread.
pub fn reveal(path: &Path) -> anyhow::Result<()> {
    still_there(path)?;
    let path = std::path::absolute(path)?;
    if show_items(&file_uri(&path)) {
        return Ok(());
    }
    launch(path.parent().unwrap_or(&path))
}

/// `/a b/ç.pdf` → `file:///a%20b/%C3%A7.pdf`.
fn file_uri(path: &Path) -> String {
    format!(
        "file://{}",
        trash::encode(path.as_os_str().as_encoded_bytes())
    )
}

/// Asks for `org.freedesktop.FileManager1.ShowItems`: whether it was done.
/// The URI goes in as a GVariant string: encoded, it has no quote to break it.
fn show_items(uri: &str) -> bool {
    outside("gdbus")
        .args(["call", "--session", "--timeout", "5"])
        .args(["--dest", "org.freedesktop.FileManager1"])
        .args(["--object-path", "/org/freedesktop/FileManager1"])
        .args(["--method", "org.freedesktop.FileManager1.ShowItems"])
        .arg(format!("['{uri}']"))
        .arg("''")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// A program of the system's. An AppImage's start script points the
/// libraries, Perl, Python and GTK at what is inside it, for the app alone:
/// XeLaTeX or a PDF viewer would load those instead of their own. So a child
/// gets the environment as it was.
pub fn outside(program: &str) -> Command {
    let mut c = Command::new(program);
    if let Some(dir) = std::env::var("APPDIR").ok().filter(|d| !d.is_empty()) {
        c.env_remove("GTK_THEME");
        for (name, value) in std::env::vars_os() {
            match without(&value.to_string_lossy(), &dir) {
                None => {}
                Some(v) if v.is_empty() => {
                    c.env_remove(name);
                }
                Some(v) => {
                    c.env(name, v);
                }
            }
        }
    }
    c
}

/// A variable's value with the paths inside `dir` gone, if it had any.
fn without(value: &str, dir: &str) -> Option<String> {
    value.contains(dir).then(|| {
        value
            .split(':')
            .filter(|p| !p.contains(dir))
            .collect::<Vec<_>>()
            .join(":")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appimage_paths_stay_inside() {
        let d = "/tmp/.mount_DactilX";
        assert_eq!(
            without("/tmp/.mount_DactilX/usr/bin/:/usr/bin:/bin", d).as_deref(),
            Some("/usr/bin:/bin")
        );
        assert_eq!(
            without("/tmp/.mount_DactilX//usr/lib/gtk-3.0", d).as_deref(),
            Some("")
        );
        assert_eq!(without("/usr/bin", d), None);
    }

    #[test]
    fn file_uris_are_percent_encoded() {
        assert_eq!(
            file_uri(Path::new("/ev/Yazı İşleri/şğüöçı oyun.pdf")),
            "file:///ev/Yaz%C4%B1%20%C4%B0%C5%9Fleri/%C5%9F%C4%9F%C3%BC%C3%B6%C3%A7%C4%B1%20oyun.pdf"
        );
        // Nothing that would break the URI or the GVariant around it.
        assert_eq!(
            file_uri(Path::new("/a'b/100%,[x]#?.pdf")),
            "file:///a%27b/100%25%2C%5Bx%5D%23%3F.pdf"
        );
        assert_eq!(file_uri(Path::new("/a-b_c.d~e")), "file:///a-b_c.d~e");
    }

    #[test]
    fn nothing_to_show_says_so() {
        let gone = Path::new("/nonexistent-dactile/oyun.pdf");
        assert!(reveal(gone).is_err());
        assert!(launch(gone).is_err());
    }

    const PLAY: &str = include_str!("../../examples/ornek-oyun.tex");

    #[test]
    fn casts_as_the_class_writes_them() {
        assert_eq!(
            casts(&outline_of(PLAY)),
            "\\tiyatrocast{I-1}{se,oz,ih}\n\\tiyatrocast{I-2}{ka,co,hep,se}\n\\tiyatrocast{II-1}{oz,ih}\n"
        );
    }

    #[test]
    fn seeding_keeps_the_rest_of_the_aux() {
        let c = "\\tiyatrocast{I-1}{a,b}\n";
        assert_eq!(seeded("", c), "\\relax\n\\tiyatrocast{I-1}{a,b}\n");
        let old = "\\relax\n\\tiyatrocast{I-1}{a}\n\\@writefile{toc}{x}\n\\tiyatrocast{I-2}{b}\n\\gdef \\@abspage@last{3}\n";
        assert_eq!(
            seeded(old, c),
            "\\relax\n\\@writefile{toc}{x}\n\\gdef \\@abspage@last{3}\n\\tiyatrocast{I-1}{a,b}\n"
        );
        let d = std::env::temp_dir().join(format!("dactile-test-seed-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        let play = d.join("oyun.tex");
        seed(&play, c, false).unwrap();
        assert!(!aux_path(&play).exists());
        seed(&play, c, true).unwrap();
        assert_eq!(
            fs::read_to_string(aux_path(&play)).unwrap(),
            "\\relax\n\\tiyatrocast{I-1}{a,b}\n"
        );
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn errors_from_the_log() {
        let log = "This is XeTeX\n./oyun.tex:42: Undefined control sequence.\nl.42 \\foo\n";
        assert_eq!(
            error_in(log, "oyun.tex"),
            Some(Failed {
                line: Some(42),
                message: "Undefined control sequence.".into()
            })
        );
        // In the class, not the play: no line to go to.
        let log = "./tiyatro-core.sty:300: Missing } inserted.\n";
        assert_eq!(error_in(log, "oyun.tex").unwrap().line, None);
        assert_eq!(
            error_in("! Emergency stop.\n", "oyun.tex").unwrap().message,
            "Emergency stop."
        );
        assert_eq!(error_in("nothing\n", "oyun.tex"), None);
        assert_eq!(
            pages_in("Output written on oyun.pdf (12 pages, 3456 bytes)."),
            Some(12)
        );
    }
}
