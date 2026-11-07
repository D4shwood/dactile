//! Colours: poet's dark family, with a curtain claret of its own, and a paper
//! page for the play itself.

use dactile::config::Config;
use dactile::render::Style as S;
use ratatui::style::{Color, Modifier, Style};
use std::sync::Mutex;

pub const BG: Color = Color::Rgb(0x26, 0x26, 0x24);
pub const FG: Color = Color::Rgb(0xe8, 0xe6, 0xdd);
pub const MUTED: Color = Color::Rgb(0x9c, 0x9a, 0x92);
pub const FAINT: Color = Color::Rgb(0x5f, 0x5d, 0x56);
/// Starred characters and names inside a line.
pub const BEIGE: Color = Color::Rgb(0xd8, 0xcf, 0xc0);
/// Acts, scenes and what has focus.
pub const CLARET: Color = Color::Rgb(0xb0, 0x44, 0x4a);
pub const WARN: Color = Color::Rgb(0xc8, 0x8a, 0x5a);
pub const SELECTION: Color = Color::Rgb(0x4a, 0x3a, 0x32);
/// The paper page's shadow on the dark around it.
pub const SHADOW: Color = Color::Rgb(0x14, 0x14, 0x13);

/// The colours the play's text is drawn in.
#[derive(Debug, Clone, Copy, PartialEq)]
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
    // Lighter than the claret around the page: it reads on this black.
    heading: Color::Rgb(0xe0, 0x8f, 0x94),
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

/// Sepia: older paper, browner ink.
pub const SEPIA: Palette = Palette {
    bg: Color::Rgb(0xef, 0xe0, 0xc2),
    fg: Color::Rgb(0x3a, 0x2a, 0x1a),
    muted: Color::Rgb(0x7a, 0x62, 0x46),
    faint: Color::Rgb(0xb2, 0x9a, 0x78),
    star: Color::Rgb(0x7a, 0x3b, 0x12),
    name_ref: Color::Rgb(0x6b, 0x4a, 0x2a),
    heading: Color::Rgb(0x8a, 0x33, 0x24),
    warn: Color::Rgb(0xb0, 0x58, 0x20),
    selection: Color::Rgb(0xdd, 0xc5, 0x96),
};

/// Night: a blue-grey page, easy in the dark.
pub const NIGHT: Palette = Palette {
    bg: Color::Rgb(0x1f, 0x25, 0x30),
    fg: Color::Rgb(0xd6, 0xdb, 0xe4),
    muted: Color::Rgb(0x8d, 0x96, 0xa8),
    faint: Color::Rgb(0x4e, 0x57, 0x6a),
    star: Color::Rgb(0xe2, 0xc0, 0x8d),
    name_ref: Color::Rgb(0xc9, 0xb3, 0x8a),
    heading: Color::Rgb(0xd5, 0x7f, 0x86),
    warn: Color::Rgb(0xd7, 0x9a, 0x5a),
    selection: Color::Rgb(0x31, 0x3c, 0x52),
};

/// High contrast: black on white.
pub const CONTRAST: Palette = Palette {
    bg: Color::Rgb(0xff, 0xff, 0xff),
    fg: Color::Rgb(0x00, 0x00, 0x00),
    muted: Color::Rgb(0x30, 0x30, 0x30),
    faint: Color::Rgb(0x70, 0x70, 0x70),
    star: Color::Rgb(0x00, 0x00, 0x00),
    name_ref: Color::Rgb(0x1a, 0x1a, 0x1a),
    heading: Color::Rgb(0xa0, 0x00, 0x10),
    warn: Color::Rgb(0xb0, 0x40, 0x00),
    selection: Color::Rgb(0xff, 0xd8, 0x4d),
};

/// The themes, in the order F6 goes through them.
pub const THEMES: [&str; 5] = ["paper", "sepia", "night", "contrast", "dark"];

/// The accents, as the settings name them, on a light page and on a dark
/// one (the desktop app's, in its theme.css). The first leaves each theme
/// its own headings.
pub const ACCENTS: [(&str, Color, Color); 6] = [
    (
        "claret",
        Color::Rgb(0xb0, 0x44, 0x4a),
        Color::Rgb(0xe0, 0x8f, 0x94),
    ),
    (
        "blue",
        Color::Rgb(0x2f, 0x5f, 0x9e),
        Color::Rgb(0x8a, 0xb4, 0xe8),
    ),
    (
        "green",
        Color::Rgb(0x2f, 0x6f, 0x4f),
        Color::Rgb(0x7c, 0xc4, 0xa0),
    ),
    (
        "plum",
        Color::Rgb(0x7a, 0x3f, 0x8a),
        Color::Rgb(0xc7, 0x9a, 0xd6),
    ),
    (
        "ochre",
        Color::Rgb(0x8a, 0x5a, 0x10),
        Color::Rgb(0xd9, 0xa9, 0x5a),
    ),
    (
        "clay",
        Color::Rgb(0x96, 0x3f, 0x20),
        Color::Rgb(0xd9, 0x77, 0x57),
    ),
];

