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
    /// Character cards by alias.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub cards: BTreeMap<String, Card>,
}

pub fn path_for(play: &Path) -> PathBuf {
    let stem = play
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    play.with_file_name(format!("{stem}.dactile.toml"))
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

    /// An alias changed: its card follows.
    pub fn rename(&mut self, old: &str, new: &str) {
        if let Some(c) = self.cards.remove(old) {
            self.cards.insert(new.to_string(), c);
        }
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
        s.save(&play).unwrap();
        let back = Sidecar::load(&play).unwrap();
        assert_eq!(back, s);
        s.rename("se", "sel");
        assert!(s.cards.contains_key("sel") && !s.cards.contains_key("se"));
        // Empty: the file goes.
        Sidecar::default().save(&play).unwrap();
        assert!(!path_for(&play).exists());
        std::fs::remove_dir_all(d).unwrap();
    }
}
