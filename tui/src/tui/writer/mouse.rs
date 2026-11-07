//! The mouse.

use super::*;

/// Presses closer than this make a double or triple click.
const DOUBLE: Duration = Duration::from_millis(400);

impl Writer {
    pub fn mouse(&mut self, m: MouseEvent) {
        // The menus along the top, and the one open.
        if matches!(self.popup, Popup::Backstage(_)) {
            self.backstage_mouse(m);
            return;
        }
        if matches!(self.popup, Popup::None | Popup::Ribbon(_)) && self.ribbon_mouse(m) {
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
                } else if matches!(self.popup, Popup::None)
                    && m.column >= self.sheet().x
                    && m.row >= self.text_top
                {
                    // On the text: the word under it, and its spelling menu.
                    let row = self.shown + (m.row - self.text_top) as usize;
                    let col = m.column.saturating_sub(self.sheet().text_x);
                    if let Some(s) = self.lay.stop_near(row, col) {
                        self.go(s.byte, false);
                        self.suggest();
                    }
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
                    if !drag {
                        match self.click_count(m.column, m.row) {
                            2 => self.choose_word(),
                            3 => self.choose_row(),
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// How many presses in a row this is, at one place: after three it
    /// starts over.
    fn click_count(&mut self, x: u16, y: u16) -> u8 {
        let n = match self.clicks {
            Some((at, cx, cy, n)) if at.elapsed() < DOUBLE && (cx, cy) == (x, y) => n % 3 + 1,
            _ => 1,
        };
        self.clicks = Some((Instant::now(), x, y, n));
        n
    }

    /// The word at the cursor chosen.
    fn choose_word(&mut self) {
        let t = &self.buf.text;
        let cur = self.buf.cursor;
        let from = t[..cur]
            .char_indices()
            .rev()
            .take_while(|(_, c)| c.is_alphanumeric())
            .last()
            .map_or(cur, |(i, _)| i);
        let to = t[cur..]
            .char_indices()
            .find(|(_, c)| !c.is_alphanumeric())
            .map_or(t.len(), |(i, _)| cur + i);
        if from < to {
            self.buf.anchor = Some(from);
            self.buf.cursor = to;
        }
    }

    /// The row the cursor is on chosen, as it stands on the screen.
    fn choose_row(&mut self) {
        let stops = self.row_stops();
        if let (Some(a), Some(b)) = (stops.first(), stops.last()) {
            self.buf.anchor = Some(a.byte);
            self.buf.cursor = b.byte;
        }
    }
}
