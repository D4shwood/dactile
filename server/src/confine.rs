//! The browser names files by path, as the desktop's window does. Here every
//! such path must stay inside the person's own folder, and names that become
//! part of a path must be plain names.

use serde_json::Value;
use std::path::{Component, Path, PathBuf};

/// `p`, if it is inside `root` once links and `..` are followed. A path that
/// isn't there yet counts by its nearest folder that is.
pub fn inside(root: &Path, p: &str) -> Result<PathBuf, String> {
    let p = Path::new(p);
    if !p.is_absolute() {
        return Err(refused());
    }
    let mut there = p.to_path_buf();
    let mut rest = Vec::new();
    let real = loop {
        match there.canonicalize() {
            Ok(r) => break r,
            Err(_) => {
                let Some(name) = there.file_name() else {
                    return Err(refused());
                };
                rest.push(name.to_owned());
                if !there.pop() {
                    return Err(refused());
                }
            }
        }
    };
    // What isn't there yet is plain names only.
    if rest
        .iter()
        .any(|n| !matches!(Path::new(n).components().next(), Some(Component::Normal(_))))
    {
        return Err(refused());
    }
    let full = rest.iter().rev().fold(real, |p, n| p.join(n));
    if full.starts_with(root) {
        Ok(full)
    } else {
        Err(refused())
    }
}

/// A file or folder name with no way out of its folder in it.
pub fn plain(name: &str) -> Result<(), String> {
    let mut parts = Path::new(name).components();
    match (parts.next(), parts.next()) {
        (Some(Component::Normal(_)), None) if !name.contains(['/', '\\']) => Ok(()),
        _ => Err(refused()),
    }
}

fn refused() -> String {
    dactile::lang::l(
        "Bu dosyaya buradan ulaşılamaz.",
        "That file can't be reached from here.",
    )
    .to_string()
}

/// A command's arguments, checked before it runs: paths stay in `root`,
/// names stay names, and the plays' folder is the server's to say.
pub fn check(root: &Path, name: &str, args: &Value) -> Result<(), String> {
    if name == "set_setting" && args.get("key").and_then(Value::as_str) == Some("plays_dir") {
        return Err(refused());
    }
    let Some(args) = args.as_object() else {
        return Ok(());
    };
    for (key, v) in args {
        match (key.as_str(), v) {
            ("path" | "pdf", Value::String(p)) => {
                inside(root, p)?;
            }
            ("paths", Value::Array(ps)) => {
                for p in ps {
                    inside(root, p.as_str().ok_or_else(refused)?)?;
                }
            }
            ("file" | "draft", Value::String(n)) => plain(n)?,
            ("path" | "pdf" | "paths" | "file", Value::Null) => {}
            ("path" | "pdf" | "paths" | "file", _) => return Err(refused()),
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn root() -> PathBuf {
        let d = std::env::temp_dir().join(format!("dactile-confine-{}", std::process::id()));
        std::fs::create_dir_all(d.join("plays/Kar Masalı")).unwrap();
        std::fs::write(d.join("plays/Kar Masalı/kar-masali.tex"), "x").unwrap();
        d.canonicalize().unwrap()
    }

    #[test]
    fn paths_inside_pass_and_the_rest_do_not() {
        let r = root();
        let play = r.join("plays/Kar Masalı/kar-masali.tex");
        let s = |p: &Path| p.display().to_string();
        assert_eq!(inside(&r, &s(&play)).unwrap(), play);
        // Not there yet, but inside.
        assert!(inside(&r, &s(&r.join("plays/Yeni/yeni.tex"))).is_ok());
        for bad in [
            "/etc/passwd".to_string(),
            "plays/x.tex".to_string(),
            s(&r.join("plays/../../etc/passwd")),
            s(&r.join("plays/yok/../../../etc")),
            format!("{}/../{}", r.display(), "x"),
        ] {
            assert!(inside(&r, &bad).is_err(), "{bad}");
        }
        // A link out of the folder is outside.
        let link = r.join("plays/dışarı");
        let _ = std::fs::remove_file(&link);
        std::os::unix::fs::symlink("/etc", &link).unwrap();
        assert!(inside(&r, &s(&link.join("passwd"))).is_err());
    }

    #[test]
    fn commands_are_checked_by_their_arguments() {
        let r = root();
        let play = r
            .join("plays/Kar Masalı/kar-masali.tex")
            .display()
            .to_string();
        assert!(check(&r, "open", &json!({"path": play})).is_ok());
        assert!(check(&r, "open", &json!({"path": "/etc/hosts"})).is_err());
        assert!(
            check(
                &r,
                "book_gather",
                &json!({"paths": [play, "/etc"], "title": "K"})
            )
            .is_err()
        );
        assert!(
            check(
                &r,
                "play_make",
                &json!({"path": play, "file": "yeni.tex", "draft": null})
            )
            .is_ok()
        );
        assert!(
            check(
                &r,
                "play_make",
                &json!({"path": play, "file": "../yeni.tex"})
            )
            .is_err()
        );
        assert!(check(&r, "fork_file", &json!({"path": play, "draft": "a/b"})).is_err());
        assert!(
            check(
                &r,
                "set_setting",
                &json!({"key": "plays_dir", "value": "/"})
            )
            .is_err()
        );
        assert!(
            check(
                &r,
                "set_setting",
                &json!({"key": "theme", "value": "paper"})
            )
            .is_ok()
        );
        assert!(check(&r, "view", &json!({"text": "/etc/passwd"})).is_ok());
    }
}
