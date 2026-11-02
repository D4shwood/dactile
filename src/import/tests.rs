//! Made-up documents, built here, read and shaped end to end.

use super::shape::{Item, shape};
use super::*;
use crate::lang::Lang;
use crate::render::Doc;
use std::io::Write;

fn zip_of(files: &[(&str, &str)]) -> Vec<u8> {
    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let mut z = zip::ZipWriter::new(&mut buf);
        let opts = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        for (name, body) in files {
            z.start_file(*name, opts).unwrap();
            z.write_all(body.as_bytes()).unwrap();
        }
        z.finish().unwrap();
    }
    buf.into_inner()
}

fn w_p(inner: &str) -> String {
    format!("<w:p>{inner}</w:p>")
}
fn w_r(text: &str) -> String {
    format!("<w:r><w:t xml:space=\"preserve\">{text}</w:t></w:r>")
}

const STYLES: &str = r#"<?xml version="1.0"?><w:styles xmlns:w="w">
<w:style w:type="paragraph" w:styleId="Normal"><w:name w:val="Normal"/></w:style>
<w:style w:type="paragraph" w:styleId="Title"><w:name w:val="Title"/><w:basedOn w:val="Normal"/></w:style>
<w:style w:type="paragraph" w:styleId="Yonerge"><w:name w:val="Yönerge"/><w:basedOn w:val="Normal"/><w:rPr><w:i/></w:rPr></w:style>
</w:styles>"#;

fn docx() -> Vec<u8> {
    let body = [
        "<w:p><w:pPr><w:pStyle w:val=\"Title\"/></w:pPr><w:r><w:t>Kar Masalı</w:t></w:r></w:p>".into(),
        w_p(&w_r("Yazan: Örnek Yazar")),
        w_p(&w_r("KİŞİLER")),
        w_p(&w_r("SELİN — köy öğretmeni")),
        w_p(&w_r("Ozan Usta, değirmenci")),
        w_p(&w_r("BİRİNCİ PERDE")),
        w_p(&w_r("1. Sahne: Ahır — Gece")),
        "<w:p><w:pPr><w:pStyle w:val=\"Yonerge\"/></w:pPr><w:r><w:t>Işıklar yavaşça açılır.</w:t></w:r></w:p>".into(),
        w_p(&format!(
            "{}<w:r><w:rPr><w:i/></w:rPr><w:t>(kapıya bakar)</w:t></w:r>{}<w:r><w:br/><w:t>Ses yok.</w:t></w:r>",
            w_r("SELİN: Kim var "),
            w_r(" orada?")
        )),
        w_p(&format!(
            "<w:r><w:rPr><w:b/></w:rPr><w:t>OZAN</w:t></w:r>{}",
            w_r(": Benim. 100% eminim &amp; geldim_")
        )),
        w_p(&w_r("(Selin güler.)")),
        format!(
            "<w:tbl><w:tr><w:tc>{}</w:tc><w:tc>{}</w:tc></w:tr></w:tbl>",
            w_p(&w_r("SELİN")),
            w_p(&w_r("Gel içeri."))
        ),
        w_p(&format!(
            "{}<w:del><w:r><w:delText> silinen</w:delText></w:r></w:del>",
            w_r("OZAN: Geliyorum")
        )),
        w_p(&w_r("NOT: bu sahne kısa.")),
        w_p(&w_r("SON")),
    ]
    .concat();
    zip_of(&[
        (
            "word/document.xml",
            &format!(
                "<?xml version=\"1.0\"?><w:document xmlns:w=\"w\"><w:body>{body}</w:body></w:document>"
            ),
        ),
        ("word/styles.xml", STYLES),
        (
            "docProps/core.xml",
            "<?xml version=\"1.0\"?><cp:coreProperties xmlns:cp=\"c\" xmlns:dc=\"d\"><dc:title>Microsoft Word - eski.docx</dc:title><dc:creator>kullanici</dc:creator></cp:coreProperties>",
        ),
    ])
}

fn read_bytes(name: &str, bytes: &[u8]) -> Source {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("dactile-import-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    std::fs::write(&path, bytes).unwrap();
    let src = read(&path).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    src
}

#[test]
fn a_word_document() {
    let src = read_bytes("oyun.docx", &docx());
    let draft = shape(&src, Lang::Tr);
    assert_eq!(draft.title, "Kar Masalı");
    assert_eq!(draft.author, "Örnek Yazar");
    let names: Vec<&str> = draft.people.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["SELİN", "OZAN", "NOT"]);
    assert_eq!(draft.people[1].full.as_deref(), Some("Ozan Usta"));
    assert_eq!(draft.people[0].desc.as_deref(), Some("köy öğretmeni"));
    assert_eq!(draft.people[0].lines, 2);
    assert_eq!(draft.people[1].lines, 2);
    let tex = draft.tex();
    for want in [
        "\\title{Kar Masalı}",
        "\\author{Örnek Yazar}",
        "\\karakter[köy öğretmeni]{SELİN}{se}",
        "\\karakter[Ozan Usta][değirmenci]{OZAN}{oz}",
        "#perde\n",
        "#sahne Ahır — Gece\n",
        "\\action{Işıklar yavaşça açılır.}",
        "\\se Kim var \\ic{kapıya bakar} orada?\nSes yok.\n",
        "\\oz Benim. 100\\% eminim \\& geldim\\_\n",
        "\\action{Selin güler.}",
        "\\se Gel içeri.\n",
        "\\oz Geliyorum\n",
        "\nSON\n",
    ] {
        assert!(tex.contains(want), "{want:?} not in:\n{tex}");
    }
    assert!(!tex.contains("silinen"));
    assert!(!tex.contains("kullanici"));
    // What Dactile reads back.
    let doc = Doc::new(&tex);
    assert_eq!(doc.outline.acts.len(), 1);
    assert_eq!(doc.outline.scenes.len(), 1);
    assert_eq!(doc.cast.characters.len(), 3);
    let lines = doc
        .blocks
        .iter()
        .filter(|b| matches!(b.kind, crate::lex::Kind::Line { .. }))
        .count();
    assert_eq!(lines, 5);
}

