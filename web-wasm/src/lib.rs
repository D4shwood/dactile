//! Dactile's commands that only read the play's text, for the browser: what
//! runs as the writer types, with JSON going in and out as it does between
//! the desktop app's two halves. Everything else is the server's.

use dactile_backend::{Ctx, TEXT_ONLY, dispatch_text};
use wasm_bindgen::prelude::*;

/// The command `name` with its arguments, a JSON object: its answer as
/// JSON, or its own error as the desktop's window gets it.
pub fn answer(name: &str, args: &str) -> Result<String, String> {
    let args = serde_json::from_str(args).map_err(|e| format!("{name}: {e}"))?;
    // These commands never look at it.
    let ctx = Ctx::default();
    dispatch_text(&ctx, name, args).map(|v| v.to_string())
}

#[wasm_bindgen]
pub fn call(name: &str, args: &str) -> Result<String, JsValue> {
    answer(name, args).map_err(|e| JsValue::from_str(&e))
}

/// The commands `call` runs, a JSON list.
#[wasm_bindgen]
pub fn names() -> String {
    serde_json::to_string(TEXT_ONLY).unwrap_or_default()
}

/// The interface's language, as the server has it: "en", or Turkish.
#[wasm_bindgen]
pub fn set_lang(lang: &str) {
    use dactile_backend::lang::{Lang, set};
    set(if lang == "en" { Lang::En } else { Lang::Tr });
}

#[cfg(test)]
mod tests {
    use super::answer;
    use serde_json::{Value, json};

    const PLAY: &str = include_str!("../../examples/ornek-oyun.tex");

    #[test]
    fn a_text_command_answers_as_the_backend_does() {
        let args = json!({ "text": PLAY });
        let ours: Value =
            serde_json::from_str(&answer("view", &args.to_string()).unwrap()).unwrap();
        let ctx = dactile_backend::Ctx::default();
        assert_eq!(ours, dactile_backend::dispatch(&ctx, "view", args).unwrap());
    }

    #[test]
    fn every_text_command_runs_with_no_play_open() {
        // Whatever they answer to arguments that aren't theirs, none of them
        // asks for an open play.
        for name in dactile_backend::TEXT_ONLY {
            let said = answer(name, &json!({ "text": PLAY }).to_string());
            assert_ne!(said, Err("no play open".into()), "{name}");
        }
    }

    #[test]
    fn the_rest_is_the_servers() {
        assert!(answer("save", r#"{"text":"","force":false}"#).is_err());
        assert!(answer("open", r#"{"path":"/etc/hosts"}"#).is_err());
        assert!(answer("build", "{}").is_err());
    }
}
