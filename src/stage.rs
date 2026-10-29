//! Who is on stage: entrances and exits, explicit (`\girer{…}`, `\cikar{…}`)
//! or read from the directions ("Selin girer.", "\ozdo Çıkar."), where each
//! scene starts empty, the opening directions set it, and whoever speaks has
//! come in. A guess too weak to count is asked about instead; someone who went
//! out and speaks again is noticed, unless the line says it comes from off
//! stage ("(içeriden)").

use crate::cast::{Cast, lower_tr};
use crate::lex::{self, Inline, Kind};
use crate::render::Doc;
use crate::tex;
use std::ops::Range;

#[derive(Debug, Clone, PartialEq)]
pub struct Move {
    /// The block it is read from, an index into `doc.blocks`.
    pub block: usize,
    pub enter: bool,
    pub who: Vec<String>,
}

/// A guess to confirm: a direction that may be an entrance or an exit.
#[derive(Debug, Clone, PartialEq)]
pub struct Ask {
    pub block: usize,
    pub enter: bool,
    pub who: Vec<String>,
    /// The sentence, lower case: what a "no" is remembered by.
    pub key: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Stage {
    /// For each scene of the outline: who is on stage once its opening
    /// directions are read.
    pub opening: Vec<Vec<String>>,
    /// Entrances and exits that count, in order.
    pub moves: Vec<Move>,
    pub asks: Vec<Ask>,
    /// Lines spoken by someone who went out, with who: block indexes.
    pub off: Vec<(usize, String)>,
    /// Who is on stage after a block, at each block where it changed.
    changes: Vec<(usize, Vec<String>)>,
}

impl Stage {
    /// Who is on stage at block `b` (after it).
    pub fn at(&self, b: usize) -> &[String] {
        match self.changes.iter().rposition(|(at, _)| *at <= b) {
            Some(i) => &self.changes[i].1,
            None => &[],
        }
    }

    pub fn ask_at(&self, b: usize) -> Option<&Ask> {
        self.asks.iter().find(|a| a.block == b)
    }

