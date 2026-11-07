//! The writer driven by its keys, on a copy of the example play.

use super::*;
use dactile::config::Config;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

const PLAY: &str = include_str!("../../../../examples/ornek-oyun.tex");

struct Open {
    w: Writer,
    dir: PathBuf,
}

impl Drop for Open {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn open(tag: &str) -> Open {
    let dir = std::env::temp_dir().join(format!("dactile-writer-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("oyun.tex");
    std::fs::write(&path, PLAY).unwrap();
    let w = Writer::open(&path, &Config::default()).unwrap();
    Open { w, dir }
}

fn press(w: &mut Writer, code: KeyCode) {
    w.key(KeyEvent::new(code, KeyModifiers::NONE));
}

fn with(w: &mut Writer, code: KeyCode, mods: KeyModifiers) {
    w.key(KeyEvent::new(code, mods));
}

fn type_in(w: &mut Writer, s: &str) {
    for c in s.chars() {
        press(w, KeyCode::Char(c));
    }
}

fn screen(w: &mut Writer) -> String {
    let mut t = Terminal::new(TestBackend::new(110, 40)).unwrap();
    t.draw(|f| w.draw(f)).unwrap();
    t.backend()
        .buffer()
        .content()
        .iter()
        .map(|c| c.symbol())
        .collect()
}

/// The cursor at the end of the block holding `words`.
fn at_end_of(w: &mut Writer, words: &str) {
    let at = w.buf.text.find(words).unwrap();
    let b = w.doc.block_at(at).unwrap();
    w.buf.cursor = w.doc.blocks[b].span.end;
    w.buf.anchor = None;
    w.relayout();
}

#[test]
fn others_are_set_shown_and_cleared() {
    let mut o = open("others");
    let w = &mut o.w;
    with(w, KeyCode::Char('o'), KeyModifiers::CONTROL);
    assert!(matches!(w.popup, Popup::Cast(_)));
    press(w, KeyCode::Char('o'));
    type_in(w, "Koylu & coban");
    press(w, KeyCode::Enter);
    assert!(matches!(w.popup, Popup::Cast(_)));
    assert_eq!(dcast::others(&w.buf.text), "Koylu & coban");
    assert!(screen(w).contains("Koylu & coban"));
    press(w, KeyCode::Char('o'));
    for _ in 0.."Koylu & coban".len() {
        press(w, KeyCode::Backspace);
    }
    press(w, KeyCode::Enter);
    assert_eq!(dcast::others(&w.buf.text), "");
    assert_eq!(w.buf.text, PLAY);
}

#[test]
fn f_goes_to_where_a_character_first_speaks() {
    let mut o = open("first");
    let w = &mut o.w;
    let first = w.buf.text.find("\\se Amca").unwrap();
    with(w, KeyCode::Char('o'), KeyModifiers::CONTROL);
    let i = w.cast_index("se");
    if let Popup::Cast(p) = &mut w.popup {
        p.sel = i;
    }
    press(w, KeyCode::Char('f'));
    assert!(matches!(w.popup, Popup::None));
    let b = w.block().unwrap();
    assert_eq!(w.doc.blocks[b].span.start, first);
}

#[test]
fn a_read_only_play_keeps_its_castings() {
    let mut o = open("casting");
    let w = &mut o.w;
    w.sidecar.castings.push(Casting {
        name: "Birinci".into(),
        actors: Default::default(),
    });
    w.read_only = true;
    w.roles.tab = roles::Tab::Casting;
    w.popup = Popup::Roles;
    press(w, KeyCode::Char('x'));
    press(w, KeyCode::Char('x'));
    assert_eq!(w.sidecar.castings.len(), 1);
    w.read_only = false;
    press(w, KeyCode::Char('x'));
    press(w, KeyCode::Char('x'));
    assert!(w.sidecar.castings.is_empty());
}

#[test]
fn the_pace_goes_back_to_the_default() {
    let mut o = open("pace");
    let w = &mut o.w;
    w.sidecar.wpm = Some(120);
    w.press(PAct::PlaySettings);
    assert!(screen(w).contains("120"));
    w.read_only = true;
    press(w, KeyCode::Char('d'));
    assert_eq!(w.sidecar.wpm, Some(120));
    w.read_only = false;
    press(w, KeyCode::Char('d'));
    assert_eq!(w.sidecar.wpm, None);
    assert_eq!(w.wpm(), w.default_wpm);
    // The form comes back to the settings, not to the numbers.
    press(w, KeyCode::Enter);
    assert!(matches!(
        &w.popup,
        Popup::Form(f) if f.kind == FormKind::Wpm(true)
    ));
    press(w, KeyCode::Esc);
    assert!(matches!(w.popup, Popup::PlaySettings(0)));
}

#[test]
fn a_word_comes_out_of_the_play() {
    let mut o = open("words");
    let w = &mut o.w;
    w.sidecar.words = vec!["zıpzıp".into(), "hopla".into()];
    w.refresh_known();
    assert!(w.known.contains("zıpzıp"));
    w.popup = Popup::PlaySettings(0);
    press(w, KeyCode::Down);
    assert!(screen(w).contains("zıpzıp"));
    press(w, KeyCode::Char('x'));
    assert_eq!(w.sidecar.words, ["hopla"]);
    assert!(!w.known.contains("zıpzıp"));
    press(w, KeyCode::Char('x'));
    assert!(w.sidecar.words.is_empty());
    assert!(matches!(w.popup, Popup::PlaySettings(0)));
}

#[test]
fn the_draft_of_is_shown() {
    let mut o = open("parent");
    let w = &mut o.w;
    w.sidecar.parent = Some("kar-masali.tex".into());
    w.popup = Popup::PlaySettings(0);
    assert!(screen(w).contains("kar-masali.tex"));
}

#[test]
fn enter_at_a_lines_end_answers_and_on_an_empty_one_ends_it() {
    let mut o = open("enter");
    let w = &mut o.w;
    at_end_of(w, "\\oz Durdu kızım");
    press(w, KeyCode::Enter);
    assert!(
        w.buf.text.contains("durdurur.\n\\se \n"),
        "the one who spoke before answers"
    );
    press(w, KeyCode::Enter);
    assert!(w.buf.text.contains("durdurur.\n\n"));
    assert!(!w.buf.text.contains("durdurur.\n\\se"));
}

#[test]
fn enter_in_the_middle_of_a_line_breaks_it_as_before() {
    let mut o = open("middle");
    let w = &mut o.w;
    let at = w.buf.text.find("yine mi").unwrap();
    w.buf.cursor = at;
    w.relayout();
    press(w, KeyCode::Enter);
    assert!(w.buf.text.contains("Amca, \nyine mi"));
}

#[test]
fn tab_after_the_speaker_gives_the_line_to_the_next() {
    let mut o = open("tab");
    let w = &mut o.w;
    at_end_of(w, "\\oz Durdu kızım");
    press(w, KeyCode::Enter);
    assert!(w.buf.text.contains("\n\\se \n"));
    press(w, KeyCode::Tab);
    assert!(w.buf.text.contains("\n\\oz \n"), "the next of the cast");
    type_in(w, "Evet");
    assert!(w.buf.text.contains("\n\\oz Evet\n"));
    // With something said it does nothing.
    let before = w.buf.text.clone();
    press(w, KeyCode::Tab);
    assert_eq!(w.buf.text, before);
}

#[test]
fn f12_and_the_view_tab_have_the_pdf() {
    let tabs = ribbon::tabs();
    assert!(
        tabs.iter()
            .flat_map(|t| t.btns())
            .any(|b| b.key == "F12" && b.act == PAct::Key(KeyCode::F(12), KeyModifiers::NONE))
    );
    let mut o = open("f12");
    let w = &mut o.w;
    // No PDF built: it says so, and nothing is launched.
    press(w, KeyCode::F(12));
    assert!(w.message.as_ref().is_some_and(|m| m.1));
}

#[test]
fn the_footnote_is_on_the_ribbon() {
    assert!(
        ribbon::tabs()
            .iter()
            .flat_map(|t| t.btns())
            .any(|b| { b.act == PAct::InsertInline("\\dipnot{‸}") })
    );
}

#[test]
fn alt_g_goes_to_the_line_the_build_stopped_on() {
    let mut o = open("failed");
    let w = &mut o.w;
    let at = w.buf.text.find("\\oz Durdu").unwrap();
    let line = w.buf.text[..at].matches('\n').count() + 1;
    w.built(Err(Failed {
        line: Some(line),
        message: "Undefined control sequence".into(),
    }));
    assert_eq!(w.failed_line, Some(line));
    assert!(w.message.as_ref().is_some_and(|m| m.0.contains("Alt+G")));
    // Not by itself: the cursor stays until it is asked.
    assert_ne!(w.buf.cursor, at);
    with(w, KeyCode::Char('g'), KeyModifiers::ALT);
    assert_eq!(w.buf.cursor, at);
}

#[test]
fn a_build_without_a_line_says_nothing_of_alt_g() {
    let mut o = open("noline");
    let w = &mut o.w;
    w.built(Err(Failed {
        line: None,
        message: "xelatex not found".into(),
    }));
    assert!(w.message.as_ref().is_some_and(|m| !m.0.contains("Alt+G")));
    let at = w.buf.cursor;
    with(w, KeyCode::Char('g'), KeyModifiers::ALT);
    assert_eq!(w.buf.cursor, at);
}
