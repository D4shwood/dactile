//! A play as a Word document, for those who read it outside Dactile: the same
//! parts in the same order as the PDF, in styles a word processor can change.

use crate::cast::{self, Cast, Row};
use crate::info::{self, Side};
use crate::lang;
use crate::lex::{self, Inline, Kind};
use crate::notes::{self, Notes, State};
use crate::outline::roman;
use crate::render::{Doc, move_line};
use crate::shelf;
use crate::tex;
use anyhow::Result;
use std::collections::BTreeMap;
use std::io::{Cursor, Write};
use std::ops::Range;
use std::path::{Path, PathBuf};
use zip::write::SimpleFileOptions;

/// What tells a document made here from one someone else wrote.
const MADE_BY: &str = "Dactile";

const CONTENT_TYPES: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/><Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/><Override PartName="/docProps/core.xml" ContentType="application/vnd.openxmlformats-package.core-properties+xml"/><Override PartName="/docProps/app.xml" ContentType="application/vnd.openxmlformats-officedocument.extended-properties+xml"/></Types>"#;

const RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties" Target="docProps/core.xml"/><Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/extended-properties" Target="docProps/app.xml"/></Relationships>"#;

const DOC_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/></Relationships>"#;

/// The notes as Word's own comments, beside the lines they are about.
const COMMENTS_TYPE: &str = r#"<Override PartName="/word/comments.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.comments+xml"/>"#;
const COMMENTS_REL: &str = r#"<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/comments" Target="comments.xml"/>"#;

const STYLES: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:docDefaults><w:rPrDefault><w:rPr><w:rFonts w:ascii="Liberation Serif" w:hAnsi="Liberation Serif" w:cs="Liberation Serif"/><w:sz w:val="24"/><w:lang w:val="tr-TR"/></w:rPr></w:rPrDefault><w:pPrDefault><w:pPr><w:spacing w:after="120"/></w:pPr></w:pPrDefault></w:docDefaults><w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/></w:style><w:style w:type="paragraph" w:styleId="Title"><w:name w:val="Title"/><w:basedOn w:val="Normal"/><w:pPr><w:spacing w:before="2400" w:after="240"/><w:jc w:val="center"/></w:pPr><w:rPr><w:b/><w:sz w:val="48"/></w:rPr></w:style><w:style w:type="paragraph" w:styleId="Author"><w:name w:val="Author"/><w:basedOn w:val="Normal"/><w:pPr><w:spacing w:after="720"/><w:jc w:val="center"/></w:pPr></w:style><w:style w:type="paragraph" w:styleId="Heading1"><w:name w:val="heading 1"/><w:basedOn w:val="Normal"/><w:next w:val="Normal"/><w:pPr><w:keepNext/><w:spacing w:before="480" w:after="240"/><w:jc w:val="center"/><w:outlineLvl w:val="0"/></w:pPr><w:rPr><w:b/><w:sz w:val="30"/></w:rPr></w:style><w:style w:type="paragraph" w:styleId="Heading2"><w:name w:val="heading 2"/><w:basedOn w:val="Normal"/><w:next w:val="Normal"/><w:pPr><w:keepNext/><w:spacing w:before="360" w:after="60"/><w:outlineLvl w:val="1"/></w:pPr><w:rPr><w:b/><w:sz w:val="26"/></w:rPr></w:style><w:style w:type="paragraph" w:styleId="SceneCast"><w:name w:val="Scene Cast"/><w:basedOn w:val="Normal"/><w:pPr><w:keepNext/><w:spacing w:after="240"/></w:pPr><w:rPr><w:smallCaps/><w:sz w:val="21"/></w:rPr></w:style><w:style w:type="paragraph" w:styleId="Speech"><w:name w:val="Speech"/><w:basedOn w:val="Normal"/><w:pPr><w:tabs><w:tab w:val="left" w:pos="1701"/></w:tabs><w:ind w:left="1701" w:hanging="1701"/></w:pPr></w:style><w:style w:type="paragraph" w:styleId="Direction"><w:name w:val="Direction"/><w:basedOn w:val="Normal"/><w:rPr><w:i/></w:rPr></w:style></w:styles>"#;

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[derive(Clone, Copy)]
struct Look {
    bold: bool,
    italic: bool,
    under: bool,
}