    pub fn off_at(&self, b: usize) -> Option<&str> {
        self.off.iter().find(|o| o.0 == b).map(|o| o.1.as_str())
    }
}

const ENTER: [&str; 14] = [
    "girer",
    "girerler",
    "girip",
    "gelir",
    "gelirler",
    "gelip",
    "döner",
    "dönerler",
    "belirir",
    "belirirler",
    "görünür",
    "görünürler",
    "katılır",
    "katılırlar",
];
const EXIT: [&str; 16] = [
    "çıkar",
    "çıkarlar",
    "çıkıp",
    "gider",
    "giderler",
    "gidip",
    "çekilir",
    "çekilirler",
    "kaybolur",
    "kaybolurlar",
    "uzaklaşır",
    "uzaklaşırlar",
    "ayrılır",
    "ayrılırlar",
    "çıkış",
    "çıkışı",
];
/// Movement that may or may not take someone off.
const MAYBE: [&str; 8] = [
    "yürür",
    "yürürler",
    "koşar",
    "koşarlar",
    "yönelir",
    "yönelirler",
    "ilerler",
    "ilerlerler",
];
/// Where something is said from when the speaker is off stage.
const FROM_OFF: [&str; 9] = [
    "içeriden",
    "dışarıdan",
    "sesi",
    "telefonda",
    "telefondan",
    "uzaktan",
    "kulisten",
    "arkasından",
    "sahne dışından",
];
/// Words a dative before "gider", "gelir" or "döner" can be without being a
/// place on the stage: "dışarı gider" leaves, "kapıya gider" may not.
const OUTSIDE: [&str; 5] = ["dışarı", "dışarıya", "içeri", "içeriye", "sahneye"];

#[derive(Debug, Clone, Copy, PartialEq)]
enum Verb {
    Enter,
    Exit,
    Maybe,
}

/// The words of a lower-case sentence, without punctuation.
fn words(s: &str) -> Vec<String> {
    s.split(|c: char| !(c.is_alphanumeric() || c == '\'' || c == '’'))
        .filter(|w| !w.is_empty())
        .map(str::to_string)
        .collect()
}

fn verb(sentence: &str) -> Option<Verb> {
    let ws = words(sentence);
    let mut found = None;
    for (i, w) in ws.iter().enumerate() {
        let before = i.checked_sub(1).map(|j| ws[j].as_str());
        // "sahneye çıkar": comes on.
        if w.starts_with("çık") && before == Some("sahneye") {
            return Some(Verb::Enter);
        }
        let dative_place = before.is_some_and(|b| {
            !OUTSIDE.contains(&b)
                && (b.ends_with('a') || b.ends_with('e'))
                && [
                    "gider",
                    "giderler",
                    "gelir",
                    "gelirler",
                    "döner",
                    "dönerler",
                ]
                .contains(&w.as_str())
        });
        let v = if dative_place || MAYBE.contains(&w.as_str()) {
            Some(Verb::Maybe)
        } else if ENTER.contains(&w.as_str()) {
            Some(Verb::Enter)
        } else if EXIT.contains(&w.as_str()) {
            Some(Verb::Exit)
        } else {
            None
        };
        // A sure verb wins over a maybe.
        match (found, v) {
            (_, Some(Verb::Enter | Verb::Exit)) => return v,
            (None, Some(_)) => found = v,
            _ => {}
        }
    }
    found
}

/// What a range of a direction or a line reads as: names for `\alias`,
/// escapes as what they print, other macros left out.
fn plain(text: &str, range: Range<usize>, cast: &Cast) -> String {
    let mut out = String::new();
    for p in lex::inline(text, range, cast, false) {
        match p.kind {
            Inline::Text => out.push_str(&text[p.span]),
            Inline::Ref(a) => out.push_str(&cast.inline_name(&a).unwrap_or(a)),
            Inline::Ic { inner } | Inline::Under { inner } | Inline::Wrap { inner, .. } => {
                out.push_str(&plain(text, inner, cast))
            }
            Inline::Special => out.push_str(&tex::unescape_text(&text[p.span])),
            Inline::Raw => {}
        }
    }
    out
}

/// The characters a lower-case sentence names as who does something: a name
/// with a case ending after an apostrophe ("Ozan'a") is an object, except
/// "with" ("Selin'le").
fn subjects(sentence: &str, cast: &Cast) -> Vec<String> {
    let mut found: Vec<(usize, String)> = Vec::new();
    for c in &cast.characters {
        let mut names: Vec<String> = vec![lower_tr(&c.script)];
        names.extend(c.full.as_deref().map(lower_tr));
        names.extend(cast.inline_name(&c.alias).map(|n| lower_tr(&n)));
        names.retain(|n| !n.trim().is_empty());
        names.sort_by_key(|n| std::cmp::Reverse(n.len()));
        names.dedup();
        for n in &names {
            let mut from = 0;
            while let Some(i) = sentence[from..].find(n.as_str()).map(|i| i + from) {
                from = i + n.len();
                let before_ok = sentence[..i]
                    .chars()
                    .next_back()
                    .is_none_or(|ch| !ch.is_alphanumeric());
                let after = &sentence[i + n.len()..];
                let next = after.chars().next();
                if !before_ok || next.is_some_and(char::is_alphanumeric) {
                    continue;
                }
                if matches!(next, Some('\'' | '’')) {
                    let suffix: String = after
                        .chars()
                        .skip(1)
                        .take_while(|c| c.is_alphabetic())
                        .collect();
                    if !["la", "le", "yla", "yle"].contains(&suffix.as_str()) {
                        continue;
                    }
                }
                if !found.iter().any(|(_, a)| *a == c.alias) {
                    found.push((i, c.alias.clone()));
                }
                break;
            }
        }
    }
    found.sort();
    found.into_iter().map(|(_, a)| a).collect()
}

/// The sentences of a direction, lower case.
fn sentences(s: &str) -> Vec<String> {
    lower_tr(s)
        .split(['.', '!', '?', ';', '\n'])
        .map(|x| x.trim().to_string())
        .filter(|x| !x.is_empty())
        .collect()
}

/// Whether a line's text starts by saying it comes from off stage.
fn from_off(text: &str, range: Range<usize>, cast: &Cast) -> bool {
    let start = text[range.clone()].trim_start();
    let lead = start.starts_with("\\ic") || start.starts_with('(');
    if !lead {
        return false;
    }
    let first = lex::inline(text, range.clone(), cast, true)
        .into_iter()
        .find(|p| !(matches!(p.kind, Inline::Text) && text[p.span.clone()].trim().is_empty()));
    let said = match first.map(|p| p.kind) {
        Some(Inline::Ic { inner }) => lower_tr(&plain(text, inner, cast)),
        _ => {
            let s = lower_tr(start);
            s.split(')').next().unwrap_or("").to_string()
        }
    };
    FROM_OFF.iter().any(|w| said.contains(w))
}

/// Someone there without an entrance: on stage, and at the scene's start if
/// nothing moved them before.
struct Scene {
    on: Vec<String>,
    out: Vec<String>,
    moved: Vec<String>,
    opening: Vec<String>,
}

impl Scene {
    fn present(&mut self, a: &str) {
        if !self.on.iter().any(|x| x == a) && !self.out.iter().any(|x| x == a) {
            self.on.push(a.to_string());
            if !self.moved.iter().any(|x| x == a) && !self.opening.iter().any(|x| x == a) {
                self.opening.push(a.to_string());
            }
        }
    }

