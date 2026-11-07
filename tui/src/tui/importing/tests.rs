//! The preview driven by its keys, on made-up documents.

use super::*;
use dactile::import::pages::{Page, Volume};
use dactile::import::{Format, Para, Source};
use dactile::lang::Lang;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::KeyModifiers;

fn press(p: &mut Preview, code: KeyCode) {
    let cfg = Config::default();
    p.key(KeyEvent::new(code, KeyModifiers::NONE), &cfg);
}

fn source(paras: &[&str]) -> Source {
    Source {
        format: Format::Text,
        paras: paras
            .iter()
            .enumerate()
            .map(|(i, t)| Para {
                page: Some(i / 4),
                ..Para::plain(t)
            })
            .collect(),
        title: None,
        author: None,
        pages: (1..=paras.len()).map(|n| n.to_string()).collect(),
        body_page: None,
        plays: Vec::new(),
    }
}

/// A made-up play with a few pages of a book before it.
fn book() -> Source {
    source(&[
        "Kuğu Yayınları",
        "Birinci basım, Kuşadası",
        "Oyunun yazarı üzerine birkaç söz",
        "I. PERDE",
        "SELİN: Kapıyı kim açtı?",
        "MERT: Rüzgâr açtı.",
        "SELİN: Rüzgâr kapı çalmaz.",
        "MERT: Bu rüzgâr çalar.",
        "NUR: Ben de buradayım.",
        "NUR: Sesimi duyan var mı?",
        "SELİN: Duyuyoruz.",
        "Dizin ve kaynaklar",
    ])
}

fn preview(src: &Source) -> Preview {
    Preview::new(
        PathBuf::from("/yok/oyun.txt"),
        Read {
            draft: shape(src, Lang::Tr),
            ocr: false,
        },
    )
}

