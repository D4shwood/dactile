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
    let said = format!("{:#}", read(&p).unwrap_err());
    assert!(!said.starts_with("reading"), "{said}");
    assert!(read(&dir.join("oyun.rtf")).is_err());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn people_get_aliases_of_their_own() {
    let src = plain::read(
        "SELİN: Bir.\n\nSEDA: İki.\n\nSELİN: Üç.\n\nSEDA: Dört.\n",
        false,
    );
    let d = shape::shape(&src, Lang::Tr);
    let aliases: Vec<&str> = d.people.iter().map(|p| p.alias.as_str()).collect();
    assert_eq!(aliases.len(), 2, "{aliases:?}");
    assert_ne!(aliases[0], aliases[1]);
    assert!(d.alias_problem(0, aliases[1]).is_some());
}

/// A made-up book: everything a publisher puts around a play.
const BOOK: &str = "\
ÖRNEK YAYINLARI

Genel Yayın: 2408

Tiyatro Dizisi: 17

KAR MASALI

Deniz Örnek

Çeviren: Mert Kaya

EDİTÖR: Lale Tan

KAPAK TASARIMI: Ece Su

ISBN: 978-605-000-00-0

BASKI: Yıldız Matbaası

Sertifika No: 12345

Tel: 0212 000 00 00

Tel: 0212 000 00 01

Tel: 0212 000 00 02

www.ornekyayinlari.example

İÇİNDEKİLER

Yazar Hakkında ........ 5

Önsöz ........ 7

Birinci Perde ........ 11

İkinci Perde ........ 30

YAZAR HAKKINDA

Deniz Örnek bir kış günü küçük bir kasabada doğdu ve ilk oyununu okul yıllarında yazdı, sonra uzun yıllar gezici topluluklarla çalıştı.

ÖDÜLLER: Üç kez ödül aldı.

ÖNSÖZ

Bu oyun bir değirmenin hikâyesidir ve seyirciden sabır ister; kar yağdıkça sahne daralır, kişiler birbirine yaklaşır, söz azalır.

SELİN: bu oyunda bir öğretmendir, ama önce bir yeğendir.

OYUNUN KİŞİLERİ

SELİN — köy öğretmeni

OZAN — değirmenci

HABERCİ — yolcu

Köylüler, çobanlar, konuklar.

Yöneten: Can Er

Dekor: Ada Gül

Oyun bir köyde geçer.

BİRİNCİ PERDE

Birinci Sahne: Değirmen — Akşam

(Selin içeri girer.)

SELİN: Amca, taş yine mi durdu?

OZAN: Durdu kızım.

SELİN: Kar mı?

OZAN: Kar.

HABERCİ: Yol kapandı.

SELİN: Biz de kaldık.

OZAN: Kalırız.

İKİNCİ PERDE

SELİN: Sabah oldu.

OZAN: Oldu.

PERDE KAPANIR

AYNI YAZARIN DİĞER KİTAPLARI

Kış Oyunları: Deniz Örnek

Yol Hikâyeleri: Deniz Örnek
";

#[test]
fn a_book_around_a_play() {
    let d = shape::shape(&plain::read(BOOK, false), Lang::Tr);
    assert_eq!(
        (d.title.as_str(), d.author.as_str()),
        ("KAR MASALI", "Deniz Örnek")
    );
    let names: Vec<&str> = d.people.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["SELİN", "OZAN", "HABERCİ"]);
    assert_eq!(d.people[0].desc.as_deref(), Some("köy öğretmeni"));
    let c = d.counts();
    assert_eq!((c.acts, c.scenes), (2, 1), "{:#?}", d.items);
    assert_eq!(c.lines, 9);
    let pages: Vec<pages::Page> = d.asides.iter().map(|a| a.page).collect();
    use pages::Page::*;
    for want in [Imprint, Contents, Author, Preface, Back] {
        assert!(pages.contains(&want), "{want:?} not in {pages:?}");
    }
    assert_eq!(d.others, "Köylüler, çobanlar, konuklar.");
    let tex = d.tex();
    assert!(
        tex.contains("\\digerkisiler{Köylüler, çobanlar, konuklar.}"),
        "{tex}"
    );
    for gone in [
        "2408",
        "ISBN",
        "Lale",
        "Yıldız",
        "ödül",
        "Kış Oyunları",
        "Can Er",
    ] {
        assert!(!tex.contains(gone), "{gone:?} in:\n{tex}");
    }
    assert!(tex.contains("#önsöz\n\nBu oyun bir değirmenin"), "{tex}");
    assert!(tex.contains("Oyun bir köyde geçer."), "{tex}");
    assert!(tex.contains("PERDE KAPANIR"), "{tex}");
}