/// What the settings change in a theme's page.
#[derive(Debug, Clone, Default, PartialEq)]
struct Look {
    accent: String,
    bg: Option<Color>,
    fg: Option<Color>,
}

impl Look {
    fn of(cfg: &Config) -> Look {
        Look {
            accent: cfg.accent.clone(),
            bg: hex(&cfg.page_bg),
            fg: hex(&cfg.page_fg),
        }
    }

    fn plain(&self) -> bool {
        self.bg.is_none() && self.fg.is_none() && accent(&self.accent).is_none()
    }
}

/// The look in use; read from the settings the first time it's asked for.
static LOOK: Mutex<Option<Look>> = Mutex::new(None);
/// The palettes made so far: a few, one for each look tried.
static MADE: Mutex<Vec<(String, Look, &'static Palette)>> = Mutex::new(Vec::new());

/// The settings changed.
pub fn set_look(cfg: &Config) {
    *LOOK.lock().unwrap() = Some(Look::of(cfg));
}

fn theme_palette(theme: &str) -> &'static Palette {
    match theme {
        "sepia" => &SEPIA,
        "night" => &NIGHT,
        "contrast" => &CONTRAST,
        "dark" => &DARK,
        _ => &PAPER,
    }
}

/// A theme's palette, with the settings' accent and page colours.
pub fn palette(theme: &str) -> &'static Palette {
    let look = LOOK
        .lock()
        .unwrap()
        .get_or_insert_with(|| Look::of(&Config::load().unwrap_or_default()))
        .clone();
    if look.plain() {
        return theme_palette(theme);
    }
    let mut made = MADE.lock().unwrap();
    if let Some((_, _, p)) = made.iter().find(|(t, l, _)| t == theme && *l == look) {
        return p;
    }
    let p: &'static Palette = Box::leak(Box::new(styled(theme, &look)));
    made.push((theme.to_string(), look, p));
    p
}

/// A page of another brightness than its theme's takes the inks of a theme
/// of its own brightness: the theme's own wouldn't read on it.
fn styled(theme: &str, look: &Look) -> Palette {
    let own = theme_palette(theme);
    let bg = look.bg.unwrap_or(own.bg);
    let dark = is_dark(bg);
    let base = match (dark == is_dark(own.bg), dark) {
        (true, _) => own,
        (false, true) => &NIGHT,
        (false, false) => &PAPER,
    };
    Palette {
        bg,
        fg: look.fg.unwrap_or(base.fg),
        heading: accent(&look.accent).map_or(base.heading, |a| if dark { a.2 } else { a.1 }),
        ..*base
    }
}

/// An accent other than the first.
fn accent(name: &str) -> Option<&'static (&'static str, Color, Color)> {
    ACCENTS[1..].iter().find(|a| a.0 == name)
}

/// `#rrggbb`.
pub fn hex(s: &str) -> Option<Color> {
    let h = s.trim().strip_prefix('#')?;
    if h.len() != 6 || !h.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let part = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).ok();
    Some(Color::Rgb(part(0)?, part(2)?, part(4)?))
}

