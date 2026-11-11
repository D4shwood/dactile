//! Made-up notes on a made-up play.

use super::*;

const PLAY: &str = "\\documentclass{tiyatro}
\\begin{document}
#sahne Ahır

\\se Kar yağıyor, Gaztesi geldi mi?
\\oz Gelmedi. Kalbinin tamamen duyduğu
işitilir bu gece.
\\se Terk etmek bende Yefim.
\\oz İlk düşünce de doğru, St. Peterburg'a gideriz.
\\se Kar yağıyor.
\\end{document}
";

const REPLY: &str = "Dosyayı son kaydından aldım. Şunlar var.

Anlamı bozan yerler (karar senin)

- **7:** “Kalbinin tamamen duyduğu işitilir” → “durduğu”.
- 8: “Terk etmek bende Yefim” cümlesinde “yok” eksik: “Terk etmek bende yok Yefim”.
- 9: “İlk düşünce de doğru” → “İlk düşüncen doğru”.
devamı bir alt satırda.

Düz yazım yanlışları

• 5: “Gaztesi” → Gazetesi. Ayrıca bir tekrar var.
• 9 (son sahne): “St. Peterburg'a” → St. Petersburg'a; virgül de eksik.
- 10 “Kar yağıyor” bir tekrar.
- Genel olarak noktalama iyi.

Bunlar dışında temiz.
";

#[test]
fn a_reply_becomes_notes() {
    let notes = from_reply(REPLY, "Claude");
    let got: Vec<(Option<usize>, &str)> =
        notes.iter().map(|n| (n.line, n.group.as_str())).collect();
    let g1 = "Anlamı bozan yerler (karar senin)";
    let g2 = "Düz yazım yanlışları";
    assert_eq!(
        got,
        [
            (Some(7), g1),
            (Some(8), g1),
            (Some(9), g1),
            (Some(5), g2),
            (Some(9), g2),
            (Some(10), g2),
            (None, g2)
        ]
    );
    assert!(notes[2].text.ends_with("devamı bir alt satırda."));
    assert!(notes[4].text.starts_with("(son sahne) “St."));
    assert!(notes.iter().all(|n| n.author == "Claude"));
    assert!(!notes.iter().any(|n| n.text.contains("temiz")));
}

#[test]
fn quotes_and_fixes() {
    let n = |t: &str| Note {
        text: t.into(),
        ..Note::default()
    };
    assert_eq!(
        n("“Gaztesi” → Gazetesi. Ayrıca “Kronika” bir tekrar").fixes(),
        [Fix {
            from: "Gaztesi".into(),
            to: "Gazetesi".into()
        }]
    );
    assert_eq!(
        n("“2 şahıs’ın” → 2 şahsın; “vatan’a” → vatana; “St. Peterb…” → St. Petersburg")
            .fixes()
            .iter()
            .map(|f| (f.from.as_str(), f.to.as_str()))
            .collect::<Vec<_>>(),
        [
            ("2 şahıs’ın", "2 şahsın"),
            ("vatan’a", "vatana"),
            ("St. Peterb…", "St. Petersburg")
        ]
    );
    assert_eq!(n("dikeleniyo → dikleniyo").fixes()[0].from, "dikeleniyo");
    assert_eq!(
        n("“loş bir bir ışığın” (çift “bir”)").quotes(),
        ["loş bir bir ışığın", "bir"]
    );
    assert!(n("“yarım kalmış").quotes().is_empty());
}

