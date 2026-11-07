//! The cursor is drawn here, not left to the terminal: a terminal may be set
//! not to blink, or lose its cursor on paper. It blinks in every theme, with
//! animations off too. The terminal's own stays hidden.

use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::Position;
use ratatui::style::Modifier;
use std::cell::Cell;
use std::time::{Duration, Instant};

/// How long it stays lit, and dark.
const HALF: u128 = 530;

thread_local! {
    static SINCE: Cell<Option<Instant>> = const { Cell::new(None) };
}

fn age() -> Duration {
    SINCE.with(|s| {
        let since = s.get().unwrap_or_else(Instant::now);
        s.set(Some(since));
        since.elapsed()
    })
}

fn lit(age: Duration) -> bool {
    (age.as_millis() / HALF).is_multiple_of(2)
}

fn left(age: Duration) -> Duration {
    Duration::from_millis((HALF - age.as_millis() % HALF) as u64)
}

/// A key: lit at once, as it should be while typing.
pub fn restart() {
    SINCE.with(|s| s.set(Some(Instant::now())));
}

pub fn on() -> bool {
    lit(age())
}

/// How long the event loop may sleep and not miss a change.
pub fn until_change() -> Duration {
    left(age())
}

fn paint(buf: &mut Buffer, at: Position) {
    if let Some(cell) = buf.cell_mut(at) {
        cell.modifier.toggle(Modifier::REVERSED);
    }
}

/// The cursor at `at`, if it is lit and on the screen. After what is drawn
/// under it.
pub fn at(f: &mut Frame, at: Position) {
    if on() {
        paint(f.buffer_mut(), at);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::layout::Rect;

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    #[test]
    fn it_goes_on_and_off_by_the_half() {
        assert!(lit(ms(0)));
        assert!(lit(ms(529)));
        assert!(!lit(ms(530)));
        assert!(!lit(ms(1059)));
        assert!(lit(ms(1060)));
    }

    #[test]
    fn the_loop_wakes_for_the_change_and_no_sooner() {
        assert_eq!(left(ms(0)), ms(530));
        assert_eq!(left(ms(529)), ms(1));
        assert_eq!(left(ms(530)), ms(530));
    }

    #[test]
    fn a_key_lights_it() {
        restart();
        assert!(on());
        assert!(until_change() <= ms(530));
    }

    #[test]
    fn off_the_screen_is_nothing() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 2, 1));
        paint(&mut buf, Position::new(9, 9));
        paint(&mut buf, Position::new(1, 0));
        assert!(buf[(1, 0)].modifier.contains(Modifier::REVERSED));
        assert!(!buf[(0, 0)].modifier.contains(Modifier::REVERSED));
    }
}
