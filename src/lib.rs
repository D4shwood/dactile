//! Dactile — Dynamic Stagewright.
//!
//! The play model: a `.tex` file written for the tiyatro class, read and
//! written back byte for byte. The terminal interface lives in the binary.

pub mod buffer;
pub mod cast;
pub mod complete;
pub mod config;
pub mod file;
pub mod info;
pub mod lang;
pub mod lex;
pub mod library;
pub mod outline;
pub mod render;
pub mod shelf;
pub mod sidecar;
pub mod source;
pub mod spell;
pub mod tex;
pub mod timing;
pub mod writing;

/// The tiyatro class files, carried inside the app so a play needs nothing
/// installed besides XeLaTeX.
pub mod class {
    pub const CLS: &str = include_str!("../tex/tiyatro.cls");
    pub const CORE: &str = include_str!("../tex/tiyatro-core.sty");
    pub const THEME: &str = include_str!("../tex/tiyatro-theme.sty");

    /// The files by name, ready to be written next to a play or into a
    /// TEXINPUTS folder.
    pub const FILES: [(&str, &str); 3] = [
        ("tiyatro.cls", CLS),
        ("tiyatro-core.sty", CORE),
        ("tiyatro-theme.sty", THEME),
    ];

    /// "1.0.0", from the class's `\ProvidesClass` line.
    pub fn version() -> &'static str {
        CLS.lines()
            .find(|l| l.starts_with("\\ProvidesClass"))
            .and_then(|l| l.split(" v").nth(1))
            .and_then(|rest| rest.split_whitespace().next())
            .unwrap_or("?")
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn class_version() {
        assert_eq!(super::class::version(), "1.1.0");
    }
}
