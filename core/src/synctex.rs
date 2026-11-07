//! From a line of the play to its place in the PDF and back, by the `synctex`
//! program that comes with TeX Live. XeLaTeX leaves `play.synctex.gz` beside
//! the PDF.

use crate::{build, lang};
use std::path::{Path, PathBuf};
use std::process::Stdio;

/// A box of the PDF for a line. In points from the page's top left.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Place {
    pub page: usize,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// The line behind a point of the PDF.
#[derive(Debug, Clone, PartialEq)]
pub struct Source {
    pub file: PathBuf,
    pub line: usize,
}

/// The places `line` of `play` was set at, in its PDF; the first one first.
pub fn view(play: &Path, line: usize, column: usize) -> anyhow::Result<Vec<Place>> {
    let input = format!("{line}:{column}:{}", play.display());
    let pdf = build::pdf_path(play);
    let out = run(&["view", "-i", &input, "-o", &pdf.to_string_lossy()])?;
    let places = parse_view(&out);
    if places.is_empty() {
        anyhow::bail!(
            "{}",
            lang::l(
                "Bu satırın PDF'te bir yeri yok.",
                "This line has no place in the PDF."
            )
        );
    }
    Ok(places)
}

/// The line behind a point of `pdf`, in points from the page's top left.
pub fn edit(pdf: &Path, page: usize, x: f32, y: f32) -> anyhow::Result<Source> {
    let output = format!("{page}:{x}:{y}:{}", pdf.display());
    let out = run(&["edit", "-o", &output])?;
    parse_edit(&out).ok_or_else(|| {
        anyhow::anyhow!(
            "{}",
            lang::l(
                "PDF'in bu yeri bir satıra bağlı değil.",
                "This place in the PDF has no line."
            )
        )
    })
}

fn run(args: &[&str]) -> anyhow::Result<String> {
    let out = build::outside("synctex")
        .args(args)
        .stdin(Stdio::null())
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                anyhow::anyhow!(
                    "{}",
                    lang::l(
                        "synctex bulunamadı: TeX Live'ı kur (synctex komutu).",
                        "synctex not found: install TeX Live (the synctex command).",
                    )
                )
            } else {
                anyhow::anyhow!("synctex: {e}")
            }
        })?;
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// The records between "SyncTeX result begin" and "end": one line for each
/// field, and a new record at each `Output:`. The same box comes more than
/// once.
pub fn parse_view(text: &str) -> Vec<Place> {
    let mut out: Vec<Place> = Vec::new();
    let mut page = None;
    let (mut h, mut v, mut w) = (None, None, None);
    for l in result(text) {
        let Some((key, value)) = l.split_once(':') else {
            continue;
        };
        match key {
            "Page" => page = value.parse::<usize>().ok(),
            "h" => h = value.parse::<f32>().ok(),
            "v" => v = value.parse::<f32>().ok(),
            "W" => w = value.parse::<f32>().ok(),
            "H" => {
                let height = value.parse::<f32>().ok();
                if let (Some(page), Some(x), Some(v), Some(width), Some(height)) =
                    (page, h, v, w, height)
                {
                    // `v` is the box's foot.
                    let place = Place {
                        page,
                        x,
                        y: v - height,
                        width,
                        height,
                    };
                    if !out.contains(&place) {
                        out.push(place);
                    }
                }
            }
            _ => {}
        }
    }
    out
}

pub fn parse_edit(text: &str) -> Option<Source> {
    let mut file = None;
    let mut line = None;
    for l in result(text) {
        if let Some(f) = l.strip_prefix("Input:") {
            file = Some(PathBuf::from(f));
        } else if let Some(n) = l.strip_prefix("Line:") {
            line = n.parse::<usize>().ok().filter(|n| *n > 0);
        }
    }
    Some(Source {
        file: tidy(&file?),
        line: line?,
    })
}

/// `/a/./b.tex` → `/a/b.tex`: TeX keeps the `./` it was given.
fn tidy(p: &Path) -> PathBuf {
    p.components().collect()
}

fn result(text: &str) -> impl Iterator<Item = &str> {
    text.lines()
        .skip_while(|l| !l.starts_with("SyncTeX result begin"))
        .take_while(|l| !l.starts_with("SyncTeX result end"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const VIEW: &str = "This is SyncTeX command line utility, version 1.5
Negative width
SyncTeX result begin
Output:/ev/Çay Saati/örnek oyun.pdf
Page:5
x:85.039375
y:587.955078
h:69.871643
v:590.435974
W:440.364594
H:11.184605
before:
offset:-1
middle:
after:
Output:/ev/Çay Saati/örnek oyun.pdf
Page:5
x:69.871643
y:587.955078
h:69.871643
v:590.435974
W:440.364594
H:11.184605
before:
offset:-1
middle:
after:
Output:/ev/Çay Saati/örnek oyun.pdf
Page:6
x:70.0
y:100.0
h:70.0
v:112.5
W:100.0
H:12.5
before:
offset:-1
middle:
after:
SyncTeX result end
";

    const EDIT: &str = "This is SyncTeX command line utility, version 1.5
Negative width
SyncTeX result begin
Output:/ev/Çay Saati/örnek oyun.pdf
Input:/ev/Çay Saati/./örnek oyun.tex
Line:32
Column:-1
Offset:0
Context:
SyncTeX result end
";

    #[test]
    fn places_of_a_line() {
        let p = parse_view(VIEW);
        assert_eq!(p.len(), 2);
        assert_eq!(p[0].page, 5);
        assert!((p[0].x - 69.871_64).abs() < 0.001);
        assert!((p[0].y - 579.251_4).abs() < 0.001);
        assert!((p[0].width - 440.364_6).abs() < 0.001);
        assert_eq!(p[1].page, 6);
        assert_eq!(p[1].y, 100.0);
    }

    #[test]
    fn line_of_a_place() {
        let s = parse_edit(EDIT).unwrap();
        assert_eq!(s.line, 32);
        assert_eq!(s.file, Path::new("/ev/Çay Saati/örnek oyun.tex"));
    }

    #[test]
    fn nothing_found() {
        let none = "This is SyncTeX command line utility, version 1.5\n";
        assert!(parse_view(none).is_empty());
        assert!(parse_edit(none).is_none());
        let begun = "SyncTeX result begin\nInput:/a.tex\nLine:0\nSyncTeX result end\n";
        assert!(parse_edit(begun).is_none());
    }
}
