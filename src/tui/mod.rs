//! The terminal interface.

mod panels;
mod popup;
mod theme;
mod writer;

use anyhow::Result;
use ratatui::crossterm::event::{
    self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
    Event, KeyEventKind,
};
use ratatui::crossterm::execute;
use std::io::stdout;
use std::path::Path;
use std::time::Duration;
use writer::Writer;

/// Opens a play in the writer until the user quits.
pub fn run(play: &Path) -> Result<()> {
    let mut w = Writer::open(play)?;
    let mut terminal = ratatui::init();
    let _ = execute!(stdout(), EnableBracketedPaste, EnableMouseCapture);
    let result = (|| -> Result<()> {
        while !w.quit {
            terminal.draw(|f| w.draw(f))?;
            if event::poll(Duration::from_millis(200))? {
                match event::read()? {
                    Event::Key(k) if k.kind != KeyEventKind::Release => w.key(k),
                    Event::Paste(s) => w.paste(&s),
                    Event::Mouse(m) => w.mouse(m),
                    _ => {}
                }
            }
            w.tick();
        }
        Ok(())
    })();
    let closed = w.close();
    let _ = execute!(stdout(), DisableBracketedPaste, DisableMouseCapture);
    ratatui::restore();
    result.and(closed)
}
