//! Projects: plays several people share. Everyone keeps their own plays in
//! their own folder; a project's plays are in the project's, and its
//! members reach them by the role they have there.
//!
//! - `gate/projects.json`: every project, its members and their roles, and
//!   how its plays are written: in turns, or together. Backed up.
//! - `projects/<id>/plays/`: its plays. Backed up.
//!
//! The gate tells each person's server which of these folders it may write
//! in and which it may only read (`Reach`), when it starts it; when that
//! changes, the server is started again.

use crate::people;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// Writes, and says who is in and how the plays are written.
    Admin,
    Writer,
    /// Reads the plays and their PDFs; changes nothing.
    Reader,
}

impl Role {
    pub fn writes(self) -> bool {
        self != Role::Reader
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// One writes a play at a time; whoever opens it next reads until they
    /// are done.
    #[default]
    Turns,
    /// Several write the same play at once.
    Together,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Project {
    /// Its folder's name, `kar-masali-provasi`. It never changes.
    pub id: String,
    pub name: String,
    pub members: BTreeMap<String, Role>,
    #[serde(default)]
    pub mode: Mode,
}

/// A project's folder, as a person's server is told it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Reach {
    pub id: String,
    pub name: String,
    /// Its plays' folder.
    pub dir: PathBuf,
    pub role: Role,
    pub mode: Mode,
}

fn file(home: &Path) -> PathBuf {
    home.join("gate/projects.json")
}

pub fn folder(home: &Path, id: &str) -> PathBuf {
    home.join("projects").join(id).join("plays")
}

fn lang(tr: &str, en: &str) -> String {
    dactile::lang::f(tr, en, &[])
}

pub fn list(home: &Path) -> Vec<Project> {
    std::fs::read_to_string(file(home))
        .ok()
        .and_then(|t| serde_json::from_str::<Vec<Project>>(&t).ok())
        .unwrap_or_default()
        .into_iter()
        .filter(|p| people::valid_id(&p.id))
        .collect()
}

fn keep(home: &Path, all: &[Project]) -> Result<(), String> {
    let text = serde_json::to_string_pretty(all).map_err(|e| e.to_string())?;
    people::write(&file(home), &text, false).map_err(|e| e.to_string())
}

pub fn find(home: &Path, id: &str) -> Option<Project> {
    list(home).into_iter().find(|p| p.id == id)
}

/// The projects `who` is in, with their role in each.
pub fn of(home: &Path, who: &str) -> Vec<(Project, Role)> {
    list(home)
        .into_iter()
        .filter_map(|p| {
            let role = *p.members.get(who)?;
            Some((p, role))
        })
        .collect()
}

/// The folders `who`'s server may reach, theirs aside.
pub fn reach(home: &Path, who: &str) -> Vec<Reach> {
    of(home, who)
        .into_iter()
        .map(|(p, role)| Reach {
            dir: folder(home, &p.id),
            id: p.id,
            name: p.name,
            role,
            mode: p.mode,
        })
        .collect()
}

/// A folder's name from a project's: `Kar Masalı Provası` →
/// `kar-masali-provasi`.
fn slug(name: &str) -> String {
    let mut out = String::new();
    for c in name.to_lowercase().chars() {
        let c = match c {
            'ç' => 'c',
            'ğ' => 'g',
            'ı' | 'i' | 'î' => 'i',
            'ö' => 'o',
            'ş' => 's',
            'ü' | 'û' => 'u',
            'â' => 'a',
            c => c,
        };
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            out.push(c);
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    let out: String = out.trim_end_matches('-').chars().take(28).collect();
    let out = out.trim_end_matches('-').to_string();
    if out.is_empty() { "proje".into() } else { out }
}

fn clean(name: &str) -> Result<String, String> {
    let name = name.split_whitespace().collect::<Vec<_>>().join(" ");
    let name: String = name.chars().filter(|c| !c.is_control()).take(80).collect();
    if name.is_empty() {
        return Err(lang("Projenin bir adı olmalı.", "A project needs a name."));
    }
    Ok(name)
}

/// A new project, with `by` its admin.
pub fn create(home: &Path, name: &str, by: &str) -> Result<Project, String> {
    let name = clean(name)?;
    let mut all = list(home);
    let base = slug(&name);
    let mut id = base.clone();
    let mut n = 2;
    while all.iter().any(|p| p.id == id) || home.join("projects").join(&id).exists() {
        id = format!("{base}-{n}");
        n += 1;
    }
    std::fs::create_dir_all(folder(home, &id)).map_err(|e| e.to_string())?;
    let p = Project {
        id,
        name,
        members: BTreeMap::from([(by.to_string(), Role::Admin)]),
        mode: Mode::Turns,
    };
    all.push(p.clone());
    keep(home, &all)?;
    Ok(p)
}

/// Changes project `id` with `change`, if `by` is its admin.
fn change(
    home: &Path,
    id: &str,
    by: &str,
    change: impl FnOnce(&mut Project) -> Result<(), String>,
) -> Result<Project, String> {
    let mut all = list(home);
    let Some(p) = all.iter_mut().find(|p| p.id == id) else {
        return Err(lang("Böyle bir proje yok.", "There is no such project."));
    };
    if p.members.get(by) != Some(&Role::Admin) {
        return Err(lang(
            "Bunu yalnızca projenin yöneticisi yapabilir.",
            "Only the project's admin can do that.",
        ));
    }
    change(p)?;
    let p = p.clone();
    keep(home, &all)?;
    Ok(p)
}

fn one_admin_left(p: &Project) -> Result<(), String> {
    if p.members.values().any(|r| *r == Role::Admin) {
        Ok(())
    } else {
        Err(lang(
            "Projede en az bir yönetici kalmalı.",
            "A project needs an admin.",
        ))
    }
}

/// Puts `who` in the project with `role`, or takes them out with `None`.
/// Only someone the site knows can be put in.
pub fn set_member(
    home: &Path,
    id: &str,
    by: &str,
    who: &str,
    role: Option<Role>,
) -> Result<Project, String> {
    if role.is_some() && people::find(home, who).is_none() {
        return Err(lang(
            "Bu kişinin sitede hesabı yok.",
            "This person has no account here.",
        ));
    }
    change(home, id, by, |p| {
        match role {
            Some(r) => {
                p.members.insert(who.to_string(), r);
            }
            None => {
                p.members.remove(who);
            }
        }
        one_admin_left(p)
    })
}

pub fn set_mode(home: &Path, id: &str, by: &str, mode: Mode) -> Result<Project, String> {
    change(home, id, by, |p| {
        p.mode = mode;
        Ok(())
    })
}

pub fn rename(home: &Path, id: &str, by: &str, name: &str) -> Result<Project, String> {
    let name = clean(name)?;
    change(home, id, by, |p| {
        p.name = name;
        Ok(())
    })
}

/// `who` leaves the project; the last admin can't.
pub fn leave(home: &Path, id: &str, who: &str) -> Result<Project, String> {
    let mut all = list(home);
    let Some(p) = all
        .iter_mut()
        .find(|p| p.id == id && p.members.contains_key(who))
    else {
        return Err(lang("Bu projede değilsin.", "You aren't in this project."));
    };
    p.members.remove(who);
    one_admin_left(p)?;
    let p = p.clone();
    keep(home, &all)?;
    Ok(p)
}

/// Someone taken off the site is taken out of every project too. A project
/// left with no admin gets the site's owner as its admin: its plays stay
/// someone's.
pub fn forget(home: &Path, who: &str) -> Result<(), String> {
    let owner = people::list(home)
        .into_iter()
        .find(|p| p.role == "owner" && p.id != who)
        .map(|p| p.id);
    let mut all = list(home);
    for p in &mut all {
        if p.members.remove(who).is_some()
            && !p.members.values().any(|r| *r == Role::Admin)
            && let Some(o) = &owner
        {
            p.members.insert(o.clone(), Role::Admin);
        }
    }
    keep(home, &all)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn home(name: &str) -> PathBuf {
        let d =
            std::env::temp_dir().join(format!("dactile-projects-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        people::add(&d, "ada", "Ada", true, None).unwrap();
        people::add(&d, "baran", "Baran", false, None).unwrap();
        people::add(&d, "cem", "Cem", false, None).unwrap();
        d
    }

    #[test]
    fn names_become_folders() {
        assert_eq!(slug("Kar Masalı Provası"), "kar-masali-provasi");
        assert_eq!(slug("  Şölen / Ünlü! "), "solen-unlu");
        assert_eq!(slug("…"), "proje");
    }

    #[test]
    fn the_admin_says_who_is_in_and_one_always_stays() {
        let h = home("roles");
        let p = create(&h, "Kar Masalı", "ada").unwrap();
        assert!(folder(&h, &p.id).is_dir());
        // A second of the same name gets a folder of its own.
        assert_eq!(
            create(&h, "Kar Masalı", "baran").unwrap().id,
            "kar-masali-2"
        );

        set_member(&h, &p.id, "ada", "baran", Some(Role::Writer)).unwrap();
        set_member(&h, &p.id, "ada", "cem", Some(Role::Reader)).unwrap();
        // Only the admin, and only people the site knows.
        assert!(set_member(&h, &p.id, "baran", "cem", Some(Role::Writer)).is_err());
        assert!(set_member(&h, &p.id, "ada", "yok", Some(Role::Writer)).is_err());
        assert!(set_mode(&h, &p.id, "cem", Mode::Together).is_err());
        assert_eq!(
            set_mode(&h, &p.id, "ada", Mode::Together).unwrap().mode,
            Mode::Together
        );

        let cem = reach(&h, "cem");
        assert_eq!(cem.len(), 1);
        assert_eq!(
            (cem[0].role, cem[0].dir.clone()),
            (Role::Reader, folder(&h, &p.id))
        );
        assert_eq!(reach(&h, "baran").len(), 2);

        // The last admin can't go, nor be made something else.
        assert!(leave(&h, &p.id, "ada").is_err());
        assert!(set_member(&h, &p.id, "ada", "ada", Some(Role::Writer)).is_err());
        leave(&h, &p.id, "cem").unwrap();
        assert!(reach(&h, "cem").is_empty());

        // Ada taken off the site: the project goes to the owner, or stays
        // with an admin it has.
        set_member(&h, &p.id, "ada", "baran", Some(Role::Admin)).unwrap();
        forget(&h, "ada").unwrap();
        assert_eq!(
            find(&h, &p.id).unwrap().members.get("baran"),
            Some(&Role::Admin)
        );
        std::fs::remove_dir_all(h).unwrap();
    }
}
