//! Small movements: a window unrolling downward as it opens, colours fading.

use ratatui::buffer::Buffer;
use ratatui::style::Color;
use std::time::{Duration, Instant};

/// A window was just drawn over `before`: shows only as much of it, from
/// the top down, as has unrolled in the time since `born`.
pub fn unroll(now: &mut Buffer, before: &Buffer, born: Instant, takes: Duration) {
    let t = born.elapsed().as_secs_f32() / takes.as_secs_f32();
    if t >= 1.0 || now.area != before.area {
        return;
    }
    let p = 1.0 - (1.0 - t).powi(3);
    let a = now.area;
    let changed = |y: u16| (a.x..a.x + a.width).any(|x| now[(x, y)] != before[(x, y)]);
    let rows: Vec<u16> = (a.y..a.y + a.height).filter(|&y| changed(y)).collect();
    let (Some(&top), Some(&bottom)) = (rows.first(), rows.last()) else {
        return;
    };
    let shown = top + ((bottom - top + 1) as f32 * p).ceil() as u16;
    for y in shown..=bottom {
        for x in a.x..a.x + a.width {
            if now[(x, y)] != before[(x, y)] {
                now[(x, y)] = before[(x, y)].clone();
            }
        }
    }
}

/// `a` turned `t` (0 to 1) of the way into `b`.
pub fn mix(a: Color, b: Color, t: f32) -> Color {
    match (a, b) {
        (Color::Rgb(r1, g1, b1), Color::Rgb(r2, g2, b2)) => {
            let m =
                |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t.clamp(0.0, 1.0)).round() as u8;
            Color::Rgb(m(r1, r2), m(g1, g2), m(b1, b2))
        }
        _ => {
            if t < 0.5 {
                a
            } else {
                b
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::layout::Rect;

    #[test]
    fn colours_mix() {
        let (a, b) = (Color::Rgb(0, 100, 200), Color::Rgb(100, 100, 0));
        assert_eq!(mix(a, b, 0.0), a);
        assert_eq!(mix(a, b, 1.0), b);
        assert_eq!(mix(a, b, 0.5), Color::Rgb(50, 100, 100));
    }

    #[test]
    fn a_window_unrolls_from_the_top() {
        let area = Rect::new(0, 0, 10, 10);
        let before = Buffer::empty(area);
        let mut now = before.clone();
        for y in 2..8 {
            now[(3, y)].set_char('x');
        }
        // Just opened: nothing shows yet, then the top rows first.
        let mut early = now.clone();
        unroll(&mut early, &before, Instant::now(), Duration::from_secs(60));
        assert!((2..8).filter(|&y| early[(3, y)].symbol() == "x").count() <= 1);
        // Done: all of it.
        let born = Instant::now() - Duration::from_secs(2);
        let mut late = now.clone();
        unroll(&mut late, &before, born, Duration::from_secs(1));
        assert_eq!(late, now);
    }
}
