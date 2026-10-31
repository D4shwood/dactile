//! Roles and who plays them: how much each role says and where it is, the
//! scene/role table for planning rehearsals, and that table as CSV.

use crate::lex::{self, Kind};
use crate::render::Doc;
use crate::sidecar::Sidecar;
use crate::stage::Stage;

/// How much a role says, by index into the cast.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Role {
    pub lines: usize,
    pub words: usize,
    /// Its words at the play's pace.
    pub secs: u32,
    /// The scenes it is in, speaking or not: indexes into the outline.
    pub scenes: Vec<usize>,
}

/// A role in a scene.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Presence {
    Absent,
    /// On stage without a line.
    On,
    Speaks,
}

/// Who is where: `[scene][character]`.
pub fn table(text: &str, doc: &Doc, stage: &Stage) -> Vec<Vec<Presence>> {
    let n = doc.cast.characters.len();
    let base = doc.body.start;
    let mut out = Vec::new();
    for scene in &doc.outline.scenes {
        let mut row = vec![Presence::Absent; n];
        let mut on = |alias: &str, p: Presence| {
            if let Some(i) = doc.cast.index(alias)
                && row[i] != Presence::Speaks
            {
                row[i] = p;
            }
        };
        for a in &scene.cast {
            on(a, Presence::On);
        }
        if let Some(opening) = stage.opening.get(out.len()) {
            for a in opening {
                on(a, Presence::On);
            }
        }
        for rel in scene.block..scene.end {
            for a in stage.at(base + rel) {
                on(a, Presence::On);
            }
            if let Kind::Line { alias, text: t } = &doc.blocks[base + rel].kind
                && lex::spoken_words(text, t.clone(), &doc.cast) > 0
            {
                on(alias, Presence::Speaks);
            }
        }
        out.push(row);
    }
    out
}

/// Each role's lines, words and time, and its scenes from `table`.
pub fn roles(text: &str, doc: &Doc, table: &[Vec<Presence>], wpm: u32) -> Vec<Role> {
    let mut out = vec![Role::default(); doc.cast.characters.len()];
    for b in &doc.blocks[doc.body.clone()] {
        if let Kind::Line { alias, text: t } = &b.kind
            && let Some(i) = doc.cast.index(alias)
        {
            out[i].lines += 1;
            out[i].words += lex::spoken_words(text, t.clone(), &doc.cast);
        }
    }
    let wpm = wpm.max(30) as f64;
    for (i, r) in out.iter_mut().enumerate() {
        r.secs = (r.words as f64 * 60.0 / wpm).round() as u32;
        r.scenes = (0..table.len())
            .filter(|&s| table[s][i] != Presence::Absent)
            .collect();
    }
    out
}

/// Where the table goes beside the play: `oyun-tablo.csv`.
pub fn table_path(play: &std::path::Path, ext: &str) -> std::path::PathBuf {
    let stem = play
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let word = if crate::lang::is_tr() {
        "tablo"
    } else {
        "table"
    };
    play.with_file_name(format!("{stem}-{word}.{ext}"))
}

/// "I-2", or "2" before any act.
pub fn scene_name(doc: &Doc, s: usize) -> String {
    let scene = &doc.outline.scenes[s];
    match scene.act {
        Some(a) => format!(
            "{}-{}",
            crate::outline::roman(doc.outline.acts[a].number),
            scene.label
        ),
        None => scene.label.clone(),
    }
}

/// "SELİN (Deniz Kaya)", or the name alone when nobody plays it yet.
pub fn role_name(doc: &Doc, side: &Sidecar, i: usize) -> String {
    let c = &doc.cast.characters[i];
    match side.actor(&c.alias) {
        Some(a) => format!("{} ({a})", c.script),
        None => c.script.clone(),
    }
}

