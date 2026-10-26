//! The terminal interface.

mod theme;
mod writer;

use anyhow::Result;
use ratatui::crossterm::event::{
    self, DisableBracketedPaste, EnableBracketedPaste, Event, KeyEventKind,
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
    let _ = execute!(stdout(), EnableBracketedPaste);
    let result = (|| -> Result<()> {
        while !w.quit {
            terminal.draw(|f| w.draw(f))?;
            if event::poll(Duration::from_millis(200))? {
                match event::read()? {
                    Event::Key(k) if k.kind != KeyEventKind::Release => w.key(k),
                    Event::Paste(s) => w.paste(&s),
                    _ => {}
                }
            }
            w.tick();
        }
        Ok(())
    })();
    let closed = w.close();
    let _ = execute!(stdout(), DisableBracketedPaste);
    ratatui::restore();
    result.and(closed)
}
