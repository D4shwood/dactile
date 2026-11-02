//! A Word document: `word/document.xml` paragraph by paragraph, italics and
//! bold from the runs and their styles, the title and author from
//! `docProps/core.xml`.

use super::xml::{self, Element, Kid};
use super::{Format, Para, Run, Source};
use anyhow::{Context, Result};
use std::collections::HashMap;

#[derive(Default, Clone, Copy)]
struct Style {
    italic: Option<bool>,
    bold: Option<bool>,
    heading: bool,
    title: bool,
}

struct Styles(HashMap<String, (Style, Option<String>)>);

impl Styles {
    fn read(xml: Option<&str>) -> Result<Styles> {
        let mut map = HashMap::new();
        if let Some(xml) = xml {
            let root = xml::parse(xml)?;
            for s in root.elements().filter(|e| e.name == "w:style") {
                let Some(id) = s.attr("w:styleId") else {
                    continue;
                };
                let name = s
                    .child("w:name")
                    .and_then(|n| n.attr("w:val"))
                    .unwrap_or("")
                    .to_lowercase();
                let mut st = s.child("w:rPr").map(props).unwrap_or_default();
                st.heading = name.starts_with("heading")
                    || name.starts_with("başlık")
                    || s.child("w:pPr")
                        .is_some_and(|p| p.child("w:outlineLvl").is_some());
                st.title = id == "Title" || name == "title" || name == "konu başlığı";
                let based = s
                    .child("w:basedOn")
                    .and_then(|b| b.attr("w:val"))
                    .map(str::to_string);
                map.insert(id.to_string(), (st, based));
            }
        }
        Ok(Styles(map))
    }

    /// A style with what it inherits filled in.
    fn get(&self, id: &str) -> Style {
        let mut out = Style::default();
        let mut at = Some(id.to_string());
        let mut depth = 0;
        while let Some(id) = at {
            let Some((s, based)) = self.0.get(&id) else {
                break;
            };
            out.italic = out.italic.or(s.italic);
            out.bold = out.bold.or(s.bold);
            out.heading |= s.heading;
            out.title |= s.title;
            at = based.clone();
            depth += 1;
            if depth > 20 {
                break;
            }
        }
        out
    }
}

/// `<w:i/>` is on, `<w:i w:val="0"/>` off.
fn toggle(e: &Element) -> bool {
    !matches!(e.attr("w:val"), Some("0" | "false" | "off" | "none"))
}

fn props(rpr: &Element) -> Style {
    Style {
        italic: rpr.child("w:i").map(toggle),
        bold: rpr.child("w:b").map(toggle),
        ..Style::default()
    }
}

pub fn read(bytes: &[u8]) -> Result<Source> {
    let mut zip = super::open_zip(bytes)?;
    let doc = super::member(&mut zip, "word/document.xml")?
        .context("no word/document.xml: not a Word document")?;
    let styles = Styles::read(super::member(&mut zip, "word/styles.xml")?.as_deref())?;
    let root = xml::parse(&doc)?;
    let body = root.find("w:body").context("no document body")?;
    let mut paras = Vec::new();
    block(body, &styles, &mut paras);
    let (mut title, mut author) = (None, None);
    if let Some(core) = super::member(&mut zip, "docProps/core.xml")? {
        let core = xml::parse(&core)?;
        title = core.find("dc:title").map(Element::text);
        author = core.find("dc:creator").map(Element::text);
    }
    Ok(Source {
        format: Format::Docx,
        paras,
        title,
        author,
    })
}

/// Paragraphs and tables, in order, through content controls.
fn block(e: &Element, styles: &Styles, out: &mut Vec<Para>) {
    for k in e.elements() {
        match k.name.as_str() {
            "w:p" => out.push(para(k, styles)),
            "w:tbl" => {
                for row in k.elements().filter(|r| r.name == "w:tr") {
                    let cells = row
                        .elements()
                        .filter(|c| c.name == "w:tc")
                        .map(|c| {
                            let mut ps = Vec::new();
                            block(c, styles, &mut ps);
                            ps.into_iter()
                                .map(Para::tidy)
                                .filter(|p| !p.runs.is_empty())
                                .collect()
                        })
                        .collect();
                    out.extend(super::table_row(cells));
                }
            }
            "w:sdt" | "w:sdtContent" | "w:customXml" => block(k, styles, out),
            _ => {}
        }
    }
}

fn para(p: &Element, styles: &Styles) -> Para {
    let pstyle = p
        .child("w:pPr")
        .and_then(|ppr| ppr.child("w:pStyle"))
        .and_then(|s| s.attr("w:val"))
        .map(|id| styles.get(id))
        .unwrap_or_default();
    let mut out = Para {
        heading: pstyle.heading,
        title: pstyle.title,
        ..Para::default()
    };
    runs(p, styles, &pstyle, &mut out.runs);
    out
}

/// The runs inside `e`, through links, insertions and fields; not what
/// was deleted with changes tracked.
fn runs(e: &Element, styles: &Styles, pstyle: &Style, out: &mut Vec<Run>) {
    for k in e.elements() {
        match k.name.as_str() {
            "w:r" => {
                let rpr = k.child("w:rPr");
                let own = rpr.map(props).unwrap_or_default();
                let cstyle = rpr
                    .and_then(|r| r.child("w:rStyle"))
                    .and_then(|s| s.attr("w:val"))
                    .map(|id| styles.get(id))
                    .unwrap_or_default();
                let italic = own
                    .italic
                    .or(cstyle.italic)
                    .or(pstyle.italic)
                    .unwrap_or(false);
                let bold = own.bold.or(cstyle.bold).or(pstyle.bold).unwrap_or(false);
                let mut text = String::new();
                for part in &k.kids {
                    let Kid::El(part) = part else { continue };
                    match part.name.as_str() {
                        "w:t" => text.push_str(&part.text()),
                        "w:tab" => text.push(' '),
                        "w:br" if part.attr("w:type").is_none_or(|t| t == "textWrapping") => {
                            text.push('\n')
                        }
                        "w:cr" => text.push('\n'),
                        "w:noBreakHyphen" => text.push('-'),
                        _ => {}
                    }
                }
                out.push(Run { text, italic, bold });
            }
            "w:del" | "w:pPr" | "w:rPr" | "w:moveFrom" => {}
            _ => runs(k, styles, pstyle, out),
        }
    }
}