#[test]
fn checking_the_names() {
    let src = read_bytes("oyun.docx", &docx());
    let mut draft = shape(&src, Lang::Tr);
    // NOT is not a character: its line stays as it was written.
    let not = draft.people.iter().position(|p| p.name == "NOT").unwrap();
    draft.people[not].not_character = true;
    // OZAN's lines go to SELİN, and SELİN gets another alias.
    draft.merge(1, 0);
    assert_eq!(
        draft.alias_problem(0, "it"),
        Some(crate::lang::l(
            "bu kısaltma alınmış ya da bir LaTeX komutu",
            "this alias is taken or a LaTeX command",
        ))
    );
    assert!(draft.alias_problem(0, "Se").is_some());
    assert_eq!(draft.alias_problem(0, "sel"), None);
    draft.people[0].alias = "sel".into();
    let tex = draft.tex();
    assert!(tex.contains("NOT: bu sahne kısa."));
    assert!(tex.contains("\\karakter[Ozan Usta][köy öğretmeni]{SELİN}{sel}"));
    assert!(!tex.contains("{OZAN}"));
    assert!(tex.contains("\\sel Benim."));
    assert_eq!(Doc::new(&tex).cast.characters.len(), 1);
    let c = draft.counts();
    assert_eq!((c.lines, c.directions, c.acts, c.scenes), (4, 2, 1, 1));
}

#[test]
fn an_opendocument() {
    let content = r#"<?xml version="1.0"?>
<office:document-content xmlns:office="o" xmlns:text="t" xmlns:style="s" xmlns:fo="f">
<office:automatic-styles>
<style:style style:name="P1" style:family="paragraph"><style:text-properties fo:font-style="italic"/></style:style>
<style:style style:name="T1" style:family="text"><style:text-properties fo:font-style="italic"/></style:style>
</office:automatic-styles>
<office:body><office:text>
<text:h text:outline-level="1">İkinci Perde: Kar</text:h>
<text:p text:style-name="P1">Kar yağar.</text:p>
<text:p>SELİN:<text:s/>Üşüdüm <text:span text:style-name="T1">(titrer)</text:span>,<text:line-break/>çok.</text:p>
<text:p>İHTİYAR. Ateş yak.</text:p>
</office:text></office:body></office:document-content>"#;
    let meta = r#"<?xml version="1.0"?><office:document-meta xmlns:office="o" xmlns:dc="d" xmlns:meta="m"><office:meta><dc:title>Kar</dc:title><meta:initial-creator>Örnek Yazar</meta:initial-creator></office:meta></office:document-meta>"#;
    let src = read_bytes(
        "oyun.odt",
        &zip_of(&[("content.xml", content), ("meta.xml", meta)]),
    );
    let draft = shape(&src, Lang::Tr);
    assert_eq!(
        (draft.title.as_str(), draft.author.as_str()),
        ("Kar", "Örnek Yazar")
    );
    let tex = draft.tex();
    for want in [
        "#perde Kar\n",
        "\\action{Kar yağar.}",
        "\\se Üşüdüm \\ic{titrer},\nçok.\n",
        "\\ih Ateş yak.\n",
        // No scene heading in the document: one, before the first line.
        "#sahne\n",
    ] {
        assert!(tex.contains(want), "{want:?} not in:\n{tex}");
    }
}

#[test]
fn names_over_their_lines() {
    let text = "Kar Masalı\nÖrnek Yazar\n\nSELİN\n(gülerek)\nGel buraya.\n\nOZAN\nGeliyorum,\nkoşarak.\n\nSELİN\nKar yağıyor.\n\nSON\n";
    let src = read_bytes("oyun.txt", text.as_bytes());
    let draft = shape(&src, Lang::Tr);
    assert_eq!(
        (draft.title.as_str(), draft.author.as_str()),
        ("Kar Masalı", "Örnek Yazar")
    );
    let speeches: Vec<String> = draft
        .items
        .iter()
        .filter_map(|i| match i {
            Item::Speech { said, .. } => Some(said.iter().map(|r| r.text.as_str()).collect()),
            _ => None,
        })
        .collect();
    assert_eq!(
        speeches,
        [
            "(gülerek) Gel buraya.",
            "Geliyorum,\nkoşarak.",
            "Kar yağıyor."
        ]
    );
    let tex = draft.tex();
    assert!(tex.contains("\\se \\ic{gülerek} Gel buraya.\n"), "{tex}");
    assert!(tex.contains("\nSON\n"));
}

#[test]
fn not_a_document() {
    let dir = std::env::temp_dir().join(format!("dactile-import-bad-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join("bozuk.docx");
    std::fs::write(&p, b"not a zip").unwrap();
    assert!(read(&p).is_err());
    assert!(read(&dir.join("oyun.rtf")).is_err());
    let _ = std::fs::remove_dir_all(&dir);
}
