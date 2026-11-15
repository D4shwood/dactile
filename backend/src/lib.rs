//! Dactile's commands without a window: the desktop app's and the web
//! server's, the same code.

pub mod casting;
pub mod commands;
pub mod complete;
pub mod cover;
pub mod importing;
pub mod notes;
pub mod people;
pub mod play;
pub mod plays;
pub mod scenes;
pub mod spell;
pub mod summary;
pub mod view;

/// The interface's language, for those who have the backend alone.
pub use dactile::lang;

pub use commands::{App, Ctx, NAMES, Res, TEXT_ONLY, dispatch, dispatch_text, err};