#[test]
fn notes_find_their_place_and_fix_it() {
    let notes = from_reply(REPLY, "Claude");
    let ix = Index::new(PLAY);
    let at = |i: usize| {
        let a = ix.anchor(&notes[i]).unwrap();
        (&PLAY[a.range.clone()], a.line, a.quoted)
    };
    // Over a line end: found all the same.
    assert_eq!(at(0), ("Kalbinin tamamen duyduğu\nişitilir", 6, true));
    assert_eq!(at(3), ("Gaztesi", 5, true));
    // No quote found: the line.
    let lone = Note {
        line: Some(10),
        text: "noktalama".into(),
        ..Note::default()
    };
    let a = ix.anchor(&lone).unwrap();
    assert_eq!((&PLAY[a.range], a.quoted), ("\\se Kar yağıyor.", false));
    // Each fix changes only what differs.
    let fixed = |i: usize| crate::tex::apply(PLAY, &ix.fix_edits(&notes[i]));
    assert!(fixed(0).contains("Kalbinin tamamen durduğu\nişitilir"));
    // Said, not drawn with an arrow: not a fix.
    assert!(notes[1].fixes().is_empty());
    let added = Note {
        line: Some(8),
        text: "“Terk etmek bende Yefim” → “Terk etmek bende yok Yefim”".into(),
        ..Note::default()
    };
    let out = crate::tex::apply(PLAY, &ix.fix_edits(&added));
    assert!(out.contains("\\se Terk etmek bende yok Yefim.\n"), "{out}");
    assert!(fixed(2).contains("İlk düşüncen doğru, St."));
    assert!(fixed(3).contains("Kar yağıyor, Gazetesi geldi"));
    assert!(fixed(4).contains("St. Petersburg'a gideriz"));
    // Fixed, it looks so; the others don't.
    let after = Index::new(&fixed(3));
    assert!(after.gone(&notes[3]));
    assert!(!after.gone(&notes[0]));
    assert!(!ix.gone(&notes[3]));
    // Lines added above: still found, by what it quotes.
    let moved = PLAY.replace("#sahne Ahır\n", "#sahne Ahır\n\n\n\n");
    let mix = Index::new(&moved);
    let a = mix.anchor(&notes[3]).unwrap();
    assert_eq!((&moved[a.range], a.line), ("Gaztesi", 8));
}

#[test]
fn the_file_round_trips() {
    let mut notes = Notes {
        unread: Vec::new(),
        list: from_reply(REPLY, "Claude"),
    };
    notes.list[0].state = State::Done;
    notes.list[1].replies.push(Reply {
        author: "Deniz".into(),
        text: "bilerek böyle".into(),
    });
    notes.list[2].state = State::Hidden;
    notes.list[4].archived = true;
    notes.list.push(Note {
        line: Some(3),
        author: "Atölye".into(),
        text: "“Ahır” yerine “Samanlık”?".into(),
        ..Note::default()
    });
    let md = notes.to_markdown("oyun.tex");
    let back = Notes::parse(&md);
    // The ungrouped note goes first, the archived last; each one the same.
    let mut want = notes.list.clone();
    want.sort_by_key(|n| (n.archived, !n.group.is_empty()));
    let key = |n: &Note| (n.group.clone(), n.archived);
    let mut got = back.list.clone();
    got.sort_by_key(key);
    want.sort_by_key(key);
    assert_eq!(got, want, "{md}");
    assert!(md.contains("- [x] 7 · **Claude** · "), "{md}");
    assert!(md.contains("  - **Deniz**: bilerek böyle\n"), "{md}");
    assert!(md.contains("## Arşiv: Düz yazım yanlışları\n"), "{md}");
    assert_eq!(notes.authors(), ["Claude", "Deniz", "Atölye"]);
}

#[test]
fn a_hand_written_file() {
    let md = "# Notlar\n\n- [ ] her şey yolunda mı\n  bir satır daha\n* [X] 12 · **Okur**: “kar” mı “kış” mı?\n  - cevapsız yazar\n    ve devamı\n## Arşiv\n- [-] eski\n";
    let n = Notes::parse(md);
    assert_eq!(n.list.len(), 3);
    assert_eq!(n.list[0].text, "her şey yolunda mı bir satır daha");
    assert_eq!(
        (n.list[1].state, n.list[1].line, n.list[1].author.as_str()),
        (State::Done, Some(12), "Okur")
    );
    assert_eq!(n.list[1].replies[0].text, "cevapsız yazar ve devamı");
    assert!(n.list[2].archived && n.list[2].state == State::Hidden);
}

