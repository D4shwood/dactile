//! Zoom in the writer: the terminal's font made bigger while writing, put
//! back on leaving. Konsole takes a profile change by escape sequence
//! (`ESC ] 50 ; Font=Family,size BEL`); other terminals go on as they are.

use std::io::Write;
use std::path::PathBuf;

/// The steps, in percent of the font one writes with otherwise.
pub const LEVELS: [u16; 9] = [80, 90, 100, 110, 125, 150, 175, 200, 250];

/// Whether this terminal can be zoomed from inside.
pub fn supported() -> bool {
    std::env::var_os("KONSOLE_VERSION").is_some()
}

/// The next step up (`dir` 1), down (-1), or back to 100 (0).
pub fn step(now: u16, dir: i8) -> u16 {
    let i = LEVELS.iter().position(|&l| l >= now).unwrap_or(2);
    match dir {
        0 => 100,
        d if d > 0 => LEVELS[(i + 1).min(LEVELS.len() - 1)],
        _ => LEVELS[i.saturating_sub(1)],
    }
}

/// The font Konsole shows when nobody zooms: its profile's, KDE's fixed
/// font, or Konsole's own default. `over` (the settings' `font`, as
/// "Family,size") wins.
pub fn base(over: &str) -> (String, f32) {
    let parse = |s: &str| {
        let mut parts = s.split(',');
        let family = parts.next()?.trim().to_string();
        let size = parts.next()?.trim().parse::<f32>().ok()?;
        (!family.is_empty() && size > 0.0).then_some((family, size))
    };
    if let Some(f) = parse(over) {
        return f;
    }
    let config = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    let data = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    let key = |text: &str, k: &str| {
        text.lines()
            .find_map(|l| l.strip_prefix(k).map(|v| v.trim().to_string()))
    };
    let profile = std::fs::read_to_string(config.join("konsolerc"))
        .ok()
        .and_then(|t| key(&t, "DefaultProfile="));
    if let Some(p) = profile
        && let Ok(t) = std::fs::read_to_string(data.join("konsole").join(p))
        && let Some(f) = key(&t, "Font=").and_then(|v| parse(&v))
    {
        return f;
    }
    if let Ok(t) = std::fs::read_to_string(config.join("kdeglobals"))
        && let Some(f) = key(&t, "fixed=").and_then(|v| parse(&v))
    {
        return f;
    }
    ("Hack".into(), 10.0)
}

/// Sets the terminal's font to `percent` of `base`; false where it can't.
pub fn apply(percent: u16, base: &(String, f32)) -> bool {
    if !supported() {
        return false;
    }
    // Half points: Konsole takes them, and steps stay even.
    let size = (base.1 * percent as f32 / 100.0 * 2.0).round() / 2.0;
    let mut out = std::io::stdout();
    let _ = write!(out, "\x1b]50;Font={},{}\x07", base.0, size);
    let _ = out.flush();
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps() {
        assert_eq!(step(100, 1), 110);
        assert_eq!(step(100, -1), 90);
        assert_eq!(step(250, 1), 250);
        assert_eq!(step(80, -1), 80);
        assert_eq!(step(175, 0), 100);
        assert_eq!(base("Noto Sans Mono, 11"), ("Noto Sans Mono".into(), 11.0));
    }
}
