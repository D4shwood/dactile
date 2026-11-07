//! `dactile-light install` / `dactile-light uninstall`: the command, a menu entry with
//! its icons, and the tiyatro class where XeLaTeX finds it, so plays compile
//! from any folder.

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The name of the menu entry and of the icons.
const ID: &str = "io.github.d4shwood.dactile-light";

/// Each icon: its folder in the hicolor theme, its extension, and the file.
/// `packaging/icons.sh` makes the PNGs.
const ICONS: [(&str, &str, &[u8]); 9] = [
    ("scalable", "svg", include_bytes!("../icons/light.svg")),
    ("16x16", "png", include_bytes!("../icons/16x16.png")),
    ("24x24", "png", include_bytes!("../icons/24x24.png")),
    ("32x32", "png", include_bytes!("../icons/32x32.png")),
    ("48x48", "png", include_bytes!("../icons/48x48.png")),
    ("64x64", "png", include_bytes!("../icons/64x64.png")),
    ("128x128", "png", include_bytes!("../icons/128x128.png")),
    ("256x256", "png", include_bytes!("../icons/256x256.png")),
    ("512x512", "png", include_bytes!("../icons/512x512.png")),
];

fn home() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

fn bin() -> PathBuf {
    home().join(".local/bin/dactile-light")
}

fn data() -> PathBuf {
    dirs::data_dir().unwrap_or_else(|| home().join(".local/share"))
}

fn entry(data: &Path) -> PathBuf {
    data.join(format!("applications/{ID}.desktop"))
}

/// The entry's name before it took the app's id.
fn old_entry(data: &Path) -> PathBuf {
    data.join("applications/dactile-light.desktop")
}

fn theme(data: &Path) -> PathBuf {
    data.join("icons/hicolor")
}

fn icon(data: &Path, size: &str, ext: &str) -> PathBuf {
    theme(data).join(format!("{size}/apps/{ID}.{ext}"))
}

/// The menu entry and its icons, for the command at `exec`.
fn add_menu(data: &Path, exec: &str) -> Result<Vec<String>> {
    for (size, ext, file) in ICONS {
        let path = icon(data, size, ext);
        fs::create_dir_all(path.parent().unwrap())?;
        fs::write(&path, file)?;
    }
    let _ = Command::new("gtk-update-icon-cache")
        .args(["-q", "-t", "-f"])
        .arg(theme(data))
        .output();

    let path = entry(data);
    fs::create_dir_all(path.parent().unwrap())?;
    fs::write(
        &path,
        format!(
            "[Desktop Entry]\nType=Application\nName=Dactile Light\nGenericName=Stage play writer\n\
             Comment=Write plays for the tiyatro XeLaTeX class\nExec={}\nTerminal=true\n\
             Icon={ID}\nCategories=Office;WordProcessor;\n",
            exec_arg(exec)
        ),
    )?;
    let old = old_entry(data);
    if old.symlink_metadata().is_ok() {
        fs::remove_file(&old)?;
    }
    let _ = Command::new("update-desktop-database")
        .arg(path.parent().unwrap())
        .output();
    Ok(vec![
        format!("menu entry: {}", path.display()),
        format!("icons: {}", theme(data).display()),
    ])
}

/// Takes away what `add_menu` wrote, and an entry of the old name.
fn remove_menu(data: &Path) -> Result<Vec<String>> {
    let mut done = Vec::new();
    let icons = ICONS.iter().map(|(size, ext, _)| icon(data, size, ext));
    for p in [entry(data), old_entry(data)].into_iter().chain(icons) {
        if p.symlink_metadata().is_ok() {
            fs::remove_file(&p)?;
            done.push(format!("removed {}", p.display()));
        }
    }
    // Only a cache that is there: it may hold other apps' icons.
    if theme(data).join("icon-theme.cache").is_file() {
        let _ = Command::new("gtk-update-icon-cache")
            .args(["-q", "-t", "-f"])
            .arg(theme(data))
            .output();
    }
    let _ = Command::new("update-desktop-database")
        .arg(data.join("applications"))
        .output();
    Ok(done)
}

/// The user's own TeX tree (TEXMFHOME), usually ~/texmf.
fn texmf() -> PathBuf {
    Command::new("kpsewhich")
        .args(["-var-value", "TEXMFHOME"])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
        .map_or_else(|| home().join("texmf"), PathBuf::from)
}

fn class_dir() -> PathBuf {
    texmf().join("tex/latex/dactile")
}