const PLAIN: Look = Look {
    bold: false,
    italic: false,
    under: false,
};
const BOLD: Look = Look {
    bold: true,
    ..PLAIN
};
const ITALIC: Look = Look {
    italic: true,
    ..PLAIN
};

/// One run of text; a line break inside it stays a line break.
fn run(text: &str, look: Look) -> String {
    let mut props = String::new();
    if look.bold {
        props.push_str("<w:b/>");
    }
    if look.italic {
        props.push_str("<w:i/>");
    }
    if look.under {
        props.push_str(r#"<w:u w:val="single"/>"#);
    }
    let mut body = String::new();
    for (i, part) in text.split('\n').enumerate() {
        if i > 0 {
            body.push_str("<w:br/>");
        }
        for (j, piece) in part.split('\t').enumerate() {
            if j > 0 {
                body.push_str("<w:tab/>");
            }
            if !piece.is_empty() {
                body.push_str(&format!(
                    r#"<w:t xml:space="preserve">{}</w:t>"#,
                    escape(piece)
                ));
            }
        }
    }
    if body.is_empty() {
        return String::new();
    }
    format!("<w:r><w:rPr>{props}</w:rPr>{body}</w:r>")
}

fn para(style: &str, extra: &str, runs: &str) -> String {
    format!(r#"<w:p><w:pPr><w:pStyle w:val="{style}"/>{extra}</w:pPr>{runs}</w:p>"#)
}

/// A paragraph of one plain run.
fn line(style: &str, text: &str) -> String {
    para(style, "", &run(text, PLAIN))
}

fn align(side: Side) -> &'static str {
    match side {
        Side::Left => r#"<w:jc w:val="left"/>"#,
        Side::Right => r#"<w:jc w:val="right"/>"#,
        Side::Center => r#"<w:jc w:val="center"/>"#,
    }
}

/// Source text as printed: comments gone, a `%` joining its line to the next.
fn plain(text: &str) -> String {
    let mut out = String::new();
    let lines: Vec<&str> = text.split('\n').collect();
    for (i, line) in lines.iter().enumerate() {
        match tex::comment_start(line) {
            Some(at) => out.push_str(line[..at].trim_end()),
            None => {
                out.push_str(line);
                if i + 1 < lines.len() {
                    out.push('\n');
                }
            }
        }
    }
    out
}

fn special(s: &str) -> String {
    match s {
        "~" => "\u{a0}".into(),
        "--" => "–".into(),
        "---" => "—".into(),
        "\\\\" => "\n".into(),
        _ => s.trim_start_matches('\\').to_string(),
    }
}

/// A footnote stays where it stands, in brackets; anything else the app doesn't
/// know shows as it is.
fn raw(text: &str, span: Range<usize>, doc: &Doc, dialogue: bool, look: Look) -> String {
    let s = &text[span.clone()];
    let note = ["\\dipnot", "\\footnote"]
        .iter()
        .find(|c| s.starts_with(**c))
        .and_then(|c| tex::group(text, span.start + c.len()));
    match note {
        Some(g) => {
            let inner = inline(text, g.inner_span.clone(), doc, dialogue, look);
            format!("{}{inner}{}", run(" [", look), run("]", look))
        }
        None => run(s, look),
    }
}

fn inline(text: &str, range: Range<usize>, doc: &Doc, dialogue: bool, look: Look) -> String {
    let mut out = String::new();
    for p in lex::inline(text, range, &doc.cast, dialogue) {
        match p.kind {
            Inline::Text => out.push_str(&run(&plain(&text[p.span]), look)),
            Inline::Ref(alias) => {
                let name = doc.cast.inline_name(&alias).unwrap_or(alias);
                out.push_str(&run(&name, look));
            }
            Inline::Ic { inner } => {
                let look = Look {
                    italic: true,
                    ..look
                };
                out.push_str(&run("(", look));
                out.push_str(&inline(text, inner, doc, dialogue, look));
                out.push_str(&run(")", look));
            }
            Inline::Under { inner } => {
                let look = Look {
                    under: true,
                    ..look
                };
                out.push_str(&inline(text, inner, doc, dialogue, look));
            }
            Inline::Wrap { inner, .. } => out.push_str(&inline(text, inner, doc, dialogue, look)),
            Inline::Special => out.push_str(&run(&special(&text[p.span]), look)),
            Inline::Raw => out.push_str(&raw(text, p.span, doc, dialogue, look)),
        }
    }
    out
}

/// The contents: the acts and scenes by name. Word can't know the pages
/// before it lays them out, so none are given.
fn contents(text: &str, doc: &Doc) -> String {
    let title = lang::l("İÇİNDEKİLER", "CONTENTS");
    let mut out = para("Heading1", "<w:pageBreakBefore/>", &run(title, PLAIN));
    for u in shelf::units(text, doc) {
        if u.hidden {
            continue;
        }
        let name = run(&tex::unescape_text(&u.name()), PLAIN);
        out.push_str(&match u.kind {
            shelf::UnitKind::Act => para("Normal", "", &name),
            shelf::UnitKind::Scene => para("Normal", "<w:ind w:left=\"360\"/>", &name),
        });
    }
    out
}

fn cast_rows(text: &str, cast: &Cast) -> String {
    let full = |c: &cast::Character| c.full.clone().unwrap_or_else(|| c.script.clone());
    let title = if info::latin_cast(text) {
        "DRAMATIS PERSONAE"
    } else {
        lang::l("KİŞİLER", "CHARACTERS")
    };
    let mut out = line("Heading1", title);
    let person = |c: &cast::Character| {
        let mut r = run(&full(c), if c.star { BOLD } else { PLAIN });
        if let Some(d) = c.desc.as_deref().filter(|d| !d.trim().is_empty()) {
            r.push_str(&run(&format!(" — {}", tex::unescape_text(d)), PLAIN));
        }
        r
    };
    for row in cast.rows() {
        match row {
            Row::Character(i) if cast.characters[i].kind == cast::Kind::Cast => {
                out.push_str(&para("Normal", "", &person(&cast.characters[i])));
            }
            Row::Character(_) => {}
            Row::Group(g) => {
                let g = &cast.groups[g];
                let names: Vec<String> = g
                    .members
                    .iter()
                    .map(|&m| full(&cast.characters[m]))
                    .collect();
                let line = format!("{} ({})", tex::unescape_text(&g.desc), names.join(", "));
                out.push_str(&self::line("Normal", &line));
            }
        }
    }
    let others = cast::others(text);
    if !others.trim().is_empty() {
        out.push_str(&line("Normal", others.trim()));
    }
    out
}

/// The paragraphs from `at` on as the range of these comments, the marks
/// after the first one's properties and at the end of the last.
fn comment_on(out: &mut String, at: usize, ids: &[usize]) {
    if ids.is_empty() {
        return;
    }
    let first = out[at..]
        .find("</w:pPr>")
        .map(|k| at + k + "</w:pPr>".len());
    let last = out.rfind("</w:p>");
    let (Some(first), Some(last)) = (first, last) else {
        return;
    };
    if first > last {
        return;
    }
    let starts: String = ids
        .iter()
        .map(|i| format!(r#"<w:commentRangeStart w:id="{i}"/>"#))
        .collect();
    let ends: String = ids
        .iter()
        .map(|i| {
            format!(r#"<w:commentRangeEnd w:id="{i}"/><w:r><w:commentReference w:id="{i}"/></w:r>"#)
        })
        .collect();
    out.insert_str(last, &ends);
    out.insert_str(first, &starts);
}

/// `on`: the comments on each block, by its index.
fn body(text: &str, doc: &Doc, on: &BTreeMap<usize, Vec<usize>>) -> String {
    let side = align(doc.scene_align);
    let mut out = String::new();
    // Comments on blocks that print nothing of their own yet (prose waits
    // for its blank line): they go on the next paragraph that comes.
    let mut waiting: Vec<usize> = Vec::new();
    let mut acts = doc.outline.acts.iter();
    // Prose: its source lines are one paragraph, up to a blank line.
    let mut prose = String::new();
    for b in doc.body.clone() {
        if !prose.is_empty() && !matches!(doc.blocks[b].kind, Kind::Other | Kind::Unknown { .. }) {
            // The prose ends before this block, with its own comments.
            let at = out.len();
            out.push_str(&para("Normal", "", &prose));
            prose.clear();
            comment_on(&mut out, at, &waiting);
            waiting.clear();
        }
        waiting.extend(on.get(&b).into_iter().flatten());
        let at = out.len();
        let block = &doc.blocks[b];
        let dialogue = doc.is_dialogue(b);
        if !matches!(block.kind, Kind::Other | Kind::Unknown { .. }) && !prose.is_empty() {
            out.push_str(&para("Normal", "", &prose));
            prose.clear();
        }
        match &block.kind {
            Kind::Line { alias, text: t } => {
                let who = doc
                    .cast
                    .get(alias)
                    .map_or(alias.as_str(), |c| c.script.as_str());
                let mut runs = run(who, BOLD);
                runs.push_str(&run("\t", PLAIN));
                runs.push_str(&inline(text, t.clone(), doc, dialogue, PLAIN));
                out.push_str(&para("Speech", "", &runs));
            }
            Kind::Do { aliases, text: t } => {
                let who: Vec<String> = aliases
                    .iter()
                    .map(|a| doc.cast.inline_name(a).unwrap_or_else(|| a.clone()))
                    .collect();
                let mut runs = run(&format!("({} ", who.join(", ")), ITALIC);
                runs.push_str(&inline(text, t.clone(), doc, dialogue, ITALIC));
                runs.push_str(&run(")", ITALIC));
                out.push_str(&para("Direction", "", &runs));
            }
            Kind::Action { text: t } => {
                out.push_str(&para(
                    "Direction",
                    "",
                    &inline(text, t.clone(), doc, false, ITALIC),
                ));
            }
            Kind::Stage { text: t, right } => {
                let at = align(if *right { Side::Right } else { Side::Center });
                let look = if *right { ITALIC } else { PLAIN };
                out.push_str(&para(
                    "Normal",
                    at,
                    &inline(text, t.clone(), doc, false, look),
                ));
            }
            Kind::Move { enter, aliases } => {
                let said = move_line(&doc.cast, *enter, aliases);
                out.push_str(&para("Direction", "", &run(&said, ITALIC)));
            }
            Kind::Beat => {
                out.push_str(&para(
                    "Direction",
                    "",
                    &run(lang::l("(Sus.)", "(Pause.)"), ITALIC),
                ));
            }
            Kind::Act { title, .. } => {
                let number = acts.next().map_or(0, |a| a.number);
                let mut s = format!("{}. {}", roman(number), lang::l("Perde", "Act"));
                if !title.trim().is_empty() {
                    s.push_str(&format!(": {}", title.trim()));
                }
                out.push_str(&para("Heading1", "<w:pageBreakBefore/>", &run(&s, PLAIN)));
            }
            Kind::Scene { place, .. } => {
                let scene = doc.scene_of(b).map(|s| &doc.outline.scenes[s]);
                let mut s = scene.map_or_else(String::new, |s| s.title());
                if !place.trim().is_empty() {
                    s.push_str(&format!(" — {}", place.trim()));
                }
                out.push_str(&para("Heading2", side, &run(&s, PLAIN)));
                if let Some(scene) = scene.filter(|s| !s.cast.is_empty()) {
                    let who = doc.cast.script_names(&scene.cast).join(", ");
                    out.push_str(&para("SceneCast", side, &run(&who, PLAIN)));
                }
            }
            Kind::Section { which, title, .. } => {
                let name = if title.trim().is_empty() {
                    which.default_title().to_string()
                } else {
                    title.trim().to_string()
                };
                out.push_str(&para(
                    "Heading1",
                    "<w:pageBreakBefore/>",
                    &run(&name, PLAIN),
                ));
            }
            Kind::Other | Kind::Unknown { .. } => {
                let t = text[block.span.clone()].trim();
                // `\maketitle`, `\kisiler` and the like print nothing here.
                let only_a_command = t.starts_with('\\')
                    && !t.contains(char::is_whitespace)
                    && !t[1..].contains('\\');
                if !t.is_empty() && !only_a_command {
                    if !prose.is_empty() {
                        prose.push_str(&run(" ", PLAIN));
                    }
                    prose.push_str(&inline(text, block.span.clone(), doc, false, PLAIN));
                }
            }
            Kind::Comment | Kind::Blank => {}
        }
        if out.len() > at && !waiting.is_empty() {
            comment_on(&mut out, at, &waiting);
            waiting.clear();
        }
    }
    if !prose.is_empty() {
        let at = out.len();
        out.push_str(&para("Normal", "", &prose));
        comment_on(&mut out, at, &waiting);
        waiting.clear();
    }
    // Past the last paragraph: on the last one.
    if !waiting.is_empty()
        && let Some(at) = out.rfind("<w:p>")
    {
        comment_on(&mut out, at, &waiting);
    }
    out
}

/// The open notes as comments: which block each is on (none: the cover),
/// and the comments' own part.
fn comments(
    text: &str,
    doc: &Doc,
    notes: &Notes,
) -> (BTreeMap<usize, Vec<usize>>, Vec<usize>, String) {
    let ix = notes::Index::new(text);
    let mut on: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    let mut loose = Vec::new();
    let mut xml = String::new();
    let open = notes
        .list
        .iter()
        .filter(|n| !n.archived && n.state == State::Open);
    for (id, n) in open.enumerate() {
        let block = ix
            .anchor(n)
            .and_then(|a| doc.block_at(a.range.start))
            .filter(|b| doc.body.contains(b));
        match block {
            Some(b) => on.entry(b).or_default().push(id),
            None => loose.push(id),
        }
        let who = |a: &str| {
            if a.trim().is_empty() {
                "?".to_string()
            } else {
                a.trim().to_string()
            }
        };
        let author = who(&n.author);
        let initials: String = author
            .split_whitespace()
            .filter_map(|w| w.chars().next())
            .collect();
        let mut paras = String::new();
        let mut said = n.text.trim().to_string();
        if !n.group.trim().is_empty() {
            said = format!("[{}] {said}", n.group.trim());
        }
        paras.push_str(&format!("<w:p>{}</w:p>", run(&said, PLAIN)));
        for r in &n.replies {
            let mut runs = run(&format!("{}: ", who(&r.author)), BOLD);
            runs.push_str(&run(r.text.trim(), PLAIN));
            paras.push_str(&format!("<w:p>{runs}</w:p>"));
        }
        xml.push_str(&format!(
            r#"<w:comment w:id="{id}" w:author="{}" w:initials="{}">{paras}</w:comment>"#,
            escape(&author),
            escape(&initials)
        ));
    }
    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:comments xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">{xml}</w:comments>"#
    );
    (on, loose, xml)
}

fn document_xml(
    text: &str,
    doc: &Doc,
    title: &str,
    author: &str,
    on: &BTreeMap<usize, Vec<usize>>,
    loose: &[usize],
) -> String {
    let mut out = String::new();
    if !title.is_empty() {
        out.push_str(&line("Title", title));
    }
    if !author.is_empty() {
        out.push_str(&line("Author", author));
    }
    let editor = tex::unescape_text(&info::get(text, "editedby").unwrap_or_default());
    if !editor.trim().is_empty() {
        let said = format!("{}: {}", lang::l("Düzenleyen", "Edited by"), editor.trim());
        out.push_str(&line("Author", &said));
    }
    // Notes on the cover, or that no longer find their line, go on the title.
    if !loose.is_empty() {
        if out.is_empty() {
            out.push_str(&para("Normal", "", ""));
        }
        let first = out.find("</w:p>").map_or(out.len(), |k| k + "</w:p>".len());
        let mut head = out[..first].to_string();
        comment_on(&mut head, 0, loose);
        out.replace_range(..first, &head);
    }
    if info::contents_page(text) {
        out.push_str(&contents(text, doc));
    }
    if !doc.cast.characters.is_empty() {
        out.push_str(&cast_rows(text, &doc.cast));
    }
    out.push_str(&body(text, doc, on));
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body>{out}<w:sectPr><w:pgSz w:w="11906" w:h="16838"/><w:pgMar w:top="1418" w:right="1418" w:bottom="1418" w:left="1418" w:header="709" w:footer="709" w:gutter="0"/></w:sectPr></w:body></w:document>"#
    )
}

/// The play as the bytes of a `.docx`.
/// The styles, in the play's own font and size when it has them.
fn styles_for(text: &str) -> String {
    let font = crate::info::play_font(text);
    let mut styles = STYLES.to_string();
    if !font.is_empty() {
        styles = styles.replace("Liberation Serif", &escape(&font).replace('"', "&quot;"));
    }
    // Word counts in half points.
    if let Ok(size) = crate::info::play_font_size(text).parse::<f32>() {
        let half = (size * 2.0).round() as u32;
        styles = styles.replacen(
            "<w:sz w:val=\"24\"/>",
            &format!("<w:sz w:val=\"{half}\"/>"),
            1,
        );
    }
    styles
}

pub fn document(text: &str) -> Result<Vec<u8>> {
    package(text, None)
}

/// The play with its open notes as Word comments, as the bytes of a `.docx`.
pub fn with_notes(text: &str, notes: &Notes) -> Result<Vec<u8>> {
    package(text, Some(notes))
}

fn package(text: &str, notes: Option<&Notes>) -> Result<Vec<u8>> {
    let doc = Doc::new(text);
    let title = tex::unescape_text(&info::get(text, "title").unwrap_or_default());
    let author = tex::unescape_text(&info::get(text, "author").unwrap_or_default());
    let core = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:title>{}</dc:title><dc:creator>{}</dc:creator></cp:coreProperties>"#,
        escape(&title),
        escape(&author)
    );
    let app = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties"><Application>{MADE_BY}</Application></Properties>"#
    );
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let opts = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let (on, loose, notes_xml) = match notes {
        Some(n) => comments(text, &doc, n),
        None => Default::default(),
    };
    let commented = !on.is_empty() || !loose.is_empty();
    let body = document_xml(text, &doc, &title, &author, &on, &loose);
    let styles = styles_for(text);
    let (types, rels) = if commented {
        (
            CONTENT_TYPES.replace("</Types>", &format!("{COMMENTS_TYPE}</Types>")),
            DOC_RELS.replace(
                "</Relationships>",
                &format!("{COMMENTS_REL}</Relationships>"),
            ),
        )
    } else {
        (CONTENT_TYPES.to_string(), DOC_RELS.to_string())
    };
    let mut parts = vec![
        ("[Content_Types].xml", types.as_str()),
        ("_rels/.rels", RELS),
        ("word/_rels/document.xml.rels", rels.as_str()),
        ("word/styles.xml", styles.as_str()),
        ("word/document.xml", body.as_str()),
        ("docProps/core.xml", core.as_str()),
        ("docProps/app.xml", app.as_str()),
    ];
    if commented {
        parts.push(("word/comments.xml", notes_xml.as_str()));
    }
    for (name, data) in parts {
        zip.start_file(name, opts)?;
        zip.write_all(data.as_bytes())?;
    }
    Ok(zip.finish()?.into_inner())
}