#[test]
fn a_copy_with_notes() {
    let play = include_str!("../../../examples/ornek-oyun.tex");
    let doc = crate::render::Doc::new(play);
    let mut notes = Notes::parse(
        "## Yazım\n\n- [ ] 47 · **Claude** · “taş durdu” → taşı durdu\n  - **Deniz**: bakarım\n- [x] 55 · **Claude** · “Biraz ısınsam”: çözüldü\n- [ ] 1 · **Atölye** · kapakta bir not\n- [ ] 49 · **Atölye** · 100% sahne_dışı & {belirsiz}\n",
    );
    notes.list.push(Note {
        line: Some(41),
        author: "Okur".into(),
        text: "“İç, Değirmen” başlıkta".into(),
        ..Note::default()
    });
    let tex = copy_tex(play, &doc, &notes, false);
    assert!(tex.contains("\\tiyatronotes\n\\begin{document}"), "{tex}");
    // Beside its line, its words in its colour; numbered by place.
    assert!(tex.contains(
        "\\se Amca, yine mi \\tiyatronote{B0444A}{Claude}{2}{“taş durdu” → taşı durdu}\\tiyatronoted{B0444A}{taş durdu}?"
    ), "{tex}");
    // A line without a quote: at the start of its words; escaped.
    assert!(
        tex.contains(
            "\\se \\tiyatronote{B0703A}{Atölye}{3}{100\\% sahne\\_dışı \\& \\{belirsiz\\}}Ama"
        ),
        "{tex}"
    );
    // The heading's note beside the scene's first line, numbered first.
    assert!(
        tex.contains("\\action{\\tiyatronote{4F6AA8}{Okur}{1}{"),
        "{tex}"
    );
    // The done note left out; the cover's only listed.
    assert!(!tex.contains("çözüldü"));
    assert_eq!(tex.matches("\\tiyatronote{").count(), 3);
    assert_eq!(tex.matches("\\tiyatronoteitem{").count(), 4);
    assert!(tex.contains("\\tiyatronotereply{"));
    // The list says where each is as the PDF has it: the line's number and
    // page from LaTeX, the scene by its place.
    assert!(tex.contains(
        "{1}{Okur}{Sahne başı: İç, Değirmen — Akşam · Replik~\\ref{tiyatronote:1} · s.~\\pageref{tiyatronote:1}}"
    ), "{tex}");
    assert!(tex.contains("{2}{Claude}{Replik~\\ref{tiyatronote:2} · İç, Değirmen — Akşam · s.~\\pageref{tiyatronote:2}}"), "{tex}");
    assert!(tex.contains("{4}{Atölye}{Kapak}{kapakta bir not}"), "{tex}");
    assert!(tex.trim_end().ends_with("\\end{document}"));
}

#[test]
fn a_correction_in_source_stays_source() {
    let play = "\\la Maria! \\ic{kapıyı çalar.}Mustafa gelir.\n\\mus İyiyim \\ic{gülümser.}. Gel.\n\\ma Yüzde 50 kaldı.\n";
    let ix = Index::new(play);
    let note = |line: usize, t: &str| Note {
        line: Some(line),
        text: t.into(),
        ..Note::default()
    };
    let out = crate::tex::apply(
        play,
        &[
            ix.fix_edits(&note(
                1,
                "“çalar.}Mustafa gelir.” → “çalar.} Mustafa gelir.”",
            )),
            ix.fix_edits(&note(2, "“\\ic{gülümser.}. Gel” → “\\ic{gülümser.} Gel”")),
            ix.fix_edits(&note(3, "“Yüzde 50 kaldı” → “yüzde 50% kaldı”")),
        ]
        .concat(),
    );
    assert!(out.contains("\\ic{kapıyı çalar.} Mustafa gelir."), "{out}");
    assert!(out.contains("\\ic{gülümser.} Gel."), "{out}");
    assert!(out.contains("\\ma yüzde 50\\% kaldı."), "{out}");
}