fn screen(p: &Preview, w: u16, h: u16) -> String {
    let mut t = Terminal::new(TestBackend::new(w, h)).unwrap();
    t.draw(|f| p.draw(f, f.area())).unwrap();
    let b = t.backend().buffer();
    (0..h)
        .map(|y| {
            (0..w)
                .map(|x| b[(x, y)].symbol().to_string())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn tab_goes_round_the_views() {
    let mut p = preview(&book());
    assert!(screen(&p, 100, 30).contains("SELİN"));
    press(&mut p, KeyCode::Tab);
    assert!(p.view == View::Document);
    let s = screen(&p, 100, 30);
    assert!(s.contains("Rüzgâr açtı"), "{s}");
    assert!(s.contains("BÖLÜMLER") || s.contains("PARTS"), "{s}");
    press(&mut p, KeyCode::Tab);
    assert!(p.view == View::Play);
    press(&mut p, KeyCode::Tab);
    assert!(p.view == View::Cast);
    press(&mut p, KeyCode::Tab);
    assert!(p.view == View::People);
    press(&mut p, KeyCode::BackTab);
    assert!(p.view == View::Cast);
}

#[test]
fn a_range_is_marked_and_the_play_is_read_again() {
    let mut p = preview(&book());
    press(&mut p, KeyCode::Tab);
    press(&mut p, KeyCode::Home);
    press(&mut p, KeyCode::Char(' '));
    press(&mut p, KeyCode::Down);
    press(&mut p, KeyCode::Char('m'));
    for _ in 0..12 {
        press(&mut p, KeyCode::Up);
    }
    let cover = Page::ALL.iter().position(|&x| x == Page::Cover).unwrap();
    for _ in 0..cover {
        press(&mut p, KeyCode::Down);
    }
    press(&mut p, KeyCode::Enter);
    assert_eq!(&p.draft.labels[..2], &[Page::Cover, Page::Cover]);
    assert!(!p.draft.tex().contains("Kuğu Yayınları"));
}

#[test]
fn the_play_starts_and_ends_where_it_is_told() {
    let mut p = preview(&book());
    press(&mut p, KeyCode::Tab);
    press(&mut p, KeyCode::Home);
    for _ in 0..3 {
        press(&mut p, KeyCode::Down);
    }
    press(&mut p, KeyCode::Char('s'));
    for _ in 0..7 {
        press(&mut p, KeyCode::Down);
    }
    press(&mut p, KeyCode::Char('e'));
    let plays: Vec<usize> = (0..p.draft.labels.len())
        .filter(|&i| p.draft.labels[i] == Page::Play)
        .collect();
    assert_eq!(plays.first(), Some(&3));
    assert_eq!(plays.last(), Some(&10));
    let tex = p.draft.tex();
    assert!(tex.contains("Duyuyoruz"), "{tex}");
    assert!(!tex.contains("Dizin ve kaynaklar"), "{tex}");
}

/// A mark that drops a person leaves the list shorter than the row chosen
/// on it.
#[test]
fn a_mark_that_drops_a_person_does_not_panic() {
    let mut p = preview(&book());
    let before = p.draft.people.len();
    assert!(before >= 3, "{:?}", p.draft.people);
    p.sel = before - 1;
    press(&mut p, KeyCode::Tab);
    press(&mut p, KeyCode::Home);
    for _ in 0..4 {
        press(&mut p, KeyCode::Down);
    }
    press(&mut p, KeyCode::Char('e'));
    assert!(p.draft.people.len() < before);
    assert!(p.sel < p.draft.people.len());
    press(&mut p, KeyCode::BackTab);
    assert!(p.view == View::People);
    screen(&p, 100, 30);
    press(&mut p, KeyCode::Char('x'));
    press(&mut p, KeyCode::Char('m'));
}

#[test]
fn parts_are_stepped_through_and_going_on_can_be_turned_off() {
    let mut p = preview(&book());
    press(&mut p, KeyCode::Tab);
    press(&mut p, KeyCode::Home);
    press(&mut p, KeyCode::Char('n'));
    press(&mut p, KeyCode::Char('n'));
    press(&mut p, KeyCode::Char('p'));
    press(&mut p, KeyCode::Char('g'));
    let s = screen(&p, 120, 30);
    assert!(s.contains("kapalı") || s.contains(": off"), "{s}");
}

#[test]
fn small_screens_do_not_panic() {
    let mut p = preview(&book());
    for _ in 0..4 {
        for (w, h) in [(120, 40), (80, 24), (41, 13), (30, 8), (10, 4)] {
            screen(&p, w, h);
        }
        press(&mut p, KeyCode::Tab);
    }
    press(&mut p, KeyCode::Tab);
    press(&mut p, KeyCode::Char('m'));
    assert!(p.doc.picking());
    for (w, h) in [(80, 24), (30, 8), (10, 4)] {
        screen(&p, w, h);
    }
}

#[test]
fn the_play_view_stops_at_as_many_lines_as_it_shows() {
    let lines: Vec<String> = (0..1000)
        .map(|i| format!("{}: Satır {i}.", ["SELİN", "MERT"][i % 2]))
        .collect();
    let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
    let mut p = preview(&source(&refs));
    press(&mut p, KeyCode::BackTab);
    press(&mut p, KeyCode::BackTab);
    press(&mut p, KeyCode::End);
    let s = screen(&p, 100, 30);
    assert!(s.contains("daha") || s.contains("more"), "{s}");
}

/// A key press reads the play again, on a long book too.
#[test]
fn a_long_book_marks_in_good_time() {
    let lines: Vec<String> = (0..20_000)
        .map(|i| {
            format!(
                "{}: Satır {i}, söz uzadıkça uzar.",
                ["SELİN", "MERT"][i % 2]
            )
        })
        .collect();
    let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
    let src = source(&refs);
    let at = Instant::now();
    let mut p = preview(&src);
    let read = at.elapsed();
    press(&mut p, KeyCode::Tab);
    let at = Instant::now();
    press(&mut p, KeyCode::Char('s'));
    let marked = at.elapsed();
    let at = Instant::now();
    screen(&p, 120, 40);
    eprintln!("read {read:?}, mark {marked:?}, frame {:?}", at.elapsed());
    // A mark reads the play again, no more: against the first reading, as
    // a debug build on a busy machine has no fixed time.
    assert!(marked < read * 3, "read {read:?}, mark {marked:?}");
}

fn volume() -> Source {
    let mut paras = vec!["Birinci Oyun".to_string()];
    paras.extend((0..6).map(|i| format!("SELİN: Söz {i}.")));
    paras.push("İkinci Oyun".to_string());
    paras.extend((0..6).map(|i| format!("MERT: Söz {i}.")));
    let refs: Vec<&str> = paras.iter().map(String::as_str).collect();
    let mut src = source(&refs);
    src.plays = vec![
        Volume {
            title: "Birinci Oyun".into(),
            start: 0,
            from: 1,
            sure: true,
        },
        Volume {
            title: "İkinci Oyun".into(),
            start: 7,
            from: 8,
            sure: true,
        },
    ];
    src
}

#[test]
fn v_goes_through_the_plays_of_a_volume() {
    let mut p = preview(&volume());
    assert_eq!(p.draft.play, Some(0));
    press(&mut p, KeyCode::Char('v'));
    assert_eq!(p.draft.play, Some(1));
    assert_eq!(p.draft.title, "İkinci Oyun");
    assert!(screen(&p, 100, 30).contains("2/2"));
    press(&mut p, KeyCode::Char('v'));
    assert_eq!(p.draft.play, Some(0));
}

/// Reading again by the labels forgets which play of a volume it was.
#[test]
fn marking_keeps_the_play_of_a_volume() {
    let mut p = preview(&volume());
    press(&mut p, KeyCode::Char('v'));
    press(&mut p, KeyCode::Tab);
    press(&mut p, KeyCode::Char('e'));
    assert_eq!(p.draft.play, Some(1));
    assert_eq!(p.draft.title, "İkinci Oyun");
}

#[test]
fn marks_stay_with_their_play_when_v_goes_away_and_back() {
    let mut p = preview(&volume());
    press(&mut p, KeyCode::Tab);
    press(&mut p, KeyCode::Tab);
    press(&mut p, KeyCode::Char('e'));
    let marked = p.draft.labels.clone();
    assert_ne!(
        marked,
        dactile::import::shape::shape_play(&volume(), Lang::Tr, 0).labels
    );
    press(&mut p, KeyCode::Char('v'));
    assert_eq!(p.draft.title, "İkinci Oyun");
    press(&mut p, KeyCode::Char('v'));
    assert_eq!(p.draft.play, Some(0));
    assert_eq!(p.draft.labels, marked);
}

#[test]
fn v_does_nothing_with_one_play() {
    let mut p = preview(&book());
    press(&mut p, KeyCode::Char('v'));
    assert_eq!(p.draft.play, None);
}

#[test]
fn o_is_there_only_for_a_pdf_with_the_tools_and_asks_first() {
    let mut p = preview(&book());
    let cfg = Config::default();
    let o = KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE);
    assert!(matches!(p.key(o, &cfg), Step::Stay));
    assert!(matches!(p.key(o, &cfg), Step::Stay));
    assert!(!screen(&p, 120, 30).contains("OCR"));
    p.ocr = true;
    assert!(screen(&p, 120, 30).contains("OCR"));
    assert!(matches!(p.key(o, &cfg), Step::Stay));
    assert!(p.error.is_some());
    assert!(matches!(p.key(o, &cfg), Step::Again));
    // Another key between the two: it asks again.
    p.key(o, &cfg);
    press(&mut p, KeyCode::Down);
    assert!(matches!(p.key(o, &cfg), Step::Stay));
}

fn waiting(total: usize) -> (Importing, Arc<AtomicBool>) {
    let (_tx, rx) = channel();
    let stop = Arc::new(AtomicBool::new(false));
    let imp = Importing::Reading {
        rx,
        file: PathBuf::from("/yok/kitap.pdf"),
        stop: stop.clone(),
        pages: Arc::new((AtomicUsize::new(1), AtomicUsize::new(total))),
        back: None,
    };
    (imp, stop)
}

#[test]
fn esc_sets_the_stop_flag_and_waits_for_the_pages_under_way() {
    let cfg = Config::default();
    let esc = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
    let (mut imp, stop) = waiting(9);
    assert!(matches!(imp.key(esc, &cfg), Step::Stay));
    assert!(stop.load(Ordering::Relaxed));
    let (mut imp, stop) = waiting(0);
    assert!(matches!(imp.key(esc, &cfg), Step::Done(None)));
    assert!(stop.load(Ordering::Relaxed));
}

#[test]
fn a_reading_again_that_stops_goes_back_to_the_preview() {
    let (tx, rx) = channel();
    let mut imp = Importing::Reading {
        rx,
        file: PathBuf::from("/yok/kitap.pdf"),
        stop: Arc::new(AtomicBool::new(true)),
        pages: Arc::new((AtomicUsize::new(0), AtomicUsize::new(3))),
        back: Some(Box::new(preview(&book()))),
    };
    tx.send(Err("durdu".to_string())).unwrap();
    imp.tick(Lang::Tr);
    let Importing::Preview(p) = &imp else {
        panic!("no preview");
    };
    assert_eq!(p.error.as_deref(), Some("durdu"));
}

/// A job told to stop: the reading says so, not that the file failed.
#[test]
fn a_stopped_reading_is_not_an_error_of_the_file() {
    let stop = AtomicBool::new(true);
    let job = Job {
        progress: &|_, _| {},
        stop: &stop,
    };
    let e = shaped(Path::new("/yok/kitap.txt"), Lang::Tr, false, &job).err();
    assert_eq!(
        e.as_deref(),
        Some(lang::l("Okuma durduruldu.", "The reading was stopped."))
    );
}

#[test]
fn the_progress_of_ocr_is_drawn() {
    let (imp, _stop) = waiting(9);
    let mut t = Terminal::new(TestBackend::new(60, 12)).unwrap();
    t.draw(|f| imp.draw(f, f.area())).unwrap();
    let s: String = t
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|c| c.symbol())
        .collect();
    assert!(s.contains("OCR 1/9"), "{s}");
}

/// Two people, then two others, each after a page of a title.
fn found_volume() -> Source {
    let mut paras = Vec::new();
    for (page, title, who) in [(0, "Kar Masalı", "SELİN"), (5, "Yol", "OZAN")] {
        paras.push(dactile::import::Para {
            page: Some(page),
            ..dactile::import::Para::plain(title)
        });
        for i in 0..30 {
            paras.push(dactile::import::Para {
                page: Some(page + 1 + i / 10),
                ..dactile::import::Para::plain(&format!("{who}. Bir söz {i}."))
            });
        }
    }
    let mut src = source(&[]);
    src.format = dactile::import::Format::Pdf;
    src.plays = dactile::import::pages::volume(&paras);
    src.paras = paras;
    src
}

#[test]
fn a_volume_opens_on_its_plays() {
    let mut p = preview(&found_volume());
    assert!(p.view == View::Volume);
    let s = screen(&p, 100, 30);
    assert!(
        s.contains("Kar Masalı")
            && s.contains("Yol")
            && (s.contains("tahmin") || s.contains("guess")),
        "{s}"
    );
    press(&mut p, KeyCode::Down);
    press(&mut p, KeyCode::Char(' '));
    assert_eq!(p.chosen, [true, false]);
    press(&mut p, KeyCode::Enter);
    assert!(p.view == View::People);
    assert_eq!(p.draft.title, "Yol");
}

#[test]
fn what_is_no_play_goes_from_the_list() {
    let mut p = preview(&found_volume());
    press(&mut p, KeyCode::Down);
    press(&mut p, KeyCode::Char('x'));
    assert!(p.draft.source.plays.is_empty());
    assert!(p.view == View::People);
}

#[test]
fn small_screens_show_a_volume() {
    let p = preview(&found_volume());
    for (w, h) in [(20, 5), (40, 8), (80, 12)] {
        screen(&p, w, h);
    }
}

#[test]
fn the_cast_list_is_put_right_a_line_at_a_time() {
    let mut p = preview(&book());
    for _ in 0..3 {
        press(&mut p, KeyCode::Tab);
    }
    assert!(p.view == View::Cast);
    let before = p.draft.cast_text().lines().count();
    press(&mut p, KeyCode::Char('x'));
    assert_eq!(p.draft.cast_text().lines().count(), before - 1);
    assert!(screen(&p, 100, 30).contains("x"));
    screen(&p, 30, 8);
}