/// Whether Dactile wrote this file: someone's own document is never replaced.
fn ours(path: &Path) -> bool {
    let read = || -> Option<String> {
        let f = std::fs::File::open(path).ok()?;
        let mut z = zip::ZipArchive::new(f).ok()?;
        let mut app = z.by_name("docProps/app.xml").ok()?;
        let mut s = String::new();
        std::io::Read::read_to_string(&mut app, &mut s).ok()?;
        Some(s)
    };
    read().is_some_and(|s| s.contains(&format!("<Application>{MADE_BY}</Application>")))
}

/// Where a play's document goes: beside it, over an earlier export, never
/// over a document that came from elsewhere.
pub fn target(play: &Path) -> PathBuf {
    let stem = play.file_stem().unwrap_or_default().to_string_lossy();
    free(play, &stem)
}

/// `name.docx` beside the play, or `name-dactile-2.docx`… when someone
/// else's document has that name.
fn free(play: &Path, name: &str) -> PathBuf {
    let plain = play.with_file_name(format!("{name}.docx"));
    if !plain.exists() || ours(&plain) {
        return plain;
    }
    (1..)
        .map(|n| play.with_file_name(format!("{name}-dactile-{n}.docx")))
        .find(|p| !p.exists() || ours(p))
        .unwrap_or(plain)
}

