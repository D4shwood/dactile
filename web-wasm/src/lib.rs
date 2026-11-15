//! Dactile's commands that only read the play's text, for the browser: what
//! runs on every keystroke, with JSON going in and out as it does between
//! the desktop app's two halves.

use wasm_bindgen::prelude::*;

/// The live view of a play, as the `view` command answers it.
#[wasm_bindgen]
pub fn view(text: &str) -> String {
    serde_json::to_string(&dactile_backend::view::view(text)).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_view_is_the_backends() {
        let text =
            "\\documentclass{tiyatro}\n\\begin{document}\n\\sahne{İç}{Gece}\n\\end{document}\n";
        let ours: serde_json::Value = serde_json::from_str(&super::view(text)).unwrap();
        let theirs = serde_json::to_value(dactile_backend::view::view(text)).unwrap();
        assert_eq!(ours, theirs);
    }
}