/// What happened on a real play, made up again: an AI quoting the source,
/// braces and all, and corrections that, made blindly, would break it.
#[test]
fn a_correction_never_breaks_the_play() {
    let play = "\\documentclass{tiyatro}\n\\karakter{LAYLA}{la}\n\\karakter{MUSTAFA}{mus}\n\\begin{document}\n#sahne Bir\n\n\\la Gel! \\ic{kapıyı çalar, devam ediyordur.}Mustafa gelir.\n\\mus İyiyim \\ic{öyle gözükmüyordur.}. Üstüme geldiler.\n\\la Yüzde 50 kaldı.\n\n#sahne İki\n\n\\mus Sabah.\n\\end{document}\n";
    let note = |line: usize, t: &str| Note {
        line: Some(line),
        text: t.into(),
        ..Note::default()
    };
    let made = |n: &Note| crate::tex::apply(play, &checked_fixes(play, n).unwrap());
    // Source for source: braces stay braces.
    let out = made(&note(
        7,
        "“devam ediyordur.}Mustafa gelir.” → “devam ediyordur.} Mustafa gelir.”",
    ));
    assert!(
        out.contains("\\ic{kapıyı çalar, devam ediyordur.} Mustafa gelir."),
        "{out}"
    );
    let out = made(&note(
        8,
        "“\\ic{öyle gözükmüyordur.}. Üstüme” → “\\ic{öyle gözükmüyordur.} Üstüme”",
    ));
    assert!(
        out.contains("\\ic{öyle gözükmüyordur.} Üstüme geldiler."),
        "{out}"
    );
    // Words for words: escaped as typing would.
    let out = made(&note(9, "“Yüzde 50 kaldı” → “yüzde 50% kaldı”"));
    assert!(out.contains("\\la yüzde 50\\% kaldı."), "{out}");
    // A brace lost, a brace too many, a command nobody knows, a speech
    // gone: refused, the play untouched.
    for (line, bad) in [
        (
            8,
            "“\\ic{öyle gözükmüyordur.}” → “\\ic{öyle gözükmüyordur.”",
        ),
        (7, "“Gel!” → “Gel! }”"),
        (7, "“Gel!” → “\\gel{} Gel!”"),
        (13, "“\\mus Sabah.” → “Sabah.”"),
    ] {
        match checked_fixes(play, &note(line, bad)) {
            Err(Refused::Breaks { line: at, .. }) => assert!(at > 0),
            other => panic!("{bad}: {other:?}"),
        }
    }
    assert_eq!(
        checked_fixes(play, &note(7, "kim gelir?")),
        Err(Refused::Nothing)
    );
    assert_eq!(
        checked_fixes(play, &note(7, "“Gitti” → Geldi")),
        Err(Refused::NotFound)
    );
}

#[test]
fn corrections_go_in_together() {
    let text =
        "\\documentclass{tiyatro}\n\\begin{document}\nKar yağdı, taş durdu.\n\\end{document}\n";
    let note = |t: &str| Note {
        state: State::Open,
        archived: false,
        line: None,
        author: "Atölye".into(),
        text: t.into(),
        replies: Vec::new(),
        group: String::new(),
    };
    let mut n = Notes {
        unread: Vec::new(),
        list: vec![
            note("“Kar” → Yağmur"),
            note("“taş” → kaya"),
            note("“yok böyle” → var"),
            note("yalnızca bir yorum"),
        ],
    };
    let (out, done, left) = apply_fixes(text, &mut n, &[0, 1, 2, 3]);
    assert!(out.contains("Yağmur yağdı, kaya durdu."));
    assert_eq!((done, left), (2, 1));
    assert_eq!(n.list[0].state, State::Done);
    assert_eq!(n.list[2].state, State::Open);
    assert_eq!(n.list[3].state, State::Open);
}

