//! The check against XeLaTeX itself: each broken body must stop a real
//! build, and the check must find it first; the quiet fixes must give a play
//! that builds. Needs xelatex:
//!
//!     cargo test --test broken_sources -- --ignored

use dactile::{build, check, render::Doc, tex};
use std::path::Path;

const PLAY: &str = include_str!("../examples/ornek-oyun.tex");

fn with_body(extra: &str) -> String {
    PLAY.replace("\\end{document}", &format!("{extra}\n\\end{{document}}"))
}

fn problems(t: &str) -> Vec<check::Problem> {
    let doc = Doc::new(t);
    check::problems(t, &doc, &check::known(t, &doc))
}

fn builds(dir: &Path, name: &str, t: &str) -> bool {
    let play = dir.join(format!("{name}.tex"));
    std::fs::write(&play, t).unwrap();
    build::build(&play).is_ok()
}

#[test]
#[ignore]
fn the_check_finds_what_stops_xelatex() {
    let dir = std::env::temp_dir().join(format!("dactile-test-broken-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    assert!(builds(&dir, "clean", PLAY), "the example must build");

    // Stops the build; the check stops it first.
    let stopping = [
        ("unclosed", "\n\\se Bir \\ic{gülerek kaçar.\n"),
        ("extra", "\n\\action{Kapı açılır.}}\n"),
        ("unknown", "\n\\oz Gel \\sein yanına.\n"),
        ("command", "\n\\action{\\foo çalar.}\n"),
    ];
    for (name, body) in stopping {
        let t = with_body(body);
        assert!(!builds(&dir, name, &t), "{name}: XeLaTeX should fail");
        assert!(
            check::stops(&problems(&t)).is_some(),
            "{name}: the check missed it"
        );
    }

    // Stops the build; the quiet fixes mend it.
    let fixable = [
        ("amp", "\n\\se Ali & Veli.\n"),
        ("dollar", "\n\\se 5$ verdim.\n"),
        ("caret", "\n\\se x^2 dedi.\n"),
        ("under", "\n\\action{dosya_adı}\n"),
    ];
    for (name, body) in fixable {
        let t = with_body(body);
        assert!(!builds(&dir, name, &t), "{name}: XeLaTeX should fail");
        let p = problems(&t);
        assert!(check::stops(&p).is_none(), "{name}: should be quiet");
        let fixed = tex::apply(&t, &check::fixes(&p, None));
        assert!(
            problems(&fixed).is_empty(),
            "{name}: still found after fixing"
        );
        assert!(
            builds(&dir, &format!("{name}-fixed"), &fixed),
            "{name}: fixed, still fails"
        );
    }

    // %50 doesn't fail, it hides the rest of the line: fixed all the same.
    let t = with_body("\n\\se %50 indirim var.\n");
    let fixed = tex::apply(&t, &check::fixes(&problems(&t), None));
    assert!(fixed.contains("\\se \\%50 indirim var."));
    assert!(builds(&dir, "percent", &fixed));
    let _ = std::fs::remove_dir_all(&dir);
}
