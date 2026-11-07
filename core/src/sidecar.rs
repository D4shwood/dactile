//! Dactile's own notes about a play, kept next to it and never printed:
//! `oyun.tex` → `oyun.dactile.toml`. Character cards, the play's own words
//! for the spell checker, and its settings.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Card {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub age: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub relations: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub notes: String,
}

impl Card {
    pub fn is_empty(&self) -> bool {
        self.age.is_empty() && self.relations.is_empty() && self.notes.is_empty()
    }
}

/// Who plays whom, one of a play's castings ("A", "B").
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Casting {
    pub name: String,
    /// Actors by the alias of the role.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub actors: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Sidecar {
    /// Words per minute for the running time; the default when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wpm: Option<u32>,
    /// Words the spell checker accepts in this play.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub words: Vec<String>,
    /// The play this one is a draft of: its file name, next to it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    /// It came in through the import; it goes on the home screen's other tab.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub imported: bool,
    /// A book: the plays with this one as their parent are its parts.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub book: bool,
    /// Where this play comes in the book that is its parent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub book_at: Option<usize>,
    /// Which of `castings` is in use.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub casting: usize,
    /// Character cards by alias.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub cards: BTreeMap<String, Card>,
    /// A line or two about each scene, by the id at its heading (`%@k3`).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub summaries: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub castings: Vec<Casting>,
    /// What this Dactile doesn't know (a newer one wrote it): kept as it was.
    #[serde(flatten)]
    pub other: BTreeMap<String, toml::Value>,
}

fn is_zero(n: &usize) -> bool {
    *n == 0
}

pub fn path_for(play: &Path) -> PathBuf {
    let stem = play
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    play.with_file_name(format!("{stem}.dactile.toml"))
}

/// Notes that don't read, moved out of the way so the next save doesn't
/// go over them: `oyun.dactile.toml` → `oyun.dactile.toml.bozuk` (`-2`…).
pub fn put_aside(play: &Path) -> Result<PathBuf> {
    let path = path_for(play);
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let mut to = path.with_file_name(format!("{name}.bozuk"));
    let mut n = 2;
    while to.exists() {
        to = path.with_file_name(format!("{name}.bozuk-{n}"));
        n += 1;
    }
    std::fs::rename(&path, &to).with_context(|| format!("moving {}", path.display()))?;
    Ok(to)
}

impl Sidecar {
    /// The play's sidecar, or an empty one.
    pub fn load(play: &Path) -> Result<Sidecar> {
        let path = path_for(play);
        match std::fs::read_to_string(&path) {
            Ok(t) => toml::from_str(&t).with_context(|| format!("reading {}", path.display())),
            Err(_) => Ok(Sidecar::default()),
        }
    }

    /// Notes that `play` came in through the import, or no longer counts so.
    pub fn mark_imported(play: &Path, on: bool) -> Result<()> {
        let mut side = Sidecar::load(play)?;
        if side.imported != on {
            side.imported = on;
            side.save(play)?;
        }
        Ok(())
    }

    /// Writes it, or removes the file when there's nothing in it.
    pub fn save(&self, play: &Path) -> Result<()> {
        let path = path_for(play);
        if *self == Sidecar::default() {
            if path.exists() {
                std::fs::remove_file(&path)?;
            }
            return Ok(());
        }
        let text = format!(
            "# Dactile's notes for {}: never printed.\n{}",
            play.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default(),
            toml::to_string(self)?
        );
        crate::file::write_atomic(&path, &text)
    }

    /// An alias changed: its card and its actors follow.
    pub fn rename(&mut self, old: &str, new: &str) {
        if let Some(c) = self.cards.remove(old) {
            self.cards.insert(new.to_string(), c);
        }
        for c in &mut self.castings {
            if let Some(a) = c.actors.remove(old) {
                c.actors.insert(new.to_string(), a);
            }
        }
    }

    /// The casting in use, if there is one.
    pub fn current(&self) -> Option<&Casting> {
        self.castings.get(self.casting)
    }

    /// Who plays the role in the casting in use.
    pub fn actor(&self, alias: &str) -> Option<&str> {
        self.current()
            .and_then(|c| c.actors.get(alias))
            .map(String::as_str)
            .filter(|a| !a.is_empty())
    }

    pub fn add_word(&mut self, w: &str) {
        if !self.words.iter().any(|x| x == w) {
            self.words.push(w.to_string());
            self.words.sort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_and_rename() {
        let d = std::env::temp_dir().join(format!("dactile-sidecar-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let play = d.join("oyun.tex");
        assert_eq!(path_for(&play), d.join("oyun.dactile.toml"));
        assert_eq!(Sidecar::load(&play).unwrap(), Sidecar::default());
        let mut s = Sidecar::default();
        s.cards.insert(
            "se".into(),
            Card {
                age: "34".into(),
                notes: "Öğretmen".into(),
                ..Card::default()
            },
        );
        s.add_word("Değirmenci");
        s.castings.push(Casting {
            name: "A".into(),
            actors: [("se".to_string(), "Deniz Kaya".to_string())].into(),
        });
        s.castings.push(Casting {
            name: "B".into(),
            actors: BTreeMap::new(),
        });
        s.summaries.insert("k1".into(), "Kar yağar.".into());
        s.save(&play).unwrap();
        let back = Sidecar::load(&play).unwrap();
        assert_eq!(back, s);
        s.rename("se", "sel");
        assert!(s.cards.contains_key("sel") && !s.cards.contains_key("se"));
        assert_eq!(s.actor("sel"), Some("Deniz Kaya"));
        s.casting = 1;
        assert_eq!(s.actor("sel"), None);
        // Empty: the file goes.
        Sidecar::default().save(&play).unwrap();
        assert!(!path_for(&play).exists());
        std::fs::remove_dir_all(d).unwrap();
    }

    #[test]
    fn unknown_kept_broken_put_aside() {
        let d = std::env::temp_dir().join(format!("dactile-sidecar2-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        let play = d.join("oyun.tex");
        // A newer Dactile's fields survive an older one's save.
        std::fs::write(
            path_for(&play),
            "words = [\"a\"]\nlater = 3\n[future]\nx = \"y\"\n\n[cards.se]\nage = \"34\"\n",
        )
        .unwrap();
        let mut s = Sidecar::load(&play).unwrap();
        assert_eq!(s.cards["se"].age, "34");
        s.add_word("b");
        s.save(&play).unwrap();
        let back = Sidecar::load(&play).unwrap();
        assert_eq!(back.words, ["a", "b"]);
        assert_eq!(back.other["later"].as_integer(), Some(3));
        assert_eq!(back.other["future"]["x"].as_str(), Some("y"));
        assert_eq!(back.cards["se"].age, "34");
        // Broken: moved aside, twice under two names.
        std::fs::write(path_for(&play), "wpm = \"hızlı\"\n").unwrap();
        assert!(Sidecar::load(&play).is_err());
        let a = put_aside(&play).unwrap();
        assert!(a.ends_with("oyun.dactile.toml.bozuk") && !path_for(&play).exists());
        std::fs::write(path_for(&play), "x =").unwrap();
        let b = put_aside(&play).unwrap();
        assert!(b.ends_with("oyun.dactile.toml.bozuk-2"));
        assert_eq!(std::fs::read_to_string(a).unwrap(), "wpm = \"hızlı\"\n");
        std::fs::remove_dir_all(d).unwrap();
    }
}