pub fn install() -> Result<Vec<String>> {
    let mut done = Vec::new();
    let exe = std::env::current_exe()?.canonicalize()?;
    let target = bin();
    fs::create_dir_all(target.parent().unwrap())?;
    if exe != target {
        // A copy, so the extracted archive can go away.
        let tmp = target.with_extension("new");
        fs::copy(&exe, &tmp).with_context(|| format!("copying to {}", tmp.display()))?;
        fs::rename(&tmp, &target)?;
    }
    done.push(format!("command: {}", target.display()));

    done.extend(add_menu(&data(), &target.to_string_lossy())?);

    let dir = class_dir();
    fs::create_dir_all(&dir)?;
    for (name, text) in dactile::class::FILES {
        fs::write(dir.join(name), text)?;
    }
    // A TeX tree with an ls-R database needs it refreshed; ~/texmf usually has none.
    if dir.ancestors().any(|d| d.join("ls-R").is_file()) {
        let _ = Command::new("mktexlsr").arg(texmf()).output();
    }
    done.push(format!(
        "tiyatro class {}: {}",
        dactile::class::version(),
        dir.display()
    ));
    Ok(done)
}

/// A path as a desktop entry's `Exec=` takes it: quoted, so a space in it
/// is not two arguments, with what the quotes and the field codes mean
/// escaped.
fn exec_arg(path: &str) -> String {
    let mut out = String::from("\"");
    for c in path.chars() {
        match c {
            '"' | '`' | '$' | '\\' => {
                // Once for the quotes, once for the file's string escapes.
                out.push_str("\\\\");
                out.push(c);
            }
            '%' => out.push_str("%%"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

pub fn uninstall() -> Result<Vec<String>> {
    let mut done = Vec::new();
    let p = bin();
    if p.symlink_metadata().is_ok() {
        fs::remove_file(&p)?;
        done.push(format!("removed {}", p.display()));
    }
    done.extend(remove_menu(&data())?);
    let dir = class_dir();
    if dir.is_dir() {
        fs::remove_dir_all(&dir)?;
        done.push(format!("removed {}", dir.display()));
    }
    Ok(done)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A folder of the test's own, in place of the user's data folder.
    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "dactile-light-install-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn menu_goes_in_and_comes_out() {
        let data = scratch("menu");
        let old = old_entry(&data);
        fs::create_dir_all(old.parent().unwrap()).unwrap();
        fs::write(&old, "[Desktop Entry]\n").unwrap();
        let other = data.join("applications/other.desktop");
        fs::write(&other, "[Desktop Entry]\n").unwrap();

        add_menu(&data, "/home/a b/.local/bin/dactile-light").unwrap();
        let text = fs::read_to_string(entry(&data)).unwrap();
        assert!(text.contains("\nName=Dactile Light\n"));
        assert!(text.contains("\nExec=\"/home/a b/.local/bin/dactile-light\"\n"));
        assert!(text.contains("\nIcon=io.github.d4shwood.dactile-light\n"));
        assert!(!old.exists());
        assert!(
            data.join("icons/hicolor/scalable/apps/io.github.d4shwood.dactile-light.svg")
                .is_file()
        );
        for (size, ext, file) in ICONS {
            assert_eq!(fs::read(icon(&data, size, ext)).unwrap(), file);
        }

        fs::write(&old, "[Desktop Entry]\n").unwrap();
        remove_menu(&data).unwrap();
        assert!(!entry(&data).exists());
        assert!(!old.exists());
        for (size, ext, _) in ICONS {
            assert!(!icon(&data, size, ext).exists());
        }
        assert!(other.is_file());
        fs::remove_dir_all(&data).unwrap();
    }

    #[test]
    fn uninstall_with_nothing_there() {
        let data = scratch("empty");
        assert!(remove_menu(&data).unwrap().is_empty());
        fs::remove_dir_all(&data).unwrap();
    }

    /// Each PNG is as wide and as high as its folder says.
    #[test]
    fn icons_are_their_size() {
        for (size, ext, file) in ICONS {
            if ext == "png" {
                let side = |at: usize| {
                    u32::from_be_bytes(file[at..at + 4].try_into().unwrap()).to_string()
                };
                assert_eq!(format!("{}x{}", side(16), side(20)), size);
            }
        }
    }

    #[test]
    fn exec_quoted() {
        assert_eq!(
            super::exec_arg("/home/a b/.local/bin/dactile"),
            "\"/home/a b/.local/bin/dactile\""
        );
        assert_eq!(super::exec_arg("/x/100%/d$"), "\"/x/100%%/d\\\\$\"");
    }
}
