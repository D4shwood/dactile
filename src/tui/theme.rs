//! Colours: poet's dark family, with a curtain claret of its own, and a paper
//! page for the play itself.

use dactile::render::Style as S;
use ratatui::style::{Color, Modifier, Style};

pub const BG: Color = Color::Rgb(0x15, 0x15, 0x15);
pub const FG: Color = Color::Rgb(0xe6, 0xe6, 0xe6);
pub const MUTED: Color = Color::Rgb(0x88, 0x88, 0x88);
pub const FAINT: Color = Color::Rgb(0x4a, 0x4a, 0x4a);
/// Starred characters and names inside a line.
pub const BEIGE: Color = Color::Rgb(0xd8, 0xcf, 0xc0);
/// Acts, scenes and what has focus.
pub const CLARET: Color = Color::Rgb(0xb0, 0x44, 0x4a);
pub const WARN: Color = Color::Rgb(0xc8, 0x8a, 0x5a);
pub const SELECTION: Color = Color::Rgb(0x3a, 0x2a, 0x2c);
/// The paper page's shadow on the dark around it.
pub const SHADOW: Color = Color::Rgb(0x08, 0x08, 0x08);

/// The colours the play's text is drawn in.
pub struct Palette {
    pub bg: Color,
    pub fg: Color,
    pub muted: Color,
    pub faint: Color,
    pub star: Color,
    pub name_ref: Color,
    pub heading: Color,
    pub warn: Color,
    pub selection: Color,
}

pub const DARK: Palette = Palette {
    bg: BG,
    fg: FG,
    muted: MUTED,
    faint: FAINT,
    star: BEIGE,
    name_ref: BEIGE,
    heading: CLARET,
    warn: WARN,
    selection: SELECTION,
};

/// Ink on warm paper.
pub const PAPER: Palette = Palette {
    bg: Color::Rgb(0xe9, 0xe3, 0xd6),
    fg: Color::Rgb(0x22, 0x1f, 0x1b),
    muted: Color::Rgb(0x6e, 0x66, 0x5a),
    faint: Color::Rgb(0xa8, 0x9f, 0x90),
    star: Color::Rgb(0x6b, 0x3b, 0x1f),
    name_ref: Color::Rgb(0x5e, 0x4a, 0x34),
    heading: Color::Rgb(0x93, 0x2c, 0x34),
    warn: Color::Rgb(0xb0, 0x58, 0x20),
    selection: Color::Rgb(0xd2, 0xc4, 0xa6),
};

pub fn base() -> Style {
    Style::default().fg(FG).bg(BG)
}

/// A piece of the play in a palette.
pub fn on(s: S, p: &Palette) -> Style {
    let st = Style::default().fg(p.fg).bg(p.bg);
    match s {
        S::Text | S::Raw => st,
        S::Name | S::Title => st.add_modifier(Modifier::BOLD),
        S::NameStar => st.fg(p.star).add_modifier(Modifier::BOLD),
        S::NameMuted => st.fg(p.muted).add_modifier(Modifier::BOLD),
        S::Direction => st.fg(p.muted).add_modifier(Modifier::ITALIC),
        S::Ref => st.fg(p.name_ref),
        S::Under => st.add_modifier(Modifier::UNDERLINED),
        S::Heading => st.fg(p.heading).add_modifier(Modifier::BOLD),
        S::SceneCast => st.fg(p.muted),
        S::Faint => st.fg(p.faint),
        S::RawCmd => st.fg(p.heading),
        S::Warn => st.fg(p.warn),
    }
}