#[test]
fn a_file_with_lines_that_dont_read_is_left_alone() {
    let d = std::env::temp_dir().join(format!("dactile-notes-keep-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let play = d.join("oyun.tex");
    let theirs =
        "# Notlar: oyun.tex\n\n- [ ] 12 · **Ece** · Daha yavaş.\nBu satır bir not değil.\n";
    std::fs::write(path_for(&play), theirs).unwrap();
    let mut n = Notes::load(&play).unwrap();
    assert_eq!(n.list.len(), 1);
    assert_eq!(n.unread, vec!["Bu satır bir not değil.".to_string()]);
    n.list[0].state = State::Done;
    assert!(n.save(&play).is_err());
    assert_eq!(std::fs::read_to_string(path_for(&play)).unwrap(), theirs);
    // A file Dactile wrote reads back whole.
    let ok = Notes::parse(
        &Notes {
            list: n.list.clone(),
            unread: Vec::new(),
        }
        .to_markdown("oyun.tex"),
    );
    assert!(ok.unread.is_empty());
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_notes_file_of_ours_comes_in_as_itself() {
    let file = "# Notlar: oyun.tex\n\n## Okuma\n\n- [ ] 963 · **Deniz** · “taş” bir bak.\n  - **Ece**: tamam.\n- [x] 970 · **Ece** · Kısalsın.\n";
    let got = from_reply(file, "Notlar");
    assert_eq!(got.len(), 2);
    assert_eq!(got[0].author, "Deniz");
    assert_eq!(got[0].line, Some(963));
    assert_eq!(got[0].text, "“taş” bir bak.");
    assert_eq!(got[0].replies.len(), 1);
    assert_eq!(got[1].state, State::Done);
    // A task list someone wrote by hand: the box is not the note.
    let hand = from_reply("- [ ] 963 · “taş” bir bak", "Okur");
    assert_eq!(hand[0].line, Some(963));
    assert!(!hand[0].text.contains("[ ]"));
}

/// `DACTILE_NOTES_FILE=… cargo test -- --ignored notes_file_reads`: how
/// many notes a file has and how many lines didn't read, nothing of them.
#[test]
#[ignore]
fn notes_file_reads() {
    let path = std::env::var("DACTILE_NOTES_FILE").unwrap();
    let n = Notes::parse(&std::fs::read_to_string(path).unwrap());
    println!("notes {} unread {}", n.list.len(), n.unread.len());
}

/// A made-up play crowded with long notes, as a PDF copy:
/// `DACTILE_NOTES_OUT=dir cargo test -- --ignored crowded_copy_to`.
#[test]
#[ignore]
fn crowded_copy_to() {
    let out = std::path::PathBuf::from(std::env::var("DACTILE_NOTES_OUT").unwrap());
    std::fs::create_dir_all(&out).unwrap();
    let mut play = String::from(
        "\\documentclass{tiyatro}\n\\title{Kalabalık}\n\\author{Örnek Yazar}\n\\karakter{SELİN}{se}\n\\karakter{OZAN}{oz}\n\\begin{document}\n\\maketitle\n#perde\n#sahne İç, Değirmen — Akşam\n\nBir giriş paragrafı, düz metin.\n\n",
    );
    for i in 0..60 {
        let who = if i % 2 == 0 { "\\se" } else { "\\oz" };
        play.push_str(&format!("{who} Bu {i}. replik, biraz uzunca bir söz ki satır dolsun ve kenar not yanında dursun.\n"));
        if i == 10 {
            play.push_str("\\stagedir{Rüzgâr kapıyı çarpar.}\n\\middir{Uzaktan bir çan.}\n#sahne Dış, Meydan — Gece\n");
        }
    }
    play.push_str("\\end{document}\n");
    let long = "Bu not epey uzun: sahnenin ritmi burada düşüyor, söz bir önceki replikle çakışıyor, oyuncu nefes alacak yer bulamıyor; belki iki cümleye bölünür, belki biri hiç söylenmez. Bir de şu var: ";
    let mut md = String::from("# Notlar: kalabalik.tex\n\n");
    let lines: Vec<&str> = play.lines().collect();
    for (k, l) in lines.iter().enumerate() {
        let n = k + 1;
        if l.starts_with("\\se") || l.starts_with("\\oz") {
            if n % 3 != 0 {
                md.push_str(&format!("- [ ] {n} · **Deniz Ak** · {long}{n}.\n"));
            }
        } else if l.starts_with("#sahne")
            || l.starts_with("\\stagedir")
            || l.starts_with("\\middir")
            || l.starts_with("Bir giriş")
        {
            md.push_str(&format!(
                "- [ ] {n} · **Ece** · Başlıkta, yönergede ya da düz metinde bir not ({n}).\n"
            ));
        }
    }
    md.push_str("- [ ] 2 · **Ece** · Kapakta bir not.\n");
    let notes = Notes::parse(&md);
    let path = out.join("kalabalik.tex");
    std::fs::write(&path, &play).unwrap();
    let doc = crate::render::Doc::new(&play);
    let tex = copy_tex(&play, &doc, &notes, false);
    std::fs::write(out.join("kalabalik-notlu.tex"), &tex).unwrap();
    let aux = crate::build::casts(&doc.outline);
    let built = crate::build::build_aside(&path, &tex, &aux, &copy_path(&path)).unwrap();
    println!("{}", built.pdf.display());
}

#[test]
fn a_crowded_margin_keeps_to_numbers() {
    let mut play = String::from(
        "\\documentclass{tiyatro}\n\\karakter{SELİN}{se}\n\\begin{document}\n#sahne Ahır\n",
    );
    for i in 0..6 {
        play.push_str(&format!("\\se Söz {i}.\n"));
    }
    play.push_str("\\end{document}\n");
    let long = "uzun ".repeat(80);
    let md: String = (5..=10)
        .map(|n| format!("- [ ] {n} · **Ece** · {long}\n"))
        .chain(std::iter::once(format!(
            "- [ ] 10 · **Ece** · ikinci {long}\n"
        )))
        .collect();
    let notes = Notes::parse(&md);
    let tex = copy_tex(&play, &crate::render::Doc::new(&play), &notes, false);
    // Short lines one under another: no room for words, only numbers.
    for n in 1..=5 {
        assert!(tex.contains(&format!("{{Ece}}{{{n}}}{{}}")), "{tex}");
    }
    // Two notes on one short line: their numbers and names only.
    let last = tex.lines().find(|l| l.contains("Söz 5")).unwrap();
    assert!(last.contains("{6}{}") && last.contains("{7}{}"), "{last}");
    // The list has every word.
    assert_eq!(tex.matches(long.trim()).count(), 7);
}

#[test]
fn a_margin_excerpt_cuts_at_a_word() {
    assert_eq!(excerpt("kısa not", 40), "kısa not");
    let e = excerpt("bir iki üç dört beş altı yedi sekiz dokuz on", 20);
    assert!(e.ends_with('…') && e.chars().count() <= 21, "{e}");
    assert!(!e.contains("  "));
}

/// A play's notes copy built elsewhere, the play and its notes untouched:
/// `DACTILE_NOTES_PLAY=oyun.tex DACTILE_NOTES_OUT=dir cargo test -- --ignored copy_of_to`.
#[test]
#[ignore]
fn copy_of_to() {
    let play = std::path::PathBuf::from(std::env::var("DACTILE_NOTES_PLAY").unwrap());
    let out = std::path::PathBuf::from(std::env::var("DACTILE_NOTES_OUT").unwrap());
    let text = std::fs::read_to_string(&play).unwrap();
    let notes = Notes::load(&play).unwrap();
    let doc = crate::render::Doc::new(&text);
    let tex = copy_tex(&text, &doc, &notes, false);
    let aux = crate::build::casts(&doc.outline);
    let pdf = out.join(copy_path(&play).file_name().unwrap());
    let built = crate::build::build_aside(&play, &tex, &aux, &pdf).unwrap();
    println!("{}", built.pdf.display());
}
