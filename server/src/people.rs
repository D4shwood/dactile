//! Who may come through the gate, and the links that let them in.
//!
//! Both are small files under the gate's folder, written by the owner's
//! commands (`dactile-gate add`, `invite`, `remove`) and read by the gate
//! as it runs: nothing is kept in its memory, so a change counts at once.
//!
//! - `gate/people`: a line each, `id<TAB>role<TAB>name<TAB>mail`, the last
//!   one their Google account's address if they sign in with it. Backed up.
//! - `gate/secret/invites`: a line each, `token ends id`. A link works
//!   once, and only until it ends. Not backed up.

use crate::door::{now, random, same};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub struct Person {
    /// Their folder's name and their name to the gate: `ada`.
    pub id: String,
    /// "owner" or "writer". Nothing differs by it yet.
    pub role: String,
    /// As they are shown: "Ada Yıldız".
    pub name: String,
    /// Their Google account's address, in small letters; empty, they come in
    /// by links alone.
    pub mail: String,
}

fn people_file(home: &Path) -> PathBuf {
    home.join("gate/people")
}

fn invites_file(home: &Path) -> PathBuf {
    home.join("gate/secret/invites")
}

/// Where a person's Dactile keeps everything of theirs.
pub fn folder(home: &Path, id: &str) -> PathBuf {
    home.join("people").join(id)
}

/// A name for a folder and a line of a file: small letters, digits and
/// hyphens, a letter or digit first, 32 at the most.
pub fn valid_id(id: &str) -> bool {
    let ok = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit();
    !id.is_empty() && id.len() <= 32 && id.starts_with(ok) && id.chars().all(|c| ok(c) || c == '-')
}

/// Whole, to a file of its own, then moved over; `private` for the owner's
/// eyes alone.
fn write(file: &Path, text: &str, private: bool) -> std::io::Result<()> {
    if let Some(d) = file.parent() {
        std::fs::create_dir_all(d)?;
    }
    let tmp = file.with_extension("new");
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(if private { 0o600 } else { 0o644 })
        .open(&tmp)?;
    f.write_all(text.as_bytes())?;
    f.sync_all()?;
    std::fs::rename(&tmp, file)
}

pub fn list(home: &Path) -> Vec<Person> {
    std::fs::read_to_string(people_file(home))
        .unwrap_or_default()
        .lines()
        .filter_map(|l| {
            let mut parts = l.split('\t');
            let id = parts.next().filter(|id| valid_id(id))?;
            let role = parts.next()?;
            let name = parts.next().unwrap_or(id).trim();
            let mail = parts.next().unwrap_or("").trim();
            Some(Person {
                id: id.to_string(),
                role: role.to_string(),
                name: if name.is_empty() { id } else { name }.to_string(),
                mail: mail.to_lowercase(),
            })
        })
        .collect()
}

pub fn find(home: &Path, id: &str) -> Option<Person> {
    list(home).into_iter().find(|p| p.id == id)
}

fn keep(home: &Path, people: &[Person]) -> std::io::Result<()> {
    let text: String = people
        .iter()
        .map(|p| format!("{}\t{}\t{}\t{}\n", p.id, p.role, p.name, p.mail))
        .collect();
    write(&people_file(home), &text, false)
}

/// Whose Google account `mail` is.
pub fn find_by_mail(home: &Path, mail: &str) -> Option<Person> {
    let mail = mail.trim().to_lowercase();
    list(home)
        .into_iter()
        .find(|p| !p.mail.is_empty() && p.mail == mail)
}

/// A new person, or a new name or role for one who is there. `mail`, if
/// given, is their Google account's address ("" takes it away); not given,
/// the one they had stays.
pub fn add(
    home: &Path,
    id: &str,
    name: &str,
    owner: bool,
    mail: Option<&str>,
) -> Result<(), String> {
    if !valid_id(id) {
        return Err(format!(
            "{id}: small letters, digits and hyphens only, 32 at the most"
        ));
    }
    let name = name.replace(['\t', '\n', '\r'], " ");
    let mut people = list(home);
    let mail = match mail.map(|m| m.trim().to_lowercase()) {
        Some(m) if m.is_empty() => m,
        Some(m) if !m.contains('@') || m.contains(char::is_whitespace) => {
            return Err(format!("{m} is not an e-mail address"));
        }
        Some(m) => match people.iter().find(|p| p.mail == m && p.id != id) {
            Some(other) => return Err(format!("{m} is {}'s already", other.id)),
            None => m,
        },
        None => people
            .iter()
            .find(|p| p.id == id)
            .map(|p| p.mail.clone())
            .unwrap_or_default(),
    };
    let person = Person {
        id: id.to_string(),
        role: if owner { "owner" } else { "writer" }.to_string(),
        name: if name.trim().is_empty() {
            id
        } else {
            name.trim()
        }
        .to_string(),
        mail,
    };
    match people.iter_mut().find(|p| p.id == id) {
        Some(p) => *p = person,
        None => people.push(person),
    }
    keep(home, &people).map_err(|e| e.to_string())
}

/// Off the list, with the links not used yet. What they wrote stays in
/// their folder.
pub fn remove(home: &Path, id: &str) -> Result<(), String> {
    let mut people = list(home);
    let before = people.len();
    people.retain(|p| p.id != id);
    if people.len() == before {
        return Err(format!("{id} is not on the list"));
    }
    keep(home, &people).map_err(|e| e.to_string())?;
    let left: Vec<_> = invites(home).into_iter().filter(|i| i.2 != id).collect();
    keep_invites(home, &left).map_err(|e| e.to_string())
}

