//! The play's own settings, on its Dosya page: its pace, which play it is a
//! draft of, and the words its spell checker accepts.

use super::*;
use crate::tui::theme::FG;

impl Writer {
    pub(super) fn play_settings_key(&mut self, k: KeyEvent) {
        let Popup::PlaySettings(sel) = self.popup else {
            return;
        };
        let last = self.sidecar.words.len();
        match k.code {
            KeyCode::Esc => self.popup = Popup::None,
            KeyCode::Up => self.popup = Popup::PlaySettings(sel.saturating_sub(1)),
            KeyCode::Down => self.popup = Popup::PlaySettings((sel + 1).min(last)),
            KeyCode::Enter if sel == 0 => self.wpm_form(true),
            KeyCode::Char('d') if sel == 0 => self.default_pace(),
            KeyCode::Char('x') if sel > 0 => self.unlearn(sel - 1),
            _ => {}
        }
    }

    /// The pace of the settings again, not one of its own.
    fn default_pace(&mut self) {
        if !self.editable() || self.sidecar.wpm.is_none() {
            return;
        }
        self.sidecar.wpm = None;
        self.save_sidecar();
        self.say(
            &lang::f(
                "Konuşma hızı varsayılana döndü: {}.",
                "The pace is the default again: {}.",
                &[&self.default_wpm],
            ),
            false,
        );
    }

    /// A word out of the play's dictionary.
    fn unlearn(&mut self, i: usize) {
        if !self.editable() || i >= self.sidecar.words.len() {
            return;
        }
        self.sidecar.words.remove(i);
        self.save_sidecar();
        self.refresh_known();
        let last = self.sidecar.words.len();
        self.popup = Popup::PlaySettings((i + 1).min(last));
    }

    pub(super) fn draw_play_settings(&self, f: &mut Frame, area: Rect, sel: usize) {
        let w = 64.min(area.width);
        let h = area.height.saturating_sub(4).clamp(10, 24).min(area.height);
        let r = Rect {
            x: area.x + (area.width - w) / 2,
            y: area.y + (area.height - h) / 2,
            width: w,
            height: h,
        };
        popup::float(f, r);
        f.render_widget(popup::panel(lang::l("oyun ayarları", "play settings")), r);
        let inner = popup::inner(r);
        let muted = Style::default().fg(MUTED);
        let fg = Style::default().fg(FG);
        let pace = match self.sidecar.wpm {
            Some(n) => lang::f("{} kelime/dk", "{} words a minute", &[&n]),
            None => lang::f(
                "{} kelime/dk (varsayılan)",
                "{} words a minute (the default)",
                &[&self.default_wpm],
            ),
        };
        let parent = self
            .sidecar
            .parent
            .clone()
            .unwrap_or_else(|| lang::l("yok: ana oyun", "none: a play of its own").to_string());
        let mut lines = vec![
            popup::row(
                vec![
                    popup::mark(sel == 0),
                    Span::styled(lang::l("Konuşma hızı  ", "Pace  "), muted),
                    Span::styled(pace, fg),
                ],
                sel == 0,
                inner.width,
            ),
            Line::from(vec![
                Span::raw("  "),
                Span::styled(lang::l("Taslağı olduğu oyun  ", "A draft of  "), muted),
                Span::styled(popup::fit(&parent, inner.width as usize / 2), fg),
            ]),
            Line::raw(""),
            Line::from(Span::styled(
                lang::f(
                    "  Oyunun sözlüğü ({})",
                    "  The play's words ({})",
                    &[&self.sidecar.words.len()],
                ),
                muted,
            )),
        ];
        let top = lines.len();
        let room = (inner.height as usize).saturating_sub(top + 1).max(1);
        let first = sel.saturating_sub(room);
        for (i, word) in self.sidecar.words.iter().enumerate().skip(first).take(room) {
            let on = sel == i + 1;
            lines.push(popup::row(
                vec![popup::mark(on), Span::styled(word.clone(), fg)],
                on,
                inner.width,
            ));
        }
        f.render_widget(Paragraph::new(lines), inner);
        let keys = if sel == 0 {
            lang::l(
                "⏎ değiştir  d varsayılan  ↓ sözlük  Esc",
                "⏎ change  d default  ↓ words  Esc",
            )
        } else {
            lang::l("x sözlükten çıkar  ↑↓  Esc", "x take it out  ↑↓  Esc")
        };
        let foot = Rect {
            y: inner.y + inner.height.saturating_sub(1),
            height: 1.min(inner.height),
            ..inner
        };
        f.render_widget(Paragraph::new(Span::styled(keys, muted)), foot);
    }
}
