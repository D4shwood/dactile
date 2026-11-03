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
        list: from_reply(REPLY, "Claude"),
    };
    notes.list[0].state = State::Done;
    notes.list[1].replies.push(Reply {
        author: "Salih".into(),
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
    assert!(md.contains("  - **Salih**: bilerek böyle\n"), "{md}");
    assert!(md.contains("## Arşiv: Düz yazım yanlışları\n"), "{md}");
    assert_eq!(notes.authors(), ["Claude", "Salih", "Atölye"]);
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
    let play = include_str!("../../examples/ornek-oyun.tex");
    let doc = crate::render::Doc::new(play);
    let mut notes = Notes::parse(
        "## Yazım\n\n- [ ] 47 · **Claude** · “taş durdu” → taşı durdu\n  - **Salih**: bakarım\n- [x] 55 · **Claude** · “Biraz ısınsam”: çözüldü\n- [ ] 1 · **Atölye** · kapakta bir not\n- [ ] 49 · **Atölye** · 100% sahne_dışı & {belirsiz}\n",
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
        "\\se Amca, yine mi \\tiyatronote{B0444A}{Claude}{1}{“taş durdu” → taşı durdu}\\tiyatronoted{B0444A}{taş durdu}?"
    ), "{tex}");
    // A line without a quote: at the start of its words; escaped.
    assert!(
        tex.contains(
            "\\se \\tiyatronote{B0703A}{Atölye}{2}{100\\% sahne\\_dışı \\& \\{belirsiz\\}}Ama"
        ),
        "{tex}"
    );
    // The done note left out; the cover's and the heading's only listed.
    assert!(!tex.contains("çözüldü"));
    assert_eq!(tex.matches("\\tiyatronote{").count(), 2);
    assert_eq!(tex.matches("\\tiyatronoteitem{").count(), 4);
    assert!(tex.contains("\\tiyatronotereply{"));
    assert!(
        tex.contains("{3}{Atölye}{1. satır}{kapakta bir not}")
            || tex.contains("{4}{Atölye}{1. satır}{kapakta bir not}"),
        "{tex}"
    );
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
            ix.fix_edits(&note(1, "“çalar.}Mustafa gelir.” → “çalar.} Mustafa gelir.”")),
            ix.fix_edits(&note(2, "“\\ic{gülümser.}. Gel” → “\\ic{gülümser.} Gel”")),
            ix.fix_edits(&note(3, "“Yüzde 50 kaldı” → “yüzde 50% kaldı”")),
        ]
        .concat(),
    );
    assert!(out.contains("\\ic{kapıyı çalar.} Mustafa gelir."), "{out}");
    assert!(out.contains("\\ic{gülümser.} Gel."), "{out}");
    assert!(out.contains("\\ma yüzde 50\\% kaldı."), "{out}");
}