/// Writes the play with its open notes as comments beside itself as
/// `oyun - notlu.docx`, and says where.
pub fn export_with_notes(play: &Path, text: &str, notes: &Notes) -> Result<PathBuf> {
    let pdf = notes::copy_path(play);
    let name = pdf.file_stem().unwrap_or_default().to_string_lossy();
    let out = free(play, &name);
    crate::file::write_atomic_bytes(&out, &with_notes(text, notes)?)?;
    Ok(out)
}

/// Writes the play beside itself as a `.docx`, and says where.
pub fn export(play: &Path, text: &str) -> Result<PathBuf> {
    let out = target(play);
    crate::file::write_atomic_bytes(&out, &document(text)?)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_contents_list_the_acts_and_scenes() {
        let play = "\\documentclass{tiyatro}\n\\icindekiler\n\\begin{document}\n#perde Kar\n\n#sahne Yol\n\n\\end{document}\n";
        let doc = Doc::new(play);
        let out = contents(play, &doc);
        assert!(out.contains("İÇİNDEKİLER") || out.contains("CONTENTS"));
        assert!(out.contains("Kar") && out.contains("Yol"));
    }

    #[test]
    fn the_play_font_reaches_the_styles() {
        let play = "\\documentclass{tiyatro}\n\\playfont{Noto Serif}\n\\begin{document}\n\\end{document}\n";
        let styles = styles_for(play);
        assert!(styles.contains("w:ascii=\"Noto Serif\""));
        assert!(!styles.contains("Liberation"));
    }
    use crate::import;

    const PLAY: &str = include_str!("../../examples/ornek-oyun.tex");

    fn dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("dactile-word-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn texts(path: &Path) -> Vec<String> {
        let s = import::read(path).unwrap();
        s.paras.iter().map(import::Para::text).collect()
    }

    #[test]
    fn the_play_reads_back_as_written() {
        let d = dir("back");
        let play = d.join("oyun.tex");
        let out = export(&play, PLAY).unwrap();
        assert_eq!(out, d.join("oyun.docx"));
        let t = texts(&out);
        assert_eq!(t[0], "Kar Masalı");
        assert!(
            t.iter()
                .any(|p| p.starts_with("SELİN") && p.contains("Amca, yine mi taş durdu?"))
        );
        assert!(t.iter().any(|p| p.contains("(elini taşa koyar)")));
        assert!(
            t.iter()
                .any(|p| p.starts_with("Sahne 1 — İç, Değirmen — Akşam"))
        );
        let all = t.join("\n");
        assert!(!all.contains('\\'), "{all}");
        assert!(!all.contains("#sahne"));
        assert!(t.iter().any(|p| p.contains("#3 numaralı kostümle girer.")));
        // As in the PDF: HERKES speaks, but has no row in the cast.
        assert!(!t.iter().any(|p| p == "HERKES"));
    }

    #[test]
    fn a_document_from_elsewhere_stays() {
        let d = dir("keep");
        let play = d.join("oyun.tex");
        let theirs = d.join("oyun.docx");
        std::fs::write(&theirs, b"someone's own").unwrap();
        let out = export(&play, PLAY).unwrap();
        assert_eq!(out, d.join("oyun-dactile-1.docx"));
        assert_eq!(std::fs::read(&theirs).unwrap(), b"someone's own");
        // The export itself is replaced, not stacked up.
        assert_eq!(export(&play, PLAY).unwrap(), out);
    }

    fn note(line: usize, author: &str, text: &str) -> crate::notes::Note {
        crate::notes::Note {
            state: State::Open,
            archived: false,
            line: Some(line),
            author: author.into(),
            text: text.into(),
            replies: vec![crate::notes::Reply {
                author: "Ece".into(),
                text: "Olur <bakarım> & düzeltirim.".into(),
            }],
            group: String::new(),
        }
    }

    fn part(path: &Path, name: &str) -> String {
        let mut z = zip::ZipArchive::new(std::fs::File::open(path).unwrap()).unwrap();
        let mut s = String::new();
        std::io::Read::read_to_string(&mut z.by_name(name).unwrap(), &mut s).unwrap();
        s
    }

    #[test]
    fn the_notes_become_comments_beside_their_lines() {
        let d = dir("notes");
        let play = d.join("oyun.tex");
        let at = PLAY
            .lines()
            .position(|l| l.contains("Amca, yine mi taş durdu?"))
            .unwrap()
            + 1;
        let mut done = note(at, "Deniz Ak", "Bu satır kısalsın.");
        done.state = State::Done;
        let notes = Notes {
            unread: Vec::new(),
            list: vec![
                note(at, "Deniz Ak", "Daha sert söylesin."),
                note(1, "", "Kapak sade."),
                done,
            ],
        };
        let out = export_with_notes(&play, PLAY, &notes).unwrap();
        assert_eq!(
            out.file_name().unwrap().to_string_lossy(),
            if crate::lang::is_tr() {
                "oyun - notlu.docx"
            } else {
                "oyun - notes.docx"
            }
        );
        let c = part(&out, "word/comments.xml");
        assert!(c.contains("Daha sert söylesin."));
        assert!(c.contains(r#"w:author="Deniz Ak" w:initials="DA""#));
        assert!(c.contains("Olur &lt;bakarım&gt; &amp; düzeltirim."));
        // Only the open notes.
        assert!(!c.contains("Bu satır kısalsın."));
        let doc = part(&out, "word/document.xml");
        // The first comment marks SELİN's line, the second the title.
        let line = doc.find("Amca, yine mi taş durdu?").unwrap();
        let start = doc.find(r#"<w:commentRangeStart w:id="0"/>"#).unwrap();
        let end = doc.find(r#"<w:commentRangeEnd w:id="0"/>"#).unwrap();
        assert!(start < line && line < end);
        assert!(doc[start..line].matches("<w:p>").count() == 0);
        let title = doc.find("Kar Masalı").unwrap();
        assert!(doc.find(r#"<w:commentRangeStart w:id="1"/>"#).unwrap() < title);
        assert!(part(&out, "word/_rels/document.xml.rels").contains("comments.xml"));
        assert!(part(&out, "[Content_Types].xml").contains("/word/comments.xml"));
        // The text reads as the plain export does.
        assert!(
            texts(&out)
                .iter()
                .any(|p| p.contains("Amca, yine mi taş durdu?"))
        );
        // Without notes, no comments part.
        let plain = export(&play, PLAY).unwrap();
        assert!(!part(&plain, "[Content_Types].xml").contains("comments"));
    }

    #[test]
    fn what_xml_can_trip_on_is_escaped() {
        let t = "\\documentclass{tiyatro}\n\\title{A \\& B}\n\\karakter{SELİN}{se}\n\\begin{document}\n\\se a < b & c\n\\end{document}\n";
        let d = dir("xml");
        let out = export(&d.join("oyun.tex"), t).unwrap();
        assert!(texts(&out).iter().any(|p| p.contains("a < b & c")));
    }
}
