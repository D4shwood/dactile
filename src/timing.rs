//! How long a scene or a play runs on stage, from its words: dialogue at a
//! speaking pace, directions with some time for what they describe.

use crate::outline::Scene;

/// Words per minute when nothing else is set.
pub const WPM: u32 = 130;

/// Seconds for a scene: spoken words at `wpm`, and a third of that pace for
/// the words of its directions (actions take longer than they read).
pub fn scene_secs(s: &Scene, wpm: u32) -> u32 {
    let wpm = wpm.max(30) as f64;
    let spoken = s.words as f64;
    let directions = s.direction_words as f64 / 3.0;
    ((spoken + directions) * 60.0 / wpm).round() as u32
}

/// "45 sn", "12 dk", "1 sa 05 dk" (in English "45 s", "12 min", "1 h 05 min").
pub fn format(secs: u32) -> String {
    let (s, m, h) = if crate::lang::is_tr() {
        ("sn", "dk", "sa")
    } else {
        ("s", "min", "h")
    };
    let mins = (secs + 30) / 60;
    if secs < 60 {
        format!("{secs} {s}")
    } else if mins < 60 {
        format!("{mins} {m}")
    } else {
        format!("{} {h} {:02} {m}", mins / 60, mins % 60)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::Doc;

    #[test]
    fn times() {
        assert_eq!(format(40), "40 sn");
        assert_eq!(format(130), "2 dk");
        assert_eq!(format(3900), "1 sa 05 dk");
        let doc = Doc::new(include_str!("../examples/ornek-oyun.tex"));
        let s = &doc.outline.scenes[0];
        // 30 spoken words and 24 in directions at 130 a minute.
        assert_eq!((s.words, s.direction_words), (30, 24));
        assert_eq!(scene_secs(s, WPM), 18);
    }
}
