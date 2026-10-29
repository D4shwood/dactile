//! Acts and scenes of a play, numbered the way the class numbers them, with
//! who appears in each scene and how much is said there.

use crate::cast::Cast;
use crate::lex::{self, Block, Kind, Section};

#[derive(Debug, Clone, PartialEq)]
pub struct Act {
    /// 1, 2, 3… shown as I, II, III.
    pub number: usize,
    pub title: String,
    /// Index of its `\perde` block.
    pub block: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Scene {
    /// Index into `Outline::acts`; None before the first act.
    pub act: Option<usize>,
    /// The counter: 1, 2, 3… within its act.
    pub number: usize,
    /// What the heading shows: the manual number (`\sahne[…][2.1]`) or `number`.
    pub label: String,
    pub place: String,
    /// The id at its heading (`%@k3`), that its summary goes by.
    pub id: Option<String>,
    /// Index of its `\sahne` block; the scene runs to `end` (exclusive).
    pub block: usize,
    pub end: usize,
    /// Aliases of the characters who speak or act in it, first seen first:
    /// the list the class prints under the heading.
    pub cast: Vec<String>,
    /// Lines of dialogue.
    pub lines: usize,
    /// Words spoken.
    pub words: usize,
    /// Words in directions.
    pub direction_words: usize,
}

impl Scene {
    /// "II-3": the key the class writes to the .aux file.
    pub fn key(&self, acts: &[Act]) -> String {
        let act = self.act.map_or(0, |a| acts[a].number);
        format!("{}-{}", roman(act), self.number)
    }
}

/// A preface or notes for the director.
#[derive(Debug, Clone, PartialEq)]
pub struct SectionAt {
    pub which: Section,
    pub title: String,
    pub block: usize,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Outline {
    pub acts: Vec<Act>,
    pub scenes: Vec<Scene>,
    pub sections: Vec<SectionAt>,
    /// The draft line number of each block, as the class prints it in the
    /// margin: one per line of dialogue or direction.
    pub numbers: Vec<Option<usize>>,
}

impl Outline {
    pub fn of(text: &str, blocks: &[Block], cast: &Cast) -> Outline {
        let mut out = Outline::default();
        let mut number = 0;
        let mut open: Option<Scene> = None;
        let close = |open: &mut Option<Scene>, end: usize, scenes: &mut Vec<Scene>| {
            if let Some(mut s) = open.take() {
                s.end = end;
                scenes.push(s);
            }
        };
        for (i, b) in blocks.iter().enumerate() {
            out.numbers.push(b.numbered().then(|| {
                number += 1;
                number
            }));
            match &b.kind {
                Kind::Act { title, .. } => {
                    close(&mut open, i, &mut out.scenes);
                    out.acts.push(Act {
                        number: out.acts.len() + 1,
                        title: title.clone(),
                        block: i,
                    });
                }
                Kind::Scene { place, label, .. } => {
                    close(&mut open, i, &mut out.scenes);
                    let act = out.acts.len().checked_sub(1);
                    let number = out.scenes.iter().filter(|s| s.act == act).count() + 1;
                    open = Some(Scene {
                        act,
                        number,
                        label: label.clone().unwrap_or_else(|| number.to_string()),
                        place: place.clone(),
                        id: crate::structure::id_in(&text[b.span.clone()]).map(str::to_string),
                        block: i,
                        end: i + 1,
                        cast: Vec::new(),
                        lines: 0,
                        words: 0,
                        direction_words: 0,
                    });
                }
                Kind::Section { which, title, .. } => {
                    // The class ends the scene before a preface or notes.
                    close(&mut open, i, &mut out.scenes);
                    out.sections.push(SectionAt {
                        which: *which,
                        title: title.clone(),
                        block: i,
                    });
                }
                Kind::Line { alias, text: t } => {
                    if let Some(s) = open.as_mut() {
                        heard(&mut s.cast, alias);
                        s.lines += 1;
                        s.words += lex::spoken_words(text, t.clone(), cast);
                        s.direction_words += ic_words(text, t.clone(), cast);
                    }
                }
                Kind::Do { aliases, text: t } => {
                    if let Some(s) = open.as_mut() {
                        for a in aliases {
                            heard(&mut s.cast, a);
                        }
                        s.direction_words += lex::count_words(&text[t.clone()]);
                    }
                }
                Kind::Move { aliases, .. } => {
                    if let Some(s) = open.as_mut() {
                        for a in aliases {
                            heard(&mut s.cast, a);
                        }
                    }
                }
                Kind::Action { text: t } | Kind::Stage { text: t, .. } => {
                    if let Some(s) = open.as_mut() {
                        s.direction_words += lex::count_words(&text[t.clone()]);
                    }
                }
                _ => {}
            }
        }
        close(&mut open, blocks.len(), &mut out.scenes);
        out
    }