fn luminance(c: Color) -> f64 {
    let Color::Rgb(r, g, b) = c else { return 0.0 };
    let linear = |v: u8| {
        let v = f64::from(v) / 255.0;
        if v <= 0.03928 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(r) + 0.7152 * linear(g) + 0.0722 * linear(b)
}

fn is_dark(c: Color) -> bool {
    luminance(c) < 0.18
}

/// From 1 to 21, as the web's guidelines count it.
fn contrast(a: Color, b: Color) -> f64 {
    let (a, b) = (luminance(a), luminance(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

/// Whether the page's text reads on the page with these settings.
pub fn readable(cfg: &Config) -> bool {
    let p = styled(cfg.theme(), &Look::of(cfg));
    contrast(p.fg, p.bg) >= 4.5
}

/// A theme's name on screen.
pub fn theme_name(theme: &str) -> &'static str {
    use dactile::lang::l;
    match theme {
        "sepia" => l("sepya", "sepia"),
        "night" => l("gece", "night"),
        "contrast" => l("yüksek kontrast", "high contrast"),
        "dark" => l("koyu", "dark"),
        _ => l("kâğıt", "paper"),
    }
}

/// The theme with this name on screen, or this name in the settings.
pub fn theme_named(name: &str) -> Option<&'static str> {
    THEMES
        .into_iter()
        .find(|t| *t == name || theme_name(t) == name)
}

/// An accent's name on screen.
pub fn accent_name(accent: &str) -> &'static str {
    use dactile::lang::l;
    match accent {
        "blue" => l("mavi", "blue"),
        "green" => l("yeşil", "green"),
        "plum" => l("erik", "plum"),
        "ochre" => l("toprak", "ochre"),
        "clay" => l("kiremit", "clay"),
        _ => l("bordo", "claret"),
    }
}

/// The accent with this name on screen, or this name in the settings.
pub fn accent_named(name: &str) -> Option<&'static str> {
    ACCENTS
        .iter()
        .map(|a| a.0)
        .find(|a| *a == name || accent_name(a) == name)
}

/// The theme after `theme`, for F6.
pub fn next_theme(theme: &str) -> &'static str {
    let i = THEMES.iter().position(|t| *t == theme).unwrap_or(0);
    THEMES[(i + 1) % THEMES.len()]
}

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
        S::Direction => st.fg(p.muted).add_modifier(Modifier::ITALIC),
        S::Ref => st.fg(p.name_ref),
        S::Under => st.add_modifier(Modifier::UNDERLINED),
        S::Heading => st.fg(p.heading).add_modifier(Modifier::BOLD),
        S::SceneCast => st.fg(p.muted),
        S::Summary => st.fg(p.faint).add_modifier(Modifier::ITALIC),
        S::Faint => st.fg(p.faint),
        S::RawCmd => st.fg(p.heading),
        S::Warn => st.fg(p.warn),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn look(accent: &str, bg: &str, fg: &str) -> Look {
        Look {
            accent: accent.into(),
            bg: hex(bg),
            fg: hex(fg),
        }
    }

    #[test]
    fn every_theme_and_accent_reads() {
        for t in THEMES {
            for (a, _, _) in ACCENTS {
                let p = styled(t, &look(a, "", ""));
                assert!(contrast(p.fg, p.bg) >= 7.0, "{t} text");
                assert!(contrast(p.heading, p.bg) >= 4.5, "{t} {a} heading");
            }
        }
    }

    #[test]
    fn the_accents_are_the_ones_the_settings_keep() {
        assert_eq!(ACCENTS.map(|a| a.0), dactile::config::ACCENTS);
    }

    #[test]
    fn the_first_accent_leaves_the_theme_alone() {
        assert!(look("claret", "", "").plain());
        assert_eq!(styled("sepia", &look("claret", "", "")), SEPIA);
        assert_eq!(styled("sepia", &look("blue", "", "")).fg, SEPIA.fg);
    }

    #[test]
    fn a_page_of_another_brightness_changes_inks() {
        let p = styled("paper", &look("green", "#101820", ""));
        assert_eq!((p.fg, p.muted), (NIGHT.fg, NIGHT.muted));
        assert_eq!(p.heading, ACCENTS[2].2);
        let p = styled("night", &look("", "#fff8e8", ""));
        assert_eq!(p.fg, PAPER.fg);
        // Its own ink wins, for better or worse: the form asks `readable`.
        let p = styled("paper", &look("", "", "#3a2a1a"));
        assert_eq!(p.fg, hex("#3a2a1a").unwrap());
    }

    #[test]
    fn colours_and_what_doesnt_read() {
        assert_eq!(hex("#0a0B0c"), Some(Color::Rgb(10, 11, 12)));
        for bad in ["", "0a0b0c", "#0a0b0", "#0a0b0g", "#çğ0b0c"] {
            assert_eq!(hex(bad), None);
        }
        let mut c = Config {
            theme: "paper".into(),
            ..Config::default()
        };
        assert!(readable(&c));
        c.page_bg = "#808080".into();
        c.page_fg = "#888888".into();
        assert!(!readable(&c));
    }

    #[test]
    fn names_on_screen_find_their_settings() {
        for t in THEMES {
            assert_eq!(theme_named(theme_name(t)), Some(t));
        }
        for (a, _, _) in ACCENTS {
            assert_eq!(accent_named(accent_name(a)), Some(a));
        }
        assert_eq!(theme_named("pembe"), None);
    }
}