#[test]
fn labels_changed_by_hand() {
    let mut d = shape::shape(&plain::read(BOOK, false), Lang::Tr);
    let at = |d: &shape::Draft, t: &str| {
        d.source
            .paras
            .iter()
            .position(|p| p.text().starts_with(t))
            .unwrap()
    };
    // A page of the book into the play.
    let i = at(&d, "AYNI YAZARIN");
    d.mark(i, i + 2, pages::Page::Play);
    assert!(d.tex().contains("Deniz Örnek"));
    d.mark(i, i + 2, pages::Page::Back);
    assert!(!d.tex().contains("Kış Oyunları"));
    // Corrections last through it.
    d.people[1].name = "OZAN USTA".into();
    d.people[1].alias = "ou".into();
    d.title = "Kar".into();
    d.play_from(at(&d, "İKİNCİ PERDE"));
    assert_eq!(d.title, "Kar");
    assert_eq!(d.counts().acts, 1);
    assert!(d.tex().contains("\\ou Oldu."), "{}", d.tex());
    assert!(!d.tex().contains("Amca, taş"));
    d.play_to(at(&d, "SELİN: Sabah"));
    assert_eq!(d.counts().lines, 1);
}

#[test]
fn a_cast_page_with_no_title() {
    let text = "Kar\n\nÖrnek Yazar\n\nSELİN — öğretmen\n\nOZAN — değirmenci\n\nSELİN: Bir.\n\nOZAN: İki.\n\nSELİN: Üç.\n\nOZAN: Dört.\n";
    let d = shape::shape(&plain::read(text, false), Lang::Tr);
    assert_eq!(d.people.len(), 2);
    assert_eq!(d.people[1].desc.as_deref(), Some("değirmenci"));
    assert_eq!(d.counts().lines, 4);
}

#[test]
fn a_part_named_for_a_job_still_speaks() {
    let text = "YAZAR: Bir.\n\nEDİTÖR: İki.\n\nYAZAR: Üç.\n\nEDİTÖR: Dört.\n\nYAZAR: Beş.\n\nEDİTÖR: Altı.\n";
    let d = shape::shape(&plain::read(text, false), Lang::Tr);
    assert_eq!(d.counts().lines, 6, "{:#?}", d.items);
}

#[test]
fn spellings_of_the_cast_page() {
    let text = "KİŞİLER\n\nKÖYÜN AĞASI\n\nKORAYNO — bir dost\n\nYAŞLI TEMEL — baba\n\nI. PERDE\n\nAĞA: Bir.\n\nKORAYNG: İki.\n\nTEMEL: Üç.\n\nAĞA: Dört.\n\nKORAYNG: Beş.\n\nTEMEL: Altı.\n\nAĞA: Yedi.\n\nKORAYNG: Sekiz.\n";
    let d = shape::shape(&plain::read(text, false), Lang::Tr);
    let cast: Vec<&str> = d
        .cast()
        .into_iter()
        .map(|i| d.people[i].name.as_str())
        .collect();
    assert_eq!(cast, ["AĞA", "KORAYNG", "TEMEL"], "{:#?}", d.people);
    assert_eq!(d.lines_of(d.cast()[0]), 3);
}

#[test]
fn notes_go_where_they_are_marked() {
    let on = |t: &str, foot: bool| Para {
        page: Some(3),
        foot,
        ..Para::plain(t)
    };
    let mut paras = vec![on("Soytarı* gibi", false), on("Ve kalanı.", false)];
    let (a, b) = (on("Biri.", true), on("Öbürü.", true));
    let placed = pages::attach_notes(&mut paras, &[&a, &b]);
    assert_eq!(placed.len(), 2);
    assert_eq!(paras[0].text(), "Soytarı gibi");
    assert!(paras[0].runs[1].note && paras[0].runs[1].text == "Biri.");
    assert!(paras[1].runs.last().unwrap().note);
}

