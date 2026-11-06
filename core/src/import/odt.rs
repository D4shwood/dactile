//! An OpenDocument text: `content.xml` paragraph by paragraph, italics and
//! bold from the styles (`content.xml`'s own and `styles.xml`'s), the title
//! and author from `meta.xml`.

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

#[derive(Default)]
struct Styles(HashMap<String, (Style, Option<String>)>);

impl Styles {
    fn add(&mut self, root: &Element) {
        for group in ["office:styles", "office:automatic-styles"] {
            let Some(g) = root.find(group) else { continue };
            for s in g.elements().filter(|e| e.name == "style:style") {
                let Some(name) = s.attr("style:name") else {
                    continue;
                };
                let mut st = Style::default();
                if let Some(t) = s.child("style:text-properties") {
                    st.italic = t
                        .attr("fo:font-style")
                        .map(|v| v == "italic" || v == "oblique");
                    st.bold = t
                        .attr("fo:font-weight")
                        .map(|v| v == "bold" || v.parse::<u32>().is_ok_and(|n| n >= 600));
                }
                let shown = s.attr("style:display-name").unwrap_or(name).to_lowercase();
                st.heading = shown.starts_with("heading") || shown.starts_with("başlık");
                st.title = shown == "title" || shown == "konu başlığı";
                let parent = s.attr("style:parent-style-name").map(str::to_string);
                self.0.insert(name.to_string(), (st, parent));
            }
        }
    }

    fn get(&self, name: &str) -> Style {
        let mut out = Style::default();
        let mut at = Some(name.to_string());
        let mut depth = 0;
        while let Some(n) = at {
            let Some((s, parent)) = self.0.get(&n) else {
                break;
            };
            out.italic = out.italic.or(s.italic);
            out.bold = out.bold.or(s.bold);
            out.heading |= s.heading;
            out.title |= s.title;
            at = parent.clone();
            depth += 1;
            if depth > 20 {
                break;
            }
        }
        out
    }
}

pub fn read(bytes: &[u8]) -> Result<Source> {
    let mut zip = super::open_zip(bytes)?;
    let content = super::member(&mut zip, "content.xml")?
        .context("no content.xml: not an OpenDocument text")?;
    let mut styles = Styles::default();
    if let Some(s) = super::member(&mut zip, "styles.xml")? {
        styles.add(&xml::parse(&s)?);
    }
    let root = xml::parse(&content)?;
    styles.add(&root);
    let text = root
        .find("office:text")
        .context("no text in the document")?;
    let mut paras = Vec::new();
    block(text, &styles, &mut paras);
    let (mut title, mut author) = (None, None);
    if let Some(meta) = super::member(&mut zip, "meta.xml")? {
        let meta = xml::parse(&meta)?;
        title = meta.find("dc:title").map(Element::text);
        author = meta
            .find("dc:creator")
            .or_else(|| meta.find("meta:initial-creator"))
            .map(Element::text);
    }
    Ok(Source {
        format: Format::Odt,
        paras,
        title,
        author,
        pages: Vec::new(),
        body_page: None,
        plays: Vec::new(),
    })
}

fn block(e: &Element, styles: &Styles, out: &mut Vec<Para>) {
    for k in e.elements() {
        match k.name.as_str() {
            "text:p" | "text:h" => {
                let st = k
                    .attr("text:style-name")
                    .map(|n| styles.get(n))
                    .unwrap_or_default();
                let mut p = Para {
                    heading: k.name == "text:h" || st.heading,
                    title: st.title,
                    ..Para::default()
                };
                let base = Run {
                    italic: st.italic.unwrap_or(false),
                    bold: st.bold.unwrap_or(false),
                    ..Run::default()
                };
                runs(k, styles, &base, &mut p.runs);
                out.push(p);
            }
            "table:table" => {
                for row in rows(k) {
                    let cells = row
                        .elements()
                        .filter(|c| c.name == "table:table-cell")
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
            "text:list" | "text:list-item" | "text:list-header" | "text:section" => {
                block(k, styles, out)
            }
            _ => {}
        }
    }
}

/// A table's rows, through row groups.
fn rows(t: &Element) -> Vec<&Element> {
    let mut out = Vec::new();
    for k in t.elements() {
        match k.name.as_str() {
            "table:table-row" => out.push(k),
            "table:table-rows" | "table:table-header-rows" | "table:table-row-group" => {
                out.extend(rows(k))
            }
            _ => {}
        }
    }
    out
}

fn runs(e: &Element, styles: &Styles, style: &Run, out: &mut Vec<Run>) {
    for k in &e.kids {
        match k {
            // A line end in the XML is a space; a break is text:line-break.
            Kid::Text(t) => out.push(Run {
                text: t.replace(['\n', '\r'], " "),
                ..style.clone()
            }),
            Kid::El(k) => match k.name.as_str() {
                "text:span" => {
                    let st = k
                        .attr("text:style-name")
                        .map(|n| styles.get(n))
                        .unwrap_or_default();
                    let inner = Run {
                        text: String::new(),
                        italic: st.italic.unwrap_or(style.italic),
                        bold: st.bold.unwrap_or(style.bold),
                        note: false,
                    };
                    runs(k, styles, &inner, out);
                }
                "text:s" => {
                    let n = k.attr("text:c").and_then(|c| c.parse().ok()).unwrap_or(1);
                    out.push(Run {
                        text: " ".repeat(n),
                        ..style.clone()
                    });
                }
                "text:tab" => out.push(Run {
                    text: " ".into(),
                    ..style.clone()
                }),
                "text:line-break" => out.push(Run {
                    text: "\n".into(),
                    ..style.clone()
                }),
                "text:note" | "text:bookmark" | "office:annotation" | "text:soft-page-break" => {}
                _ => runs(k, styles, style, out),
            },
        }
    }
}
