//! The play's details and the look of its cover and cast page, in one form.

use crate::people::{Made, diff};
use dactile::info::{self, PartLook, Side};
use dactile::{lang, tex};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Part {
    /// `coverstyle` or `caststyle`.
    pub command: String,
    pub part: String,
    pub look: PartLook,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverForm {
    /// By command: `title`, `author` and the rest.
    pub fields: BTreeMap<String, String>,
    pub final_copy: bool,
    pub side: String,
    pub own_page: bool,
    pub latin: bool,
    pub parts: Vec<Part>,
}

pub fn read(text: &str) -> CoverForm {
    let parts = |command: &str, list: &[(&str, &str)]| -> Vec<Part> {
        list.iter()
            .map(|(part, _)| Part {
                command: command.to_string(),
                part: part.to_string(),
                look: info::part_look(text, command, part),
            })
            .collect()
    };
    let mut all = parts("coverstyle", &info::COVER_PARTS);
    all.extend(parts("caststyle", &info::CAST_PARTS));
    CoverForm {
        fields: info::FIELDS
            .iter()
            .map(|(cmd, _)| {
                let v = info::get(text, cmd).unwrap_or_default();
                (cmd.to_string(), tex::unescape_text(&v))
            })
            .collect(),
        final_copy: info::is_final(text),
        side: info::scene_align(text).word().to_string(),
        own_page: info::cast_own_page(text),
        latin: info::latin_cast(text),
        parts: all,
    }
}

fn updated(text: &str, form: &CoverForm) -> String {
    let mut new = text.to_string();
    let mut put = |edit: &dyn Fn(&str) -> Option<tex::Edit>| {
        if let Some(e) = edit(&new) {
            new = tex::apply(&new, &[e]);
        }
    };
    for (cmd, value) in &form.fields {
        let value = tex::escape_text(value.trim());
        put(&|t| info::fill(t, cmd, &value));
    }
    // A play with neither option is a draft, and stays without one.
    put(&|t| {
        (info::is_final(t) != form.final_copy)
            .then(|| info::set_final(t, form.final_copy))
            .flatten()
    });
    let side = Side::parse(&form.side).unwrap_or(Side::Center);
    put(&|t| info::set_scene_align(t, side));
    put(&|t| info::set_cast_own_page(t, form.own_page));
    put(&|t| info::set_latin_cast(t, form.latin));
    for p in &form.parts {
        put(&|t| info::set_part_look(t, &p.command, &p.part, &p.look));
    }
    new
}

pub fn write(text: &str, form: &CoverForm) -> Made {
    Made {
        change: diff(text, &updated(text, form)),
        cursor: None,
        message: lang::l("Oyun bilgisi güncellendi.", "Cover details updated.").to_string(),
    }
}

/// The ribbon's size list: the one line, nothing else.
pub fn font_size(text: &str, size: &str) -> Made {
    let new =
        info::set_play_font_size(text, size).map_or(text.to_string(), |e| tex::apply(text, &[e]));
    Made {
        change: diff(text, &new),
        cursor: None,
        message: lang::l(
            "Oyunun yazı boyutu değişti.",
            "The play's text size changed.",
        )
        .to_string(),
    }
}

/// The ribbon's font list: the one line, nothing else.
pub fn font(text: &str, font: &str) -> Made {
    let new = info::set_play_font(text, font).map_or(text.to_string(), |e| tex::apply(text, &[e]));
    Made {
        change: diff(text, &new),
        cursor: None,
        message: lang::l("Oyunun yazı tipi değişti.", "The play's font changed.").to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLAY: &str = "\\documentclass{tiyatro}\n\\title{Kar}\n\\author{Örnek Yazar}\n\\editedby{}\n\\draft{0.1}\n\\address{}\n\\begin{document}\n\\maketitle\n\\kisiler\n\n#sahne Kıyı\n\\end{document}\n";

    #[test]
    fn the_form_changes_only_what_was_changed() {
        assert_eq!(updated(PLAY, &read(PLAY)), PLAY);
        let mut f = read(PLAY);
        f.fields.insert("editedby".into(), "Örnek Kolektif".into());
        f.own_page = false;
        f.latin = true;
        f.side = "left".into();
        f.parts[0].look.align = "left".into();
        let out = updated(PLAY, &f);
        assert!(out.contains("\\editedby{Örnek Kolektif}\n"));
        assert!(out.contains("\\kisiler*\n") && out.contains("\\dramatispersonae\n"));
        assert!(out.contains("\\scenetitleflush[left]\n"));
        assert!(out.contains("\\coverstyle{title}{align=left}\n"));
        assert!(out.contains("\\address{}\n"));
    }
}
