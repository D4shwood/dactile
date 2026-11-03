use super::*;
use crate::render::Doc;

fn clean(t: &str) {
    let doc = Doc::new(t);
    let known = crate::check::known(t, &doc);
    let found = crate::check::problems(t, &doc, &known);
    assert!(
        found.is_empty(),
        "{:?}\n{t}",
        found.iter().map(|p| &t[p.span.clone()]).collect::<Vec<_>>()
    );
}

#[test]
fn the_skeletons_are_plays() {
    for lang in [Lang::Tr, Lang::En] {
        for t in all(lang).iter().filter(|t| !matches!(t.from, From::Own(_))) {
            let text = make(t, "Kış", "Örnek Yazar", lang).unwrap();
            assert!(library::is_play(&text), "{}", t.name);
            let doc = Doc::new(&text);
            assert_eq!(doc.title(&text).as_deref(), Some("Kış"));
            clean(&text);
            match &t.from {
                From::BuiltIn("three-acts") => {
                    assert_eq!(doc.outline.acts.len(), 3);
                    assert_eq!(doc.outline.scenes.len(), 6);
                    assert_eq!(doc.cast.characters.len(), 3);
                }
                From::BuiltIn("monologue") => {
                    assert_eq!(doc.outline.scenes.len(), 1);
                    assert_eq!(doc.cast.characters.len(), 1);
                }
                _ => {}
            }
        }
    }
}

#[test]
fn a_play_made_a_template_keeps_its_shape_not_its_words() {
    let t = from_play(EXAMPLE, "Köy oyunu");
    assert!(t.starts_with("% Dactile şablonu: Köy oyunu\n"));
    assert!(t.contains("\\karakter*[Selin][Köy öğretmeni]{SELİN}{se}"));
    assert!(t.contains("#perde Kar Yağarken"));
    assert!(t.contains("#sahne İç, Değirmen — Akşam"));
    assert!(t.contains("\\maketitle") && t.contains("\\kisiler"));
    assert!(!t.contains("Amca, yine mi taş durdu?"));
    assert!(!t.contains("Işıklar yavaşça açılır"));
    assert!(!t.contains("Bu oyun bir kış gecesinde"));
    assert!(!t.contains("\\title{Kar Masalı}"));
    let before = Doc::new(EXAMPLE);
    let after = Doc::new(&t);
    assert_eq!(before.outline.scenes.len(), after.outline.scenes.len());
    assert_eq!(before.outline.acts.len(), after.outline.acts.len());
    // Saved, it comes back by its name and makes a play.
    let dir = std::env::temp_dir().join(format!("dactile-tpl-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("koy-oyunu.tex"), &t).unwrap();
    let own = own(&dir, Lang::Tr);
    assert_eq!(own.len(), 1);
    assert_eq!(own[0].name, "Köy oyunu");
    let play = make(&own[0], "Yeni Kış", "B. Yazar", Lang::Tr).unwrap();
    assert!(!play.starts_with(MARK));
    assert_eq!(Doc::new(&play).title(&play).as_deref(), Some("Yeni Kış"));
    assert_eq!(info::get(&play, "author").as_deref(), Some("B. Yazar"));
    clean(&play);
    // The example lands beside it under a name of its own.
    let a = copy_example(&dir).unwrap();
    let b = copy_example(&dir).unwrap();
    assert_ne!(a, b);
    let _ = std::fs::remove_dir_all(&dir);
}

/// `DACTILE_TEMPLATES_OUT=dir cargo test -- --ignored skeletons_to`: each
/// skeleton written there, to build with xelatex.
#[test]
#[ignore]
fn skeletons_to() {
    let Ok(dir) = std::env::var("DACTILE_TEMPLATES_OUT") else {
        return;
    };
    for lang in [Lang::Tr, Lang::En] {
        for t in all(lang).iter().filter(|t| !matches!(t.from, From::Own(_))) {
            let text = make(t, "Kış", "Örnek Yazar", lang).unwrap();
            let name = format!(
                "{}-{lang:?}.tex",
                library::file_name(&t.name).trim_end_matches(".tex")
            );
            std::fs::write(Path::new(&dir).join(name), text).unwrap();
        }
    }
}
