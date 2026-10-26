//! `dactile install` / `dactile uninstall`: the command, a menu entry, and the
//! tiyatro class where XeLaTeX finds it, so plays compile from any folder.

use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn home() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

fn bin() -> PathBuf {
    home().join(".local/bin/dactile")
}

fn desktop() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| home().join(".local/share"))
        .join("applications/dactile.desktop")
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

    let entry = desktop();
    fs::create_dir_all(entry.parent().unwrap())?;
    fs::write(
        &entry,
        format!(
            "[Desktop Entry]\nType=Application\nName=Dactile\nGenericName=Stage play writer\n\
             Comment=Write plays for the tiyatro XeLaTeX class\nExec={}\nTerminal=true\n\
             Icon=accessories-text-editor\nCategories=Office;WordProcessor;\n",
            target.display()
        ),
    )?;
    let _ = Command::new("update-desktop-database")
        .arg(entry.parent().unwrap())
        .output();
    done.push(format!("menu entry: {}", entry.display()));

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

pub fn uninstall() -> Result<Vec<String>> {
    let mut done = Vec::new();
    for p in [bin(), desktop()] {
        if p.symlink_metadata().is_ok() {
            fs::remove_file(&p)?;
            done.push(format!("removed {}", p.display()));
        }
    }
    let dir = class_dir();
    if dir.is_dir() {
        fs::remove_dir_all(&dir)?;
        done.push(format!("removed {}", dir.display()));
    }
    Ok(done)
}