/// Two plays in a book, each with a page of its title; a book's PDF comes
/// so, with pages.
fn volume() -> Source {
    let mut paras = Vec::new();
    let mut add = |page: usize, t: String| {
        paras.push(Para {
            page: Some(page),
            ..Para::plain(&t)
        })
    };
    add(0, "Contents".into());
    add(0, "Chronology\nKAR MASALI\nYOL".into());
    add(1, "Chronology".into());
    add(2, "1990 Born.".into());
    add(3, "Kar Masalı".into());
    for i in 0..30 {
        add(4 + i / 10, format!("SELİN. Bir söz {i}."));
    }
    add(8, "Yol".into());
    for i in 0..30 {
        add(9 + i / 10, format!("OZAN. Bir söz {i}."));
    }
    let mut src = plain::read("", false);
    src.format = Format::Pdf;
    src.paras = paras;
    src.plays = pages::volume(&src.paras);
    src
}

#[test]
fn a_volume_of_plays() {
    let src = volume();
    let titles: Vec<&str> = src.plays.iter().map(|p| p.title.as_str()).collect();
    assert_eq!(titles, ["Kar Masalı", "Yol"]);
    let first = shape::shape(&src, Lang::Tr);
    assert_eq!(first.title, "Kar Masalı");
    assert_eq!(first.counts().lines, 30);
    let second = shape::shape_play(&src, Lang::Tr, 1);
    let cast: Vec<&str> = second
        .cast()
        .into_iter()
        .map(|i| second.people[i].name.as_str())
        .collect();
    assert_eq!(cast, ["OZAN"]);
}

#[test]
fn a_speech_goes_on_after_a_direction() {
    let mut src = plain::read(
        "I. PERDE\n\nSELİN: Bir.\n\nOZAN: İki.\n\nOZAN: Üç.\n\nSELİN: Dört.\n\n(Girer.)\n\nVe sürer.\n",
        false,
    );
    src.paras.last_mut().unwrap().resumes = true;
    let d = shape::shape(&src, Lang::Tr);
    assert_eq!(d.counts().lines, 5);
    assert_eq!(d.counts().directions, 1);
}

#[test]
fn a_note_with_a_stop_in_it() {
    let text = "I. PERDE\n\nSELİN (bakar. Bekler). Bir.\n\nSELİN. İki.\n\nSELİN. Üç.\n";
    let d = shape::shape(&plain::read(text, false), Lang::Tr);
    assert_eq!(d.counts().lines, 3, "{:#?}", d.people);
}

/// A PDF of one empty page: nothing in it to read as text.
const BLANK_PDF: &str = "%PDF-1.4
1 0 obj <</Type/Catalog/Pages 2 0 R>> endobj
2 0 obj <</Type/Pages/Kids[3 0 R]/Count 1>> endobj
3 0 obj <</Type/Page/Parent 2 0 R/MediaBox[0 0 200 200]>> endobj
trailer <</Root 1 0 R>>
%%EOF
";