/// The table for a spreadsheet: a scene per row, a role per column, "●"
/// where it speaks and "○" where it is on without a line.
pub fn csv(doc: &Doc, side: &Sidecar, table: &[Vec<Presence>]) -> String {
    let field = |s: &str| {
        if s.contains([',', '"', '\n']) {
            format!("\"{}\"", s.replace('"', "\"\""))
        } else {
            s.to_string()
        }
    };
    let (scene, place) = if crate::lang::is_tr() {
        ("Sahne", "Yer")
    } else {
        ("Scene", "Place")
    };
    let mut head = vec![scene.to_string(), place.to_string()];
    head.extend((0..doc.cast.characters.len()).map(|i| role_name(doc, side, i)));
    let mut out = head.iter().map(|h| field(h)).collect::<Vec<_>>().join(",");
    out.push('\n');
    for (s, row) in table.iter().enumerate() {
        let mut cells = vec![
            field(&scene_name(doc, s)),
            field(&doc.outline.scenes[s].place),
        ];
        cells.extend(row.iter().map(|p| {
            match p {
                Presence::Absent => "",
                Presence::On => "○",
                Presence::Speaks => "●",
            }
            .to_string()
        }));
        out.push_str(&cells.join(","));
        out.push('\n');
    }
    out
}

/// Text typed into Dactile (an actor's name) as TeX prints it.
pub fn tex_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\textbackslash{}"),
            '{' | '}' | '&' | '%' | '$' | '#' | '_' => {
                out.push('\\');
                out.push(c);
            }
            '~' => out.push_str("\\textasciitilde{}"),
            '^' => out.push_str("\\textasciicircum{}"),
            _ => out.push(c),
        }
    }
    out
}

/// The table as a document for the class: landscape, a scene per row, a
/// role per column with its actor under it, a full square where the role
/// speaks and an empty one where it is on without a line.
pub fn table_tex(doc: &Doc, text: &str, side: &Sidecar, table: &[Vec<Presence>]) -> String {
    let n = doc.cast.characters.len().max(1);
    let tr = crate::lang::is_tr();
    let title = doc.title(text).unwrap_or_default();
    let casting = side
        .current()
        .map(|c| tex_text(&c.name))
        .filter(|c| !c.is_empty());
    let size = match n {
        0..=10 => "\\footnotesize",
        11..=16 => "\\scriptsize",
        _ => "\\tiny",
    };
    let speaks = "\\rule{1.3ex}{1.3ex}";
    let on = "{\\fboxsep0pt\\fboxrule0.4pt\\fbox{\\phantom{\\rule{1.2ex}{1.2ex}}}}";
    let mut o = String::new();
    o.push_str("\\documentclass[final]{tiyatro}\n\\tiyatronoend\n");
    o.push_str("\\usepackage{longtable,array}\n");
    o.push_str("\\geometry{landscape, margin=1.5cm}\n");
    o.push_str("\\pagestyle{plain}\n\\begin{document}\n");
    o.push_str(&format!(
        "\\noindent{{\\large\\scshape {}}}\\hfill {}{}\\par\\smallskip\n",
        title,
        if tr {
            "sahne/rol tablosu"
        } else {
            "scenes and roles"
        },
        casting
            .map(|c| if tr {
                format!(" · {c} dağıtımı")
            } else {
                format!(" · casting {c}")
            })
            .unwrap_or_default()
    ));
    o.push_str(&format!(
        "\\noindent{{\\footnotesize {speaks}~{} \\quad {on}~{}}}\\par\\medskip\n",
        if tr { "konuşur" } else { "speaks" },
        if tr { "sahnede" } else { "on stage" },
    ));
    o.push_str(size);
    o.push_str("\n\\setlength\\tabcolsep{3pt}\\renewcommand\\arraystretch{1.3}\n");
    o.push_str(&format!(
        "\\begin{{longtable}}{{@{{}}l p{{4.2cm}}*{{{n}}}{{>{{\\centering\\arraybackslash}}p{{\\dimexpr(\\linewidth-6.2cm)/{n}-6pt\\relax}}}}@{{}}}}\n"
    ));
    let mut head = format!(
        "{} & {}",
        if tr { "Sahne" } else { "Scene" },
        if tr { "Yer" } else { "Place" }
    );
    for c in &doc.cast.characters {
        let actor = side.actor(&c.alias).map(tex_text).unwrap_or_default();
        head.push_str(&format!(
            " & {{\\scshape {}}}\\newline{{\\itshape {}}}",
            c.script, actor
        ));
    }
    o.push_str(&format!("{head}\\\\\n\\hline\n\\endhead\n"));
    let mut act = None;
    for (s, row) in table.iter().enumerate() {
        let scene = &doc.outline.scenes[s];
        if s > 0 && scene.act != act {
            o.push_str("\\hline\n");
        }
        act = scene.act;
        o.push_str(&format!("{} & {}", scene_name(doc, s), scene.place));
        for p in row {
            o.push_str(" & ");
            o.push_str(match p {
                Presence::Absent => "",
                Presence::On => on,
                Presence::Speaks => speaks,
            });
        }
        o.push_str("\\\\\n");
    }
    o.push_str("\\end{longtable}\n\\end{document}\n");
    o
}

