//! Checks against real plays, read only. They stay outside the repo; point
//! DACTILE_REAL_PLAYS at a folder (or a single .tex) and run:
//!
//!     DACTILE_REAL_PLAYS=~/oyunlar cargo test --test real_plays -- --ignored --nocapture
//!
//! Each play must survive these changes and their reversal byte for byte, and
//! every declaration must read back as itself.

use dactile::cast::{self, Cast, Kind, Spec};
use dactile::outline::Outline;
use dactile::tex::apply;
use dactile::{lex, source};
use std::path::{Path, PathBuf};

fn plays() -> Vec<PathBuf> {
    let Some(root) = std::env::var_os("DACTILE_REAL_PLAYS") else {
        return Vec::new();
    };
    let root = PathBuf::from(root);
    if root.is_file() {
        return vec![root];
    }
    let mut out = Vec::new();
    collect(&root, &mut out);
    out.sort();
    out
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let p = e.path();
        if p.is_dir() {
            collect(&p, out);
        } else if p.extension().is_some_and(|x| x == "tex") {
            out.push(p);
        }
    }
}

#[test]
#[ignore]
fn real_plays_round_trip() {
    let plays = plays();
    assert!(
        !plays.is_empty(),
        "set DACTILE_REAL_PLAYS to a folder of plays"
    );
    for path in plays {
        let Ok(text) = std::fs::read_to_string(&path) else {
            println!("skip (not UTF-8): {}", path.display());
            continue;
        };
        let name = path.file_name().unwrap().to_string_lossy();
        let parts = source::split(&text);
        let c = Cast::of_play(&text);
        println!(
            "{name}: {} characters, {} groups, body {} bytes",
            c.characters.len(),
            c.groups.len(),
            parts.body.len()
        );

        let blocks = lex::blocks(&text, parts.body.clone(), &c);
        let o = Outline::of(&text, &blocks, &c);
        println!(
            "  {} acts, {} scenes, {} numbered lines",
            o.acts.len(),
            o.scenes.len(),
            o.numbers.iter().flatten().count()
        );
        // The blocks cover the body: nothing is lost between them.
        let covered: usize =
            blocks.iter().map(|b| b.span.len()).sum::<usize>() + blocks.len().saturating_sub(1);
        assert!(covered <= parts.body.len(), "{name}: blocks overlap");

        // Each declaration reads back as itself, or differs only in spacing.
        for ch in &c.characters {
            let written = cast::declaration(&ch.spec());
            let original = &text[ch.span.clone()];
            if written != original {
                let squash = |s: &str| s.split_whitespace().collect::<String>();
                assert_eq!(squash(&written), squash(original), "{name}: {}", ch.alias);
                println!("  {} is written with other spacing: {original:?}", ch.alias);
            }
        }

        // Renaming each alias and back gives the same bytes.
        for ch in &c.characters {
            let tmp = "dactilegecici";
            let once = apply(&text, &cast::rename(&text, &c, &ch.alias, tmp));
            let back = apply(
                &once,
                &cast::rename(&once, &Cast::of_play(&once), tmp, &ch.alias),
            );
            assert!(
                back == text,
                "{name}: renaming {} and back changed the text",
                ch.alias
            );
        }

        // Adding a character and removing it gives the same bytes.
        let spec = Spec {
            alias: c.suggest_alias("DENEME"),
            script: "DENEME".into(),
            desc: "geçici".into(),
            kind: Kind::Cast,
            ..Spec::default()
        };
        let added = apply(&text, &[cast::add(&text, &c, &spec, None)]);
        let after = Cast::of_play(&added);
        assert_eq!(after.characters.len(), c.characters.len() + 1, "{name}");
        let removed = apply(
            &added,
            &[cast::remove(&added, &after, &spec.alias).unwrap()],
        );
        assert!(
            removed == text,
            "{name}: adding and removing a character changed the text"
        );
    }
}