#[test]
fn read_as_leaves_a_document_with_text_alone() {
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    let dir = std::env::temp_dir().join(format!("dactile-read-as-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("oyun.txt");
    std::fs::write(&file, "SELİN: Merhaba.\n").unwrap();
    let asked = AtomicUsize::new(0);
    let progress = |_: usize, _: usize| {
        asked.fetch_add(1, Ordering::Relaxed);
    };
    let stop = AtomicBool::new(false);
    let job = ocr::Job {
        progress: &progress,
        stop: &stop,
    };
    let src = read_as(&file, false, &job).unwrap();
    assert_eq!(src.paras.len(), 1);
    assert_eq!(asked.load(Ordering::Relaxed), 0);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Stopped before it starts, so that no Tesseract runs: the error tells
/// which way the reading went.
#[test]
fn read_as_goes_to_the_images_where_a_pdf_has_no_text() {
    let dir = std::env::temp_dir().join(format!("dactile-read-as-pdf-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("taranmis.pdf");
    std::fs::write(&file, BLANK_PDF).unwrap();
    let stop = std::sync::atomic::AtomicBool::new(true);
    let job = ocr::Job {
        progress: &|_, _| {},
        stop: &stop,
    };
    let e = read_as(&file, false, &job).unwrap_err();
    assert_eq!(
        e.downcast_ref::<pdf::NoText>().is_some(),
        !ocr::ready(),
        "{e:#}"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// The same book with no contents to go by.
fn volume_unlisted() -> Source {
    let mut src = volume();
    src.paras.drain(..2);
    src.plays = pages::volume(&src.paras);
    src
}

#[test]
fn a_volume_without_contents_is_found_by_who_speaks() {
    let src = volume_unlisted();
    let titles: Vec<&str> = src.plays.iter().map(|p| p.title.as_str()).collect();
    assert_eq!(titles, ["Kar Masalı", "Yol"]);
    assert!(src.plays.iter().all(|p| !p.sure));
}

#[test]
fn one_play_with_a_quiet_page_in_it_stays_one() {
    let mut paras = Vec::new();
    let mut add = |page: usize, t: String| {
        paras.push(Para {
            page: Some(page),
            ..Para::plain(&t)
        })
    };
    add(0, "Kar Masalı".into());
    for i in 0..30 {
        add(1 + i / 10, format!("SELİN. Bir söz {i}."));
    }
    add(4, "Bir dizeden sonra".into());
    for i in 0..30 {
        add(5 + i / 10, format!("SELİN. Yine söz {i}."));
    }
    add(8, "İKİNCİ PERDE".into());
    for i in 0..30 {
        add(9 + i / 10, format!("SELİN. Son söz {i}."));
    }
    assert!(pages::volume(&paras).is_empty());
}

#[test]
fn a_play_marked_by_hand() {
    let src = volume();
    let mut d = shape::shape(&src, Lang::Tr);
    let at = 20;
    assert_eq!(d.add_play(at, "Ara Oyun"), Ok(1));
    let titles: Vec<&str> = d.source.plays.iter().map(|p| p.title.as_str()).collect();
    assert_eq!(titles, ["Kar Masalı", "Ara Oyun", "Yol"]);
    assert_eq!(d.title, "Ara Oyun");
    assert_eq!(d.play, Some(1));
    let all = d.summaries();
    assert_eq!(all[0].lines + all[1].lines, 30);
    assert!(d.add_play(at, "Yine").is_err());
    d.drop_play(1);
    assert_eq!(d.source.plays.len(), 2);
    assert_eq!(d.summaries()[0].lines, 30);
    d.drop_play(1);
    assert!(d.source.plays.is_empty());
    assert_eq!(d.play, None);
}

#[test]
fn a_book_taken_for_one_play_is_split_by_hand() {
    let mut src = plain::read("", false);
    src.paras = (0..40)
        .map(|i| Para::plain(&format!("SELİN. Bir söz {i}.")))
        .collect();
    let mut d = shape::shape(&src, Lang::Tr);
    assert_eq!(d.add_play(20, "İkinci"), Ok(1));
    let all = d.summaries();
    assert_eq!((all[0].lines, all[1].lines), (20, 20));
}

#[test]
fn what_was_put_right_stays_with_its_play() {
    let src = volume();
    let mut d = shape::shape(&src, Lang::Tr);
    d.title = "Düzeltilmiş".into();
    d.go_to_play(1);
    assert_eq!(d.title, "Yol");
    d.go_to_play(0);
    assert_eq!(d.title, "Düzeltilmiş");
    let both = d.plays(&[0, 1]);
    assert_eq!(both[0].title, "Düzeltilmiş");
    assert_eq!(both[1].title, "Yol");
}

/// A cast page of a first production: names, who played them, notes.
fn premiere_page() -> Vec<Para> {
    [
        "KAR MASALI was first seen at a small stage with the following cast:",
        "SELİN                                   Deniz Kaya",
        "OZAN, later a doctor                    Ali Veli",
        "ASKER 1 at the door                     Can Er",
        "HAMAL                                   Ece Su",
        "SOLDIERS, WORKERS, GUARDS: Can Er, Ece Su",
    ]
    .iter()
    .map(|t| Para::plain(t))
    .collect()
}

#[test]
fn a_first_reading_of_a_cast_page_leaves_the_actors_out() {
    assert_eq!(
        pages::cast_guess(&premiere_page()),
        "SELİN\nOZAN | later a doctor\nASKER 1 | at the door\nHAMAL\n+ SOLDIERS, WORKERS, GUARDS"
    );
}

#[test]
fn the_cast_by_hand_decides_order_notes_and_the_silent() {
    let text = "I. PERDE\n\nSELİN: Bir.\n\nOZAN: İki.\n\nSELİN: Üç.\n\nOZAN: Dört.\n";
    let mut d = shape::shape(&plain::read(text, false), Lang::Tr);
    d.set_cast("OZAN | köylü\nHAMAL | hiç konuşmaz\nSELİN\n+ Askerler, köylüler");
    let names: Vec<&str> = d
        .cast()
        .into_iter()
        .map(|i| d.people[i].name.as_str())
        .collect();
    assert_eq!(names, ["OZAN", "HAMAL", "SELİN"]);
    assert_eq!(d.others, "Askerler, köylüler");
    let tex = d.tex();
    assert!(tex.contains("\\karakter[köylü]{OZAN}"), "{tex}");
    assert!(tex.contains("\\karakter[hiç konuşmaz]{HAMAL}"), "{tex}");
    assert_eq!(d.cast_check(), (vec!["HAMAL".to_string()], Vec::new()));
    d.set_cast("OZAN");
    assert_eq!(d.cast_check().1, ["SELİN"]);
    assert_eq!(d.counts().lines, 4);
}

#[test]
fn a_cast_by_hand_stays_when_the_play_is_read_again() {
    let text = "Bir not\n\nI. PERDE\n\nSELİN: Bir.\n\nOZAN: İki.\n\nSELİN: Üç.\n\nOZAN: Dört.\n";
    let mut d = shape::shape(&plain::read(text, false), Lang::Tr);
    d.set_cast("OZAN | köylü\nHAMAL");
    d.mark(0, 0, pages::Page::Other);
    assert_eq!(d.cast_text(), "OZAN | köylü\nHAMAL");
    assert!(d.tex().contains("\\karakter[köylü]{OZAN}"));
    assert!(d.tex().contains("{HAMAL}"));
}

#[test]
fn a_name_on_the_list_goes_with_the_shorter_one_that_speaks() {
    let text = "I. PERDE\n\nDÜK. Bir.\n\nOZAN. İki.\n\nDÜK. Üç.\n\nOZAN. Dört.\n";
    let mut d = shape::shape(&plain::read(text, false), Lang::Tr);
    let page = [Para::plain("KUZEY DÜK"), Para::plain("OZAN")];
    d.source.paras.extend(page);
    d.labels.extend([pages::Page::Other, pages::Page::Other]);
    let n = d.source.paras.len();
    d.mark_cast(n - 2, n - 1);
    assert!(
        d.cast_text().contains("KUZEY DÜK = DÜK"),
        "{}",
        d.cast_text()
    );
    assert!(
        d.tex().contains("\\karakter[KUZEY DÜK][]{DÜK}"),
        "{}",
        d.tex()
    );
}

#[test]
fn a_group_on_the_list_goes_out_as_a_group() {
    let text = "I. PERDE\n\nSELİN: Bir.\n\nASKER: İki.\n\nSELİN: Üç.\n\nASKER: Dört.\n";
    let mut d = shape::shape(&plain::read(text, false), Lang::Tr);
    d.set_cast("SELİN\n\\chargroup Kapıdakiler\n\\gkarakter ASKER | ilk nöbet\n\\gkarakter HAMAL");
    let tex = d.tex();
    assert!(
        tex.contains("\\karaktergrubu[Kapıdakiler]{\n\t\\gkarakter[ilk nöbet]{ASKER}"),
        "{tex}"
    );
    assert!(tex.contains("\\gkarakter{HAMAL}"), "{tex}");
    assert_eq!(tex.matches("\\karaktergrubu").count(), 1);
    let cast = crate::cast::Cast::of_play(&tex);
    assert_eq!(cast.characters.len(), 3);
}

#[test]
fn a_volumes_front_matter_starts_after_its_contents() {
    let src = volume();
    assert_eq!(src.front_matter(), "Chronology\n\n1990 Born.");
}