    fn apply(&mut self, enter: bool, who: &[String]) {
        apply(&mut self.on, &mut self.out, enter, who);
        for a in who {
            if !self.moved.contains(a) {
                self.moved.push(a.clone());
            }
        }
    }
}

/// Reads the play. `declined`: the keys of the guesses answered no.
pub fn read(text: &str, doc: &Doc, declined: &[String]) -> Stage {
    let mut st = Stage::default();
    let base = doc.body.start;
    for scene in &doc.outline.scenes {
        let mut sc = Scene {
            on: Vec::new(),
            out: Vec::new(),
            moved: Vec::new(),
            opening: Vec::new(),
        };
        // Until the first line of dialogue: the opening directions.
        let mut opening = true;
        for rel in scene.block + 1..scene.end {
            let b = base + rel;
            let block = &doc.blocks[b];
            let before = sc.on.clone();
            // The directions in the block and who they are about when a
            // sentence names nobody.
            let mut directions: Vec<(Range<usize>, Vec<String>)> = Vec::new();
            match &block.kind {
                Kind::Move { enter, aliases } => {
                    let who: Vec<String> = aliases
                        .iter()
                        .filter(|a| doc.cast.get(a).is_some())
                        .cloned()
                        .collect();
                    sc.apply(*enter, &who);
                    st.moves.push(Move {
                        block: b,
                        enter: *enter,
                        who,
                    });
                }
                Kind::Action { text: t } | Kind::Stage { text: t, .. } => {
                    directions.push((t.clone(), Vec::new()));
                }
                Kind::Do { aliases, text: t } => {
                    directions.push((t.clone(), aliases.clone()));
                }
                Kind::Line { alias, text: t } => {
                    opening = false;
                    let ics: Vec<Range<usize>> = lex::inline(text, t.clone(), &doc.cast, true)
                        .into_iter()
                        .filter_map(|p| match p.kind {
                            Inline::Ic { inner } => Some(inner),
                            _ => None,
                        })
                        .collect();
                    // "Geldim. (girer)": the entrance brings them in.
                    let enters_here = ics.iter().any(|r| {
                        sentences(&plain(text, r.clone(), &doc.cast))
                            .iter()
                            .any(|s| {
                                verb(s) == Some(Verb::Enter) && {
                                    let who = subjects(s, &doc.cast);
                                    who.is_empty() || who.contains(alias)
                                }
                            })
                    });
                    if sc.out.contains(alias) && !sc.on.contains(alias) {
                        if !enters_here && !from_off(text, t.clone(), &doc.cast) {
                            st.off.push((b, alias.clone()));
                        }
                    } else if !enters_here {
                        sc.present(alias);
                    }
                    directions.extend(ics.into_iter().map(|r| (r, vec![alias.clone()])));
                }
                _ => {}
            }
            for (range, doer) in directions {
                for s in sentences(&plain(text, range, &doc.cast)) {
                    let mut who = subjects(&s, &doc.cast);
                    if who.is_empty() {
                        who = doer.clone();
                    }
                    if who.is_empty() {
                        continue;
                    }
                    match verb(&s) {
                        Some(v @ (Verb::Enter | Verb::Exit)) => {
                            let enter = v == Verb::Enter;
                            sc.apply(enter, &who);
                            st.moves.push(Move {
                                block: b,
                                enter,
                                who,
                            });
                        }
                        Some(Verb::Maybe) => {
                            let key = format!("{}: {s}", who.join(","));
                            if declined.contains(&key) || st.asks.iter().any(|a| a.block == b) {
                                continue;
                            }
                            // Out if they are on, in if they are not.
                            let enter = !who.iter().any(|a| sc.on.contains(a));
                            st.asks.push(Ask {
                                block: b,
                                enter,
                                who,
                                key,
                            });
                        }
                        // Named in the opening directions, or doing
                        // something: there.
                        None if opening || !doer.is_empty() => {
                            for a in &who {
                                if opening || doer.contains(a) {
                                    sc.present(a);
                                }
                            }
                        }
                        None => {}
                    }
                }
            }
            if sc.on != before {
                st.changes.push((b, sc.on.clone()));
            }
        }
        // The stage empties between scenes.
        if !sc.on.is_empty() {
            st.changes.push((base + scene.end, Vec::new()));
        }
        st.opening.push(sc.opening);
    }
    st.changes.sort_by_key(|c| c.0);
    st
}

fn apply(on: &mut Vec<String>, out: &mut Vec<String>, enter: bool, who: &[String]) {
    for a in who {
        if enter {
            if !on.contains(a) {
                on.push(a.clone());
            }
            out.retain(|x| x != a);
        } else {
            on.retain(|x| x != a);
            if !out.contains(a) {
                out.push(a.clone());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEAD: &str = "\\documentclass{tiyatro}\n\\karakter{SELİN}{se}\n\\karakter[Ozan Usta][Değirmenci]{OZAN}{oz}\n\\karakter{İHTİYAR}{ih}\n\\begin{document}\n";

    fn stage(body: &str) -> (Doc, Stage, String) {
        let text = format!("{HEAD}{body}\\end{{document}}\n");
        let doc = Doc::new(&text);
        let st = read(&text, &doc, &[]);
        (doc, st, text)
    }

    fn block_of(doc: &Doc, text: &str, needle: &str) -> usize {
        let at = text.find(needle).unwrap();
        doc.block_at(at).unwrap()
    }

    #[test]
    fn verbs() {
        assert_eq!(verb("selin girer"), Some(Verb::Enter));
        assert_eq!(verb("ozan usta sahneye çıkar"), Some(Verb::Enter));
        assert_eq!(verb("ozan hızla çıkar"), Some(Verb::Exit));
        assert_eq!(verb("ikisi dışarı giderler"), Some(Verb::Exit));
        assert_eq!(verb("selin kapıya gider"), Some(Verb::Maybe));
        assert_eq!(verb("selin kapıya yürür"), Some(Verb::Maybe));
        assert_eq!(verb("selin pencereden bakar"), None);
    }

    #[test]
    fn subjects_not_objects() {
        let cast = Cast::of_play(&format!("{HEAD}\\end{{document}}"));
        assert_eq!(subjects("selin, ozan'a bakıp çıkar", &cast), ["se"]);
        assert_eq!(subjects("selin'le ozan usta girerler", &cast), ["se", "oz"]);
        assert_eq!(subjects("ihtiyar ile selin girer", &cast), ["ih", "se"]);
        assert!(subjects("selinay gelir", &cast).is_empty());
    }

    #[test]
    fn a_scene() {
        let (doc, st, text) = stage(
            "#sahne Ev\n\n\\action{Selin pencerenin önünde.}\n\n\\se Kar yağıyor.\n\\oz Geldim. \\ic{girer}\n\\ozdo Kapıya yürür.\n\\se Gitme!\n\\action{Ozan çıkar.}\n\\oz Ben yokum artık.\n\\oz \\ic{içeriden} Duyuyorum.\n\\girer{ih}\n\\ih Merhaba.\n\n#sahne Yol\n\n\\ih Yalnızım.\n",
        );
        assert_eq!(st.opening, [vec!["se".to_string()], vec!["ih".to_string()]]);
        assert!(st.moves.iter().any(|m| m.enter && m.who == ["oz"]));
        let ask = &st.asks[0];
        assert_eq!(ask.block, block_of(&doc, &text, "\\ozdo"));
        assert!(!ask.enter);
        assert_eq!(ask.who, ["oz"]);
        assert_eq!(
            st.off,
            [(block_of(&doc, &text, "\\oz Ben yokum"), "oz".into())]
        );
        assert_eq!(st.at(block_of(&doc, &text, "\\action{Ozan")), ["se"]);
        assert_eq!(st.at(block_of(&doc, &text, "\\ih Merhaba")), ["se", "ih"]);
        assert_eq!(st.at(block_of(&doc, &text, "\\ih Yalnız")), ["ih"]);
        // A "no" is remembered by its sentence.
        let st = read(&text, &doc, &["oz: kapıya yürür".into()]);
        assert!(st.asks.is_empty());
    }

    #[test]
    fn the_example() {
        let text = include_str!("../examples/ornek-oyun.tex");
        let doc = Doc::new(text);
        let st = read(text, &doc, &[]);
        // "\ihdo Çıkar." at the end.
        assert!(st.moves.iter().any(|m| !m.enter && m.who == ["ih"]));
        assert!(st.off.is_empty());
    }
}