/// What an actor's copy shows of the others' lines.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CopyKind {
    /// All of them, faint.
    #[default]
    Full,
    /// Who speaks, and room for notes where their words were.
    Rehearsal,
    /// Who speaks, the first one to three words, and room for notes.
    Light,
}

impl CopyKind {
    /// What the file name and the cover say of it; nothing for a full copy.
    fn label(self) -> Option<&'static str> {
        match self {
            CopyKind::Full => None,
            CopyKind::Rehearsal => Some(crate::lang::l("prova", "rehearsal")),
            CopyKind::Light => Some(crate::lang::l("prova, hafif", "rehearsal, light")),
        }
    }
}

/// One actor's copy: the roles they play in the casting in use. A role
/// nobody is cast in gets a copy of its own, its actor empty.
#[derive(Debug, Clone, PartialEq)]
pub struct ActorCopy {
    pub actor: String,
    /// Aliases, in the cast's order.
    pub aliases: Vec<String>,
    pub kind: CopyKind,
}

impl ActorCopy {
    /// "Deniz Kaya (SELİN, OZAN).pdf", "SELİN.pdf" when uncast; the
    /// casting's name first when there are others, the kind last:
    /// "SELİN - prova.pdf".
    pub fn file_name(&self, doc: &Doc, side: &Sidecar) -> String {
        let roles: Vec<&str> = self
            .aliases
            .iter()
            .filter_map(|a| doc.cast.get(a).map(|c| c.script.as_str()))
            .collect();
        let casting = match side.current() {
            Some(c) if side.castings.len() > 1 => format!("{} - ", c.name),
            _ => String::new(),
        };
        let mut name = if self.actor.is_empty() {
            format!("{casting}{}", roles.join(", "))
        } else {
            format!("{casting}{} ({})", self.actor, roles.join(", "))
        };
        if let Some(kind) = self.kind.label() {
            name = format!("{name} - {kind}");
        }
        let safe: String = name
            .chars()
            .map(|c| {
                if c == '/' || c == '\\' || c.is_control() {
                    '-'
                } else {
                    c
                }
            })
            .collect();
        format!("{safe}.pdf")
    }
}

/// Everyone cast in the casting in use, an actor with two roles once; then
/// each role on stage that nobody is cast in, alone.
pub fn copies(doc: &Doc, side: &Sidecar, table: &[Vec<Presence>]) -> Vec<ActorCopy> {
    let mut out: Vec<ActorCopy> = Vec::new();
    let mut uncast = Vec::new();
    for (i, c) in doc.cast.characters.iter().enumerate() {
        let Some(actor) = side.actor(&c.alias) else {
            if table.iter().any(|row| row[i] != Presence::Absent) {
                uncast.push(ActorCopy {
                    actor: String::new(),
                    aliases: vec![c.alias.clone()],
                    kind: CopyKind::Full,
                });
            }
            continue;
        };
        match out.iter_mut().find(|x| x.actor == actor) {
            Some(x) => x.aliases.push(c.alias.clone()),
            None => out.push(ActorCopy {
                actor: actor.to_string(),
                aliases: vec![c.alias.clone()],
                kind: CopyKind::Full,
            }),
        }
    }
    out.extend(uncast);
    out
}