/// The links not used and not ended: token, the second it ends, whose.
fn invites(home: &Path) -> Vec<(String, u64, String)> {
    let now = now();
    std::fs::read_to_string(invites_file(home))
        .unwrap_or_default()
        .lines()
        .filter_map(|l| {
            let mut parts = l.split(' ');
            let token = parts.next()?;
            let ends: u64 = parts.next()?.parse().ok()?;
            let id = parts.next()?;
            (ends > now).then(|| (token.to_string(), ends, id.to_string()))
        })
        .collect()
}

fn keep_invites(home: &Path, invites: &[(String, u64, String)]) -> std::io::Result<()> {
    let text: String = invites
        .iter()
        .map(|(token, ends, id)| format!("{token} {ends} {id}\n"))
        .collect();
    write(&invites_file(home), &text, true)
}

/// A link's token for `id`, good once within `days`.
pub fn invite(home: &Path, id: &str, days: u64) -> Result<String, String> {
    if find(home, id).is_none() {
        return Err(format!("{id} is not on the list: add them first"));
    }
    let token = random().map_err(|e| e.to_string())?;
    let mut all = invites(home);
    all.push((token.clone(), now() + days * 24 * 3600, id.to_string()));
    keep_invites(home, &all).map_err(|e| e.to_string())?;
    Ok(token)
}

/// Whether `token` is a link that would still let someone in. Looking
/// doesn't use it up: a chat program's preview of the link looks too.
pub fn invited(home: &Path, token: &str) -> bool {
    invites(home).iter().any(|i| same(&i.0, token))
}

/// Uses the link up: who it lets in, if they are still on the list.
pub fn redeem(home: &Path, token: &str) -> Option<Person> {
    let all = invites(home);
    let hit = all.iter().position(|i| same(&i.0, token))?;
    let id = all[hit].2.clone();
    let left: Vec<_> = all
        .into_iter()
        .enumerate()
        .filter(|(i, _)| *i != hit)
        .map(|(_, i)| i)
        .collect();
    keep_invites(home, &left).ok()?;
    find(home, &id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn home(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("dactile-people-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn ids_are_folder_names_and_nothing_else() {
        for id in ["ada", "baran-2", "7kat"] {
            assert!(valid_id(id), "{id}");
        }
        for id in [
            "",
            "Ada",
            "a b",
            "../ada",
            "-ada",
            "a/b",
            "ç",
            "a\tb",
            &"a".repeat(33),
        ] {
            assert!(!valid_id(id), "{id}");
        }
    }

    #[test]
    fn people_are_added_renamed_and_removed() {
        let h = home("list");
        assert!(list(&h).is_empty());
        add(&h, "ada", "Ada Yıldız", true, Some("Ada@Example.com")).unwrap();
        add(&h, "baran", "", false, None).unwrap();
        assert!(add(&h, "../x", "X", false, None).is_err());
        // An address is one person's, in small letters; a new name keeps it.
        assert_eq!(find_by_mail(&h, "ADA@example.com").unwrap().id, "ada");
        assert!(add(&h, "baran", "", false, Some("ada@example.com")).is_err());
        assert!(add(&h, "baran", "", false, Some("not an address")).is_err());
        add(&h, "ada", "Ada Yıldız", true, None).unwrap();
        assert_eq!(find(&h, "ada").unwrap().mail, "ada@example.com");
        // No address is no one's.
        assert!(find_by_mail(&h, "").is_none());
        assert_eq!(find(&h, "ada").unwrap().role, "owner");
        assert_eq!(find(&h, "baran").unwrap().name, "baran");
        // Again: a new name, not a second line. A tab can't break the file.
        add(&h, "baran", "Baran\tDeniz", false, None).unwrap();
        assert_eq!(list(&h).len(), 2);
        assert_eq!(find(&h, "baran").unwrap().name, "Baran Deniz");
        remove(&h, "ada").unwrap();
        assert!(find(&h, "ada").is_none());
        assert!(remove(&h, "ada").is_err());
        std::fs::remove_dir_all(h).unwrap();
    }

    #[test]
    fn a_link_lets_its_person_in_once() {
        let h = home("links");
        assert!(invite(&h, "ada", 7).is_err());
        add(&h, "ada", "Ada", false, None).unwrap();
        add(&h, "baran", "Baran", false, None).unwrap();
        let a = invite(&h, "ada", 7).unwrap();
        let b = invite(&h, "baran", 7).unwrap();
        // Looked at, it is still good.
        assert!(invited(&h, &a) && invited(&h, &a));
        assert_eq!(redeem(&h, &a).unwrap().id, "ada");
        assert!(!invited(&h, &a));
        assert!(redeem(&h, &a).is_none());
        assert!(redeem(&h, "no-such-link").is_none());
        // Taken off the list, their link goes with them.
        remove(&h, "baran").unwrap();
        assert!(redeem(&h, &b).is_none());
        // Ended: not a link any more.
        std::fs::write(invites_file(&h), "old 5 ada\n").unwrap();
        assert!(!invited(&h, "old"));
        // For the owner's eyes alone.
        let again = invite(&h, "ada", 1).unwrap();
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(invites_file(&h))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600);
        assert!(invited(&h, &again));
        std::fs::remove_dir_all(h).unwrap();
    }
}
