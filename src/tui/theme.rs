//! Colours: poet's dark family, with a curtain claret of its own.

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

pub fn base() -> Style {
    Style::default().fg(FG).bg(BG)
}

pub fn of(s: S) -> Style {
    let st = base();
    match s {
        S::Text | S::Raw => st,
        S::Name => st.add_modifier(Modifier::BOLD),
        S::NameStar => st.fg(BEIGE).add_modifier(Modifier::BOLD),
        S::NameMuted => st.fg(MUTED).add_modifier(Modifier::BOLD),
        S::Direction => st.fg(MUTED).add_modifier(Modifier::ITALIC),
        S::Ref => st.fg(BEIGE),
        S::Under => st.add_modifier(Modifier::UNDERLINED),
        S::Heading => st.fg(CLARET).add_modifier(Modifier::BOLD),
        S::SceneCast => st.fg(MUTED),
        S::Faint => st.fg(FAINT),
        S::RawCmd => st.fg(CLARET),
        S::Warn => st.fg(WARN),
    }
}