/// Where an actor's copies go: `oyun-kopyalar/` beside the play.
pub fn copies_dir(play: &std::path::Path) -> std::path::PathBuf {
    let stem = play
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let word = if crate::lang::is_tr() {
        "kopyalar"
    } else {
        "copies"
    };
    play.with_file_name(format!("{stem}-{word}"))
}

/// Room for a line in a rehearsal copy: about as many lines as it took, and
/// one more to write in.
fn gap_lines(words: &str) -> usize {
    (words.chars().count().div_ceil(55) + 1).min(6)
}

/// One to three, the same for the same words every time.
fn first_words(words: &str) -> usize {
    let h = words.bytes().fold(0xcbf2_9ce4_8422_2325_u64, |h, b| {
        (h ^ b as u64).wrapping_mul(0x100_0000_01b3)
    });
    (h % 3) as usize + 1
}

/// What a rehearsal copy puts where someone else's line was.
fn rehearsal_line(text: &str, range: std::ops::Range<usize>, doc: &Doc, kind: CopyKind) -> String {
    let words = crate::memorize::plain(text, range, &doc.cast);
    let gap = format!("\\tiyatrogap{{{}}}", gap_lines(&words));
    if kind != CopyKind::Light {
        return gap;
    }
    let all: Vec<&str> = words.split_whitespace().collect();
    // Never the whole line.
    let n = first_words(&words).min(all.len().saturating_sub(1));
    if n == 0 {
        return gap;
    }
    let head = all[..n].join(" ");
    let head = head.trim_end_matches(|c: char| !c.is_alphanumeric());
    format!("{}…{gap}", tex_text(head))
}

