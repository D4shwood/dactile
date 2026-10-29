//! The terminal interface: the home screen and the writer.

mod anim;
mod board;
mod home;
mod keys;
mod panels;
mod popup;
mod theme;
mod writer;

use anyhow::Result;
use dactile::config::Config;
use home::Home;
use ratatui::crossterm::event::{
    self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
    Event, KeyEventKind,
};
use ratatui::crossterm::execute;
use std::io::stdout;
use std::path::{Path, PathBuf};
use std::time::Duration;
use writer::Writer;

pub enum Start {
    Home,
    Play(PathBuf),
}

// One value for the whole run, so the size difference costs nothing.
#[allow(clippy::large_enum_variant)]
enum Screen {
    Home(Home),
    Writer(Box<Writer>),
}

/// Runs the app until the user quits. Opening a play from the command line
/// goes straight to it, and quitting the writer then quits the app.
pub fn run(start: Start) -> Result<()> {
    let mut cfg = Config::load()?;
    let first_run = !Config::exists();
    let direct = matches!(start, Start::Play(_)) && !first_run;
    let mut screen = match start {
        Start::Play(p) if !first_run => Screen::Writer(Box::new(open(&p, &mut cfg)?)),
        Start::Play(p) => {
            let mut h = Home::new(&cfg, true);
            h.pending = Some(p);
            Screen::Home(h)
        }
        Start::Home => Screen::Home(Home::new(&cfg, first_run)),
    };
    let mut terminal = ratatui::init();
    let _ = execute!(stdout(), EnableBracketedPaste, EnableMouseCapture);
    let result = (|| -> Result<()> {
        loop {
            terminal.draw(|f| match &mut screen {
                Screen::Home(h) => h.draw(f, &cfg),
                Screen::Writer(w) => w.draw(f),
            })?;
            // Faster while something moves on screen.
            let moving = match &screen {
                Screen::Home(h) => h.animating(&cfg),
                Screen::Writer(w) => w.animating(),
            };
            let wait = Duration::from_millis(if moving { 16 } else { 200 });
            let ev = if event::poll(wait)? {
                Some(event::read()?)
            } else {
                None
            };
            match &mut screen {
                Screen::Home(h) => {
                    if let Some(Event::Key(k)) = ev
                        && k.kind != KeyEventKind::Release
                    {
                        match h.key(k, &mut cfg) {
                            home::Action::Stay => {}
                            home::Action::Quit => return Ok(()),
                            home::Action::Open(p) => match open(&p, &mut cfg) {
                                Ok(w) => screen = Screen::Writer(Box::new(w)),
                                Err(e) => eprintln!("{e:#}"),
                            },
                        }
                    }
                }
                Screen::Writer(w) => {
                    match ev {
                        Some(Event::Key(k)) if k.kind != KeyEventKind::Release => w.key(k),
                        Some(Event::Paste(s)) => w.paste(&s),
                        Some(Event::Mouse(m)) => w.mouse(m),
                        _ => {}
                    }
                    w.tick();
                    if w.quit {
                        close(w, &mut cfg)?;
                        if direct {
                            return Ok(());
                        }
                        screen = Screen::Home(Home::new(&cfg, false));
                    }
                }
            }
        }
    })();
    // A writer still open (an error above) saves what it has.
    if let Screen::Writer(w) = &mut screen
        && !w.quit
    {
        let _ = close(w, &mut cfg);
    }
    let _ = execute!(stdout(), DisableBracketedPaste, DisableMouseCapture);
    ratatui::restore();
    result
}

fn open(path: &Path, cfg: &mut Config) -> Result<Writer> {
    let w = Writer::open(path, cfg)?;
    cfg.opened(path);
    let _ = cfg.save();
    Ok(w)
}

/// Saves the play and remembers the view toggles.
fn close(w: &mut Writer, cfg: &mut Config) -> Result<()> {
    w.close()?;
    let v = w.view();
    cfg.typewriter = v.typewriter;
    cfg.focus = v.focus;
    cfg.dim = v.dim;
    cfg.paper = v.theme != "dark";
    cfg.theme = v.theme;
    cfg.animations = v.animations;
    cfg.pages = v.pages;
    cfg.save()
}
