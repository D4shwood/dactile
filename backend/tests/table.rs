//! The command table against the window that calls it.

use dactile_backend::{Ctx, NAMES, dispatch};
use serde_json::json;

/// The desktop's own commands, which `api.ts` calls by name.
const DESKTOP: &[&str] = &[
    "import_read",
    "startup",
    "pick_text",
    "pick_import",
    "launch",
    "reveal",
    "log",
    "pick",
    "pdf",
    "full_screen",
    "pdf_print",
    "open_url",
];

#[test]
fn every_command_the_window_calls_is_in_the_table_or_the_desktops() {
    let api = std::fs::read_to_string("../app/src/lib/api.ts").unwrap();
    // `call` is the way in, not a command.
    for part in api.split("invoke").skip(1) {
        let Some(name) = part.split('"').nth(1) else {
            continue;
        };
        if name == "call"
            || !name.chars().all(|c| c.is_ascii_lowercase() || c == '_')
            || name.is_empty()
        {
            continue;
        }
        assert!(
            NAMES.contains(&name) || DESKTOP.contains(&name),
            "{name} is neither"
        );
    }
    for name in DESKTOP {
        assert!(!NAMES.contains(name), "{name} is in both");
    }
}

#[test]
fn arguments_come_in_the_windows_camel_case() {
    let ctx = Ctx::default();
    let text = "\\documentclass{tiyatro}\n\\karakter{SELİN}{se}\n\\begin{document}\n\\se Merhaba.\n\\end{document}\n";
    let rows = dispatch(&ctx, "cast", json!({ "text": text, "byLines": true })).unwrap();
    assert_eq!(rows.as_array().unwrap().len(), 1, "{rows}");
    // A command that can fail answers with its error, not a wrapped one.
    let e = dispatch(&ctx, "save", json!({ "text": text, "force": false })).unwrap_err();
    assert_eq!(e, "no play open");
    assert!(dispatch(&ctx, "cast", json!({ "text": text })).is_err());
    assert!(dispatch(&ctx, "nothing", json!({})).is_err());
}