/// The play as an actor's copy: the class told whose lines to put in bold,
/// their entrances and exits marked, the scenes they are not in down to
/// their headings, and a cover with their scenes and how much they say. A
/// rehearsal copy leaves the others' words out, or all but the first few.
pub fn copy_tex(
    text: &str,
    doc: &Doc,
    stage: &Stage,
    table: &[Vec<Presence>],
    wpm: u32,
    copy: &ActorCopy,
) -> String {
    let tr = crate::lang::is_tr();
    let idx: Vec<usize> = copy
        .aliases
        .iter()
        .filter_map(|a| doc.cast.index(a))
        .collect();
    let base = doc.body.start;
    let mut edits = Vec::new();
    // Scenes without them: the heading and nothing else.
    let mut cut: Vec<std::ops::Range<usize>> = Vec::new();
    for (s, scene) in doc.outline.scenes.iter().enumerate() {
        let absent = idx.iter().all(|&i| table[s][i] == Presence::Absent);
        if absent && scene.end > scene.block + 1 {
            let from = doc.blocks[base + scene.block].span.end;
            let to = doc.blocks[base + scene.end - 1].span.end;
            if to > from {
                cut.push(from..to);
                edits.push(crate::tex::Edit::new(from..to, ""));
            }
        }
    }
    // The others' words, in a rehearsal copy.
    if copy.kind != CopyKind::Full {
        for b in doc.body.clone() {
            let block = &doc.blocks[b];
            if let Kind::Line { alias, text: t } = &block.kind
                && !copy.aliases.contains(alias)
                && !cut.iter().any(|r| r.contains(&block.span.start))
            {
                edits.push(crate::tex::Edit::new(
                    t.clone(),
                    rehearsal_line(text, t.clone(), doc, copy.kind),
                ));
            }
        }
    }
    // Their entrances and exits in directions (in a line of their own,
    // the line is on the band already).
    for m in &stage.moves {
        let b = &doc.blocks[m.block];
        if matches!(b.kind, Kind::Line { .. })
            || !m.who.iter().any(|w| copy.aliases.contains(w))
            || cut.iter().any(|r| r.contains(&b.span.start))
        {
            continue;
        }
        if !edits.iter().any(|e| e.range.start == b.span.start) {
            edits.push(crate::tex::Edit::new(
                b.span.start..b.span.start,
                "\\tiyatromark ",
            ));
        }
    }
    // The cover: their scenes with pages, and how much they say.
    let roles = roles(text, doc, table, wpm);
    let (lines, words, secs) = idx.iter().fold((0, 0, 0), |(l, w, t), &i| {
        (l + roles[i].lines, w + roles[i].words, t + roles[i].secs)
    });
    let scenes: Vec<usize> = (0..table.len())
        .filter(|&s| idx.iter().any(|&i| table[s][i] != Presence::Absent))
        .collect();
    let page = if tr { "s." } else { "p." };
    let mut cover = String::from("\\small\n");
    let what = match (copy.kind, tr) {
        (CopyKind::Full, _) => None,
        (CopyKind::Rehearsal, true) => Some("Prova kopyası"),
        (CopyKind::Rehearsal, false) => Some("Rehearsal copy"),
        (CopyKind::Light, true) => Some("Prova kopyası (hafif)"),
        (CopyKind::Light, false) => Some("Rehearsal copy (light)"),
    };
    if let Some(what) = what {
        cover.push_str(&format!("\\textit{{{what}}}\\par\\medskip\n"));
    }
    if scenes.len() <= 24 {
        cover.push_str("\\begin{tabular}{@{}l l r@{}}\n");
        for &s in &scenes {
            let scene = &doc.outline.scenes[s];
            cover.push_str(&format!(
                "{} & {} & {page}~\\pageref{{tiy@{}}}\\\\\n",
                scene_name(doc, s),
                scene.place,
                scene.key(&doc.outline.acts)
            ));
        }
        cover.push_str("\\end{tabular}\\par\\medskip\n");
    } else {
        let list: Vec<String> = scenes
            .iter()
            .map(|&s| {
                format!(
                    "{} ({page}~\\pageref{{tiy@{}}})",
                    scene_name(doc, s),
                    doc.outline.scenes[s].key(&doc.outline.acts)
                )
            })
            .collect();
        cover.push_str(&list.join(" · "));
        cover.push_str("\\par\\medskip\n");
    }
    cover.push_str(&if tr {
        format!(
            "{lines} replik · {words} kelime · yaklaşık {}",
            crate::timing::format(secs)
        )
    } else {
        format!(
            "{lines} lines · {words} words · about {}",
            crate::timing::format(secs)
        )
    });
    let at = doc.parts.preamble.end;
    edits.push(crate::tex::Edit::new(
        at..at,
        format!(
            "\\tiyatrocopy{{{}}}{{{}}}\n\\tiyatrocopycover{{{}}}\n",
            copy.aliases.join(","),
            tex_text(&copy.actor),
            cover
        ),
    ));
    crate::tex::apply(text, &edits)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sidecar::Casting;

    const PLAY: &str = "\\documentclass{tiyatro}\n\\karakter{SELİN}{se}\n\\karakter{OZAN}{oz}\n\\karakter{İHTİYAR}{ih}\n\\begin{document}\n#perde Bir\n#sahne Ev, iç\n\\girer{se,oz}\n\\se Merhaba, nasılsın?\n\\cikar{oz}\n#sahne Meydan\n\\oz Kar yağıyor.\n\\ih Yağsın.\n\\end{document}\n";

    #[test]
    fn who_is_where() {
        let doc = Doc::new(PLAY);
        let st = crate::stage::read(PLAY, &doc, &[]);
        let t = table(PLAY, &doc, &st);
        use Presence::*;
        assert_eq!(
            t,
            vec![vec![Speaks, On, Absent], vec![Absent, Speaks, Speaks]]
        );
        let r = roles(PLAY, &doc, &t, 120);
        assert_eq!((r[0].lines, r[0].words, r[0].secs), (1, 2, 1));
        assert_eq!(r[1].scenes, vec![0, 1]);
        assert_eq!(scene_name(&doc, 1), "I-2");
    }

    #[test]
    fn an_actors_copy() {
        let doc = Doc::new(PLAY);
        let st = crate::stage::read(PLAY, &doc, &[]);
        let t = table(PLAY, &doc, &st);
        let mut side = Sidecar::default();
        side.castings.push(Casting {
            name: "A".into(),
            actors: [
                ("se".to_string(), "Deniz Kaya".to_string()),
                ("ih".to_string(), "Deniz Kaya".to_string()),
                ("oz".to_string(), "Ali Er".to_string()),
            ]
            .into(),
        });
        let all = copies(&doc, &side, &t);
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].aliases, ["se", "ih"]);
        assert_eq!(
            all[0].file_name(&doc, &side),
            "Deniz Kaya (SELİN, İHTİYAR).pdf"
        );
        let mut only_se = ActorCopy {
            actor: "Deniz Kaya".into(),
            aliases: vec!["se".into()],
            kind: CopyKind::Full,
        };
        let tex = copy_tex(PLAY, &doc, &st, &t, 120, &only_se);
        assert!(tex.contains("\\tiyatrocopy{se}{Deniz Kaya}\n\\tiyatrocopycover{"));
        assert!(tex.contains("I-1 & Ev, iç & s.~\\pageref{tiy@I-1}"));
        assert!(tex.contains("\\tiyatromark \\girer{se,oz}"));
        // Not in the second scene: its heading alone.
        assert!(tex.contains("#sahne Meydan\n\\end{document}"));
        assert!(!tex.contains("Kar yağıyor"));
        // A rehearsal copy: the others' words give way to room for notes.
        only_se.actor = "Ali Er".into();
        only_se.aliases = vec!["oz".into()];
        only_se.kind = CopyKind::Rehearsal;
        let tex = copy_tex(PLAY, &doc, &st, &t, 120, &only_se);
        assert!(tex.contains("\\se \\tiyatrogap{2}\n"));
        assert!(tex.contains("\\oz Kar yağıyor.\n"));
        assert!(tex.contains("\\ih \\tiyatrogap{2}\n"));
        assert!(tex.contains("\\textit{Prova kopyası}"));
        assert_eq!(only_se.file_name(&doc, &side), "Ali Er (OZAN) - prova.pdf");
        only_se.kind = CopyKind::Light;
        let tex = copy_tex(PLAY, &doc, &st, &t, 120, &only_se);
        assert!(tex.contains("\\se Merhaba…\\tiyatrogap{2}\n"));
        assert!(tex.contains("\\ih \\tiyatrogap{2}\n"));
        // Nobody cast: a copy for each role, named after it.
        let none = copies(&doc, &Sidecar::default(), &t);
        assert_eq!(none.len(), 3);
        assert_eq!(none[1].file_name(&doc, &Sidecar::default()), "OZAN.pdf");
        let tex = copy_tex(PLAY, &doc, &st, &t, 120, &none[1]);
        assert!(tex.contains("\\tiyatrocopy{oz}{}\n"));
    }

    #[test]
    fn as_csv() {
        let doc = Doc::new(PLAY);
        let st = crate::stage::read(PLAY, &doc, &[]);
        let t = table(PLAY, &doc, &st);
        let mut side = Sidecar::default();
        side.castings.push(Casting {
            name: "A".into(),
            actors: [("se".to_string(), "Deniz Kaya".to_string())].into(),
        });
        let csv = csv(&doc, &side, &t);
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines[0], "Sahne,Yer,SELİN (Deniz Kaya),OZAN,İHTİYAR");
        assert_eq!(lines[1], "I-1,\"Ev, iç\",●,○,");
        assert_eq!(lines[2], "I-2,Meydan,,●,●");
        let tex = table_tex(&doc, PLAY, &side, &t);
        assert!(tex.contains("{\\scshape SELİN}\\newline{\\itshape Deniz Kaya}"));
        assert!(tex.contains("I-2 & Meydan &  & \\rule"));
        assert_eq!(tex_text("A_b {c}"), "A\\_b \\{c\\}");
    }
}