    /// The scene a block belongs to.
    pub fn scene_at(&self, block: usize) -> Option<usize> {
        self.scenes
            .iter()
            .position(|s| (s.block..s.end).contains(&block))
    }

    /// The act a block belongs to.
    pub fn act_at(&self, block: usize) -> Option<usize> {
        self.acts.iter().rposition(|a| a.block <= block)
    }
}

fn heard(cast: &mut Vec<String>, alias: &str) {
    if !cast.iter().any(|a| a == alias) {
        cast.push(alias.to_string());
    }
}

/// Words inside `\ic{…}` directions of a line.
fn ic_words(text: &str, range: std::ops::Range<usize>, cast: &Cast) -> usize {
    lex::inline(text, range, cast, true)
        .iter()
        .map(|p| match &p.kind {
            lex::Inline::Ic { inner } => lex::count_words(&text[inner.clone()]),
            _ => 0,
        })
        .sum()
}

/// 1 → "I", 4 → "IV", 0 → "" (as LaTeX's `\Roman`).
pub fn roman(mut n: usize) -> String {
    const TABLE: [(usize, &str); 13] = [
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];
    let mut out = String::new();
    for (v, s) in TABLE {
        while n >= v {
            out.push_str(s);
            n -= v;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{lex, source};

    const PLAY: &str = include_str!("../examples/ornek-oyun.tex");

    fn outline(t: &str) -> (Vec<Block>, Outline) {
        let cast = Cast::of_play(t);
        let blocks = lex::blocks(t, source::split(t).body, &cast);
        let o = Outline::of(t, &blocks, &cast);
        (blocks, o)
    }

    #[test]
    fn example_outline() {
        let (blocks, o) = outline(PLAY);
        assert_eq!(o.acts.len(), 2);
        assert_eq!(o.acts[0].title, "Kar Yağarken");
        let got: Vec<(String, String, Vec<String>)> = o
            .scenes
            .iter()
            .map(|s| (s.key(&o.acts), s.label.clone(), s.cast.clone()))
            .collect();
        // The same as the class writes to the .aux file (tex/test/run.sh).
        let v = |x: &[&str]| x.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(
            got,
            vec![
                ("I-1".into(), "1".into(), v(&["se", "oz", "ih"])),
                ("I-2".into(), "2".into(), v(&["ka", "co", "hep", "se"])),
                ("II-1".into(), "2.1".into(), v(&["oz", "ih"])),
            ]
        );
        assert_eq!(o.scenes[0].place, "İç, Değirmen — Akşam");
        assert_eq!(o.scenes[0].lines, 6);
        // Last numbered block is 18, the last number in the draft PDF.
        assert_eq!(o.numbers.iter().flatten().max(), Some(&18));
        let last = blocks.iter().rposition(|b| b.numbered()).unwrap();
        assert_eq!(o.scene_at(last), Some(2));
        assert_eq!(o.act_at(last), Some(1));
        assert_eq!(o.scene_at(0), None);
    }

    #[test]
    fn numbering_restarts_per_act() {
        let t = "\\begin{document}\n\\sahne\n\\perde\n\\sahne\n\\sahne\n\\perde[B]\n\\sahne[x]\n\\end{document}";
        let (_, o) = outline(t);
        let keys: Vec<String> = o.scenes.iter().map(|s| s.key(&o.acts)).collect();
        assert_eq!(keys, vec!["-1", "I-1", "I-2", "II-1"]);
    }

    #[test]
    fn romans() {
        assert_eq!(roman(0), "");
        assert_eq!(roman(4), "IV");
        assert_eq!(roman(14), "XIV");
    }
}
