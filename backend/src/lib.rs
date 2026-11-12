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

pub use commands::{App, Ctx, NAMES, Res, dispatch, err};
