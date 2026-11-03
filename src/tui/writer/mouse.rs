//! The mouse.

use super::*;

impl Writer {
    pub fn mouse(&mut self, m: MouseEvent) {
        // The menus along the top, and the one open.
        if matches!(self.popup, Popup::None | Popup::Bar(_)) && self.bar_mouse(m) {
            return;
        }
        // On the help screen the mouse points at keys.
        if let Popup::Help(h) = &mut self.popup {
            if matches!(m.kind, MouseEventKind::Moved | MouseEventKind::Down(_)) {
                h.point(m.column, m.row, &self.keyboard);
            }
            return;
        }
        match m.kind {
            MouseEventKind::ScrollUp => self.scroll = self.scroll.saturating_sub(3),
            MouseEventKind::ScrollDown => {
                let max = self.lay.rows.len().saturating_sub(self.text_height() / 2);
                self.scroll = (self.scroll + 3).min(max);
            }
            // In the margin map, a right click marks an act or scene for ^G.
            MouseEventKind::Down(MouseButton::Right) => {
                if m.column < self.sheet().x
                    && let Some(&(_, at)) = self.map_hits.iter().find(|(y, _)| *y == m.row)
                    && shelf::units(&self.buf.text, &self.doc)
                        .iter()
                        .any(|u| u.span.start == at)
                {
                    if !self.marks.remove(&at) {
                        self.marks.insert(at);
                    }
                    let n = self.marks.len();
                    self.say(
                        &lang::f(
                            "{} işaretli: ^G'de h gizler, t çöpe atar.",
                            "{} marked: in ^G, h hides, t throws away.",
                            &[&n],
                        ),
                        false,
                    );
                }
            }
            MouseEventKind::Down(MouseButton::Left) | MouseEventKind::Drag(MouseButton::Left) => {
                if !matches!(self.popup, Popup::None) || m.row < self.text_top {
                    return;
                }
                // A heading in the margin map: go there.
                if m.column < self.sheet().x {
                    if let Some(&(_, at)) = self.map_hits.iter().find(|(y, _)| *y == m.row)
                        && matches!(m.kind, MouseEventKind::Down(_))
                    {
                        self.go(at, false);
                        self.snap_forward();
                        self.scroll = self
                            .lay
                            .stop_at(at)
                            .map_or(self.scroll, |s| s.row.saturating_sub(2));
                    }
                    return;
                }
                let row = self.shown + (m.row - self.text_top) as usize;
                let col = m.column.saturating_sub(self.sheet().text_x);
                let hit = (row..self.lay.rows.len())
                    .chain((0..row).rev())
                    .find_map(|r| self.lay.stop_near(r, col));
                if let Some(s) = hit {
                    let drag = matches!(m.kind, MouseEventKind::Drag(_));
                    self.go(s.byte, drag);
                }
            }
            _ => {}
        }
    }
}
