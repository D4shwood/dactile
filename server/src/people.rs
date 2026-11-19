//! Who may come through the gate: their accounts, their passwords, and the
//! one-time codes that bring them in.
//!
//! All are small files under the gate's folder, read afresh each time, so
//! a change counts at once, whether the site made it or the owner's
//! commands (`dactile-gate add`, `invite`, `reset`, `remove`) did.
//!
//! - `gate/people.json`: the accounts. Backed up.
//! - `gate/secret/passwords`: a line each, `id<TAB>argon2 hash`. Not backed
//!   up: a site put back from a backup asks for new passwords.
//! - `gate/secret/codes.json`: the one-time codes: an invitation to make
//!   an account, a way to set a new password, an address to check. Each
//!   works once, and only until it ends. Not backed up.
//!
//! No one can make an account of their own: the owner invites them by
//! their e-mail address, and the invitation is the only way to one.

use crate::door::{now, random, same};
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Person {
    /// Their user name: their folder's name and their name to the gate,
    /// `ada`. It never changes.
    pub id: String,
    /// "owner", who keeps the list of people, or "member".
    pub role: String,
    /// As they are shown: "Ada Yıldız".
    pub name: String,
    /// Their e-mail address, in small letters: where their mail goes, what
    /// they may sign in with, and the Google account that lets them in.
    #[serde(default)]
    pub mail: String,
    /// Whether the address is known to be theirs: mail sent to it came back
    /// as a link they opened, or Google said so.
    #[serde(default)]
    pub checked: bool,
}

impl Person {
    pub fn owner(&self) -> bool {
        self.role == "owner"
    }
}

/// What a one-time code is for.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Errand {
    /// An account to be made, by whoever has the mail sent to `mail`.
    Davet {
        mail: String,
        /// The name the owner gave them, offered as theirs.
        #[serde(default)]
        name: String,
        /// Who invited them, by name.
        #[serde(default)]
        by: String,
    },
    /// A new password for `id`, and in with it.
    Parola { id: String },
    /// `mail` is `id`'s: they opened what was sent to it.
    Dogrula { id: String, mail: String },
    /// A way in as `id` that lasts and works again and again: the trial
    /// site's alone (gate.rs), so as not to type a password there.
    Anahtar { id: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Code {
    code: String,
    ends: u64,
    #[serde(flatten)]
    errand: Errand,
}

fn people_file(home: &Path) -> PathBuf {
    home.join("gate/people.json")
}

/// The list as it was kept before accounts: `id<TAB>role<TAB>name<TAB>mail`.
fn old_people_file(home: &Path) -> PathBuf {
    home.join("gate/people")
}

fn passwords_file(home: &Path) -> PathBuf {
    home.join("gate/secret/passwords")
}

fn codes_file(home: &Path) -> PathBuf {
    home.join("gate/secret/codes.json")
}

/// The sign-in links made before accounts: `token ends id`.
fn old_invites_file(home: &Path) -> PathBuf {
    home.join("gate/secret/invites")
}

/// Where a person's Dactile keeps everything of theirs.
pub fn folder(home: &Path, id: &str) -> PathBuf {
    home.join("people").join(id)
}

/// A user name, which is also a folder's name: small letters, digits and
/// hyphens, a letter or digit first, 32 at the most.
pub fn valid_id(id: &str) -> bool {
    let ok = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit();
    !id.is_empty() && id.len() <= 32 && id.starts_with(ok) && id.chars().all(|c| ok(c) || c == '-')
}

/// An address as it is kept: trimmed, in small letters; an error if it
/// isn't one.
pub fn address(mail: &str) -> Result<String, String> {
    let m = mail.trim().to_lowercase();
    let parts = m.split_once('@');
    let fine = parts.is_some_and(|(who, at)| {
        !who.is_empty() && at.contains('.') && !at.starts_with('.') && !at.ends_with('.')
    });
    if !fine || m.len() > 254 || m.contains(|c: char| c.is_whitespace() || c.is_control()) {
        return Err(lang(
            &format!("{} bir e-posta adresi değil.", mail.trim()),
            &format!("{} is not an e-mail address.", mail.trim()),
        ));
    }
    Ok(m)
}

fn lang(tr: &str, en: &str) -> String {
    dactile::lang::f(tr, en, &[])
}

/// A name as shown: one line, not too long.
fn clean_name(name: &str) -> String {
    let one: String = name
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    one.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(80)
        .collect()
}

/// Whole, to a file of its own, then moved over; `private` for the owner's
/// eyes alone.
pub(crate) fn write(file: &Path, text: &str, private: bool) -> std::io::Result<()> {
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
    if let Ok(text) = std::fs::read_to_string(people_file(home)) {
        let people: Vec<Person> = serde_json::from_str(&text).unwrap_or_default();
        return people.into_iter().filter(|p| valid_id(&p.id)).collect();
    }
    // Kept the old way: read so, and written the new way next time. An
    // address on the old list was a Google account's, which Google checks.
    std::fs::read_to_string(old_people_file(home))
        .unwrap_or_default()
        .lines()
        .filter_map(|l| {
            let mut parts = l.split('\t');
            let id = parts.next().filter(|id| valid_id(id))?;
            let role = parts.next()?;
            let name = parts.next().unwrap_or(id).trim();
            let mail = parts.next().unwrap_or("").trim().to_lowercase();
            Some(Person {
                id: id.to_string(),
                role: if role == "owner" { "owner" } else { "member" }.to_string(),
                name: if name.is_empty() { id } else { name }.to_string(),
                checked: !mail.is_empty(),
                mail,
            })
        })
        .collect()
}

pub fn find(home: &Path, id: &str) -> Option<Person> {
    list(home).into_iter().find(|p| p.id == id)
}

/// Whose address `mail` is.
pub fn find_by_mail(home: &Path, mail: &str) -> Option<Person> {
    let mail = mail.trim().to_lowercase();
    list(home)
        .into_iter()
        .find(|p| !p.mail.is_empty() && p.mail == mail)
}

/// Who signs in as `who`: a user name, or an address.
pub fn find_by_either(home: &Path, who: &str) -> Option<Person> {
    let who = who.trim();
    if who.contains('@') {
        find_by_mail(home, who)
    } else {
        find(home, &who.to_lowercase())
    }
}

fn keep(home: &Path, people: &[Person]) -> Result<(), String> {
    let text = serde_json::to_string_pretty(people).map_err(|e| e.to_string())?;
    write(&people_file(home), &text, false).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(old_people_file(home));
    Ok(())
}

/// The address no one else has: `Err` if someone does.
fn free_address(people: &[Person], mail: &str, id: &str) -> Result<(), String> {
    match people.iter().find(|p| p.mail == mail && p.id != id) {
        Some(_) => Err(lang(
            "Bu adresle bir hesap zaten var.",
            "There is an account with this address already.",
        )),
        None => Ok(()),
    }
}

/// A new person, or a new name or role for one who is there: the owner's
/// way, with no invitation. `mail`, if given, is their address, taken as
/// checked ("" takes it away); not given, the one they had stays.
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
    let mut people = list(home);
    let had = people.iter().find(|p| p.id == id).cloned();
    let (mail, checked) = match mail.map(str::trim) {
        Some("") => (String::new(), false),
        Some(m) => {
            let m = address(m)?;
            free_address(&people, &m, id)?;
            (m, true)
        }
        None => had
            .as_ref()
            .map(|p| (p.mail.clone(), p.checked))
            .unwrap_or_default(),
    };
    let name = clean_name(name);
    let person = Person {
        id: id.to_string(),
        role: if owner { "owner" } else { "member" }.to_string(),
        name: if name.is_empty() {
            id.to_string()
        } else {
            name
        },
        mail,
        checked,
    };
    match people.iter_mut().find(|p| p.id == id) {
        Some(p) => *p = person,
        None => people.push(person),
    }
    keep(home, &people)
}

/// A new name for `id`, as they choose it.
pub fn rename(home: &Path, id: &str, name: &str) -> Result<Person, String> {
    let name = clean_name(name);
    if name.is_empty() {
        return Err(lang("Ad boş olamaz.", "The name can't be empty."));
    }
    let mut people = list(home);
    let p = people
        .iter_mut()
        .find(|p| p.id == id)
        .ok_or("no such person")?;
    p.name = name;
    let p = p.clone();
    keep(home, &people)?;
    Ok(p)
}

/// Off the list, with their password and their codes. What they wrote stays
/// in their folder.
pub fn remove(home: &Path, id: &str) -> Result<(), String> {
    let mut people = list(home);
    let before = people.len();
    people.retain(|p| p.id != id);
    if people.len() == before {
        return Err(format!("{id} is not on the list"));
    }
    keep(home, &people)?;
    forget_password(home, id)?;
    let left: Vec<Code> = codes(home)
        .into_iter()
        .filter(|c| match &c.errand {
            Errand::Parola { id: who }
            | Errand::Dogrula { id: who, .. }
            | Errand::Anahtar { id: who } => who != id,
            Errand::Davet { .. } => true,
        })
        .collect();
    keep_codes(home, &left)
}

// ------------------------------------------------------------- passwords

/// Long enough to be hard to guess, short enough to hash quickly.
pub fn fit_password(password: &str) -> Result<(), String> {
    let n = password.chars().count();
    if n < 10 {
        return Err(lang(
            "Parola en az 10 karakter olmalı.",
            "A password needs 10 characters at least.",
        ));
    }
    if n > 200 {
        return Err(lang("Parola çok uzun.", "The password is too long."));
    }
    Ok(())
}

fn hashes(home: &Path) -> Vec<(String, String)> {
    std::fs::read_to_string(passwords_file(home))
        .unwrap_or_default()
        .lines()
        .filter_map(|l| l.split_once('\t'))
        .map(|(id, h)| (id.to_string(), h.to_string()))
        .collect()
}

fn keep_hashes(home: &Path, all: &[(String, String)]) -> Result<(), String> {
    let text: String = all.iter().map(|(id, h)| format!("{id}\t{h}\n")).collect();
    write(&passwords_file(home), &text, true).map_err(|e| e.to_string())
}

pub fn has_password(home: &Path, id: &str) -> bool {
    hashes(home).iter().any(|(who, _)| who == id)
}

pub fn set_password(home: &Path, id: &str, password: &str) -> Result<(), String> {
    fit_password(password)?;
    let salt = random().map_err(|e| e.to_string())?;
    let salt = SaltString::encode_b64(&salt.as_bytes()[..16]).map_err(|e| e.to_string())?;
    let hash = argon2::Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| e.to_string())?
        .to_string();
    let mut all = hashes(home);
    all.retain(|(who, _)| who != id);
    all.push((id.to_string(), hash));
    keep_hashes(home, &all)
}

fn forget_password(home: &Path, id: &str) -> Result<(), String> {
    let mut all = hashes(home);
    let before = all.len();
    all.retain(|(who, _)| who != id);
    if all.len() == before {
        return Ok(());
    }
    keep_hashes(home, &all)
}

/// Whose account `who` (a user name or an address) is, if `password` is
/// theirs. Takes as long for a name no one has: how long it takes says
/// nothing of who is here.
pub fn check_password(home: &Path, who: &str, password: &str) -> Option<Person> {
    // A hash of nothing anyone could type, for the names no one has.
    const NO_ONE: &str = "$argon2id$v=19$m=19456,t=2,p=1$bm8tb25lLWhlcmUtYXQtYWxs$yd3E3S7JpdhkP0XIdjwtw5Hy1n1t2LVKmGsZVo1ZZEw";
    let person = find_by_either(home, who);
    let hash = person
        .as_ref()
        .and_then(|p| hashes(home).into_iter().find(|(id, _)| *id == p.id))
        .map(|(_, h)| h);
    let stored = hash.as_deref().unwrap_or(NO_ONE);
    let right = PasswordHash::new(stored).is_ok_and(|h| {
        argon2::Argon2::default()
            .verify_password(password.as_bytes(), &h)
            .is_ok()
    });
    (right && hash.is_some()).then_some(person).flatten()
}

// ----------------------------------------------------------------- codes

/// The codes not used and not ended. Links made before accounts count as
/// codes for a new password.
fn codes(home: &Path) -> Vec<Code> {
    let now = now();
    let mut all: Vec<Code> = std::fs::read_to_string(codes_file(home))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default();
    let old = std::fs::read_to_string(old_invites_file(home)).unwrap_or_default();
    all.extend(old.lines().filter_map(|l| {
        let mut parts = l.split(' ');
        let code = parts.next()?.to_string();
        let ends = parts.next()?.parse().ok()?;
        let id = parts.next()?.to_string();
        Some(Code {
            code,
            ends,
            errand: Errand::Parola { id },
        })
    }));
    all.retain(|c| c.ends > now);
    all
}

fn keep_codes(home: &Path, all: &[Code]) -> Result<(), String> {
    let text = serde_json::to_string_pretty(all).map_err(|e| e.to_string())?;
    write(&codes_file(home), &text, true).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(old_invites_file(home));
    Ok(())
}

/// A code for `errand`, good once for `secs`. A code for the same thing
/// made before goes: only the newest works.
fn make(home: &Path, errand: Errand, secs: u64) -> Result<String, String> {
    let code = random().map_err(|e| e.to_string())?;
    let mut all = codes(home);
    all.retain(|c| c.errand != errand);
    all.push(Code {
        code: code.clone(),
        ends: now() + secs,
        errand,
    });
    keep_codes(home, &all)?;
    Ok(code)
}

/// What `code` is for, if it still works. Looking doesn't use it up: a chat
/// program's preview of the link looks too.
pub fn peek(home: &Path, code: &str) -> Option<Errand> {
    codes(home)
        .into_iter()
        .find(|c| same(&c.code, code))
        .map(|c| c.errand)
}

/// Uses `code` up, if it works and `fits` its errand.
fn take(home: &Path, code: &str, fits: impl Fn(&Errand) -> bool) -> Option<Errand> {
    let mut all = codes(home);
    let at = all
        .iter()
        .position(|c| same(&c.code, code) && fits(&c.errand))?;
    let c = all.remove(at);
    keep_codes(home, &all).ok()?;
    Some(c.errand)
}

pub const DAYS: u64 = 24 * 3600;
/// A code for a new password, asked for by mail.
pub const HOUR: u64 = 3600;

/// An invitation to make an account, for whoever reads `mail`'s mail.
pub fn invite(home: &Path, mail: &str, name: &str, by: &str) -> Result<String, String> {
    let mail = address(mail)?;
    free_address(&list(home), &mail, "")?;
    let errand = Errand::Davet {
        mail,
        name: clean_name(name),
        by: clean_name(by),
    };
    make(home, errand, 7 * DAYS)
}

/// The invitations not taken up yet: code, address, name, the second they
/// end. For the owner's list.
pub fn invitations(home: &Path) -> Vec<(String, String, String, u64)> {
    codes(home)
        .into_iter()
        .filter_map(|c| match c.errand {
            Errand::Davet { mail, name, .. } => Some((c.code, mail, name, c.ends)),
            _ => None,
        })
        .collect()
}

/// Takes back the invitation to `mail`.
pub fn uninvite(home: &Path, mail: &str) -> Result<(), String> {
    let mail = mail.trim().to_lowercase();
    let mut all = codes(home);
    all.retain(|c| !matches!(&c.errand, Errand::Davet { mail: m, .. } if *m == mail));
    keep_codes(home, &all)
}

/// A way for `id` to set a new password, good for `secs`.
pub fn reset(home: &Path, id: &str, secs: u64) -> Result<String, String> {
    if find(home, id).is_none() {
        return Err(format!("{id} is not on the list: add them first"));
    }
    make(home, Errand::Parola { id: id.to_string() }, secs)
}

/// A lasting way in as `id`, for the trial site; the one made before stops.
pub fn key(home: &Path, id: &str) -> Result<String, String> {
    if find(home, id).is_none() {
        return Err(format!("{id} is not on the list: add them first"));
    }
    make(
        home,
        Errand::Anahtar { id: id.to_string() },
        100 * 365 * DAYS,
    )
}

/// Who `code` lets in, if it is a lasting way in. Using it leaves it be.
pub fn by_key(home: &Path, code: &str) -> Option<Person> {
    match peek(home, code)? {
        Errand::Anahtar { id } => find(home, &id),
        _ => None,
    }
}

/// The new account an invitation was for.
pub struct Account<'a> {
    pub id: &'a str,
    pub name: &'a str,
    /// Their address: the invitation's, or another they chose.
    pub mail: &'a str,
    /// None where they sign in with Google alone.
    pub password: Option<&'a str>,
    /// The address Google says is theirs, where they came by Google.
    pub google: Option<&'a str>,
}

/// Makes the account `code` invited, and uses the code up. The address the
/// invitation went to is theirs; another they choose, or Google's, is
/// theirs once checked (Google's is). `Err` says what to change.
pub fn accept(home: &Path, code: &str, a: Account) -> Result<Person, String> {
    let Some(Errand::Davet { mail: invited, .. }) = peek(home, code) else {
        return Err(lang(
            "Bu davet artık geçerli değil.",
            "This invitation is no longer good.",
        ));
    };
    if !valid_id(a.id) {
        return Err(lang(
            "Kullanıcı adı küçük harf, rakam ve tireden oluşur; en çok 32 karakter.",
            "A user name is small letters, digits and hyphens, 32 at the most.",
        ));
    }
    let people = list(home);
    if people.iter().any(|p| p.id == a.id) || folder(home, a.id).exists() {
        return Err(lang(
            "Bu kullanıcı adı alınmış; başka birini seç.",
            "This user name is taken; choose another.",
        ));
    }
    let (mail, checked) = match a.google {
        Some(g) => (address(g)?, true),
        None => {
            let m = address(a.mail)?;
            let checked = m == invited;
            (m, checked)
        }
    };
    free_address(&people, &mail, a.id)?;
    match a.password {
        Some(p) => fit_password(p)?,
        None if a.google.is_none() => {
            return Err(lang("Bir parola seç.", "Choose a password."));
        }
        None => {}
    }
    take(home, code, |e| matches!(e, Errand::Davet { .. })).ok_or_else(|| {
        lang(
            "Bu davet artık geçerli değil.",
            "This invitation is no longer good.",
        )
    })?;
    let name = clean_name(a.name);
    let person = Person {
        id: a.id.to_string(),
        role: "member".to_string(),
        name: if name.is_empty() {
            a.id.to_string()
        } else {
            name
        },
        mail,
        checked,
    };
    let mut people = people;
    people.push(person.clone());
    keep(home, &people)?;
    if let Some(p) = a.password {
        set_password(home, a.id, p)?;
    }
    Ok(person)
}

/// Uses a code for a new password: whose it was.
pub fn new_password(home: &Path, code: &str, password: &str) -> Result<Person, String> {
    fit_password(password)?;
    let no = || {
        lang(
            "Bu bağlantı artık geçerli değil.",
            "This link is no longer good.",
        )
    };
    let Some(Errand::Parola { id }) = peek(home, code) else {
        return Err(no());
    };
    let person = find(home, &id).ok_or_else(no)?;
    take(home, code, |e| matches!(e, Errand::Parola { .. })).ok_or_else(no)?;
    set_password(home, &id, password)?;
    Ok(person)
}

/// A code that checks `mail` is `id`'s, to be sent to it. Their account
/// takes it once they open what was sent.
pub fn check_address(home: &Path, id: &str, mail: &str) -> Result<String, String> {
    let mail = address(mail)?;
    free_address(&list(home), &mail, id)?;
    make(
        home,
        Errand::Dogrula {
            id: id.to_string(),
            mail,
        },
        2 * DAYS,
    )
}

/// Uses an address's code: the account, now with that address, checked.
pub fn checked(home: &Path, code: &str) -> Result<Person, String> {
    let no = || {
        lang(
            "Bu bağlantı artık geçerli değil.",
            "This link is no longer good.",
        )
    };
    let Some(Errand::Dogrula { id, mail }) =
        take(home, code, |e| matches!(e, Errand::Dogrula { .. }))
    else {
        return Err(no());
    };
    let mut people = list(home);
    free_address(&people, &mail, &id)?;
    let p = people.iter_mut().find(|p| p.id == id).ok_or_else(no)?;
    p.mail = mail;
    p.checked = true;
    let p = p.clone();
    keep(home, &people)?;
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn home(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("dactile-people-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    fn mode(f: &Path) -> u32 {
        use std::os::unix::fs::PermissionsExt;
        std::fs::metadata(f).unwrap().permissions().mode() & 0o777
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
    fn addresses_are_kept_small_and_must_be_addresses() {
        assert_eq!(address(" Ada@Example.COM ").unwrap(), "ada@example.com");
        for no in [
            "",
            "ada",
            "ada@",
            "@example.com",
            "ada@example",
            "a da@example.com",
            "ada@.com",
        ] {
            assert!(address(no).is_err(), "{no}");
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
        assert_eq!(find_by_either(&h, "Ada@example.com").unwrap().id, "ada");
        assert_eq!(find_by_either(&h, "ADA").unwrap().id, "ada");
        assert!(add(&h, "baran", "", false, Some("ada@example.com")).is_err());
        assert!(add(&h, "baran", "", false, Some("not an address")).is_err());
        add(&h, "ada", "Ada Yıldız", true, None).unwrap();
        assert_eq!(find(&h, "ada").unwrap().mail, "ada@example.com");
        assert!(find(&h, "ada").unwrap().checked);
        assert!(find_by_mail(&h, "").is_none());
        assert!(find(&h, "ada").unwrap().owner());
        assert_eq!(find(&h, "baran").unwrap().name, "baran");
        // Again: a new name, not a second account. A tab can't break it.
        add(&h, "baran", "Baran\tDeniz", false, None).unwrap();
        assert_eq!(list(&h).len(), 2);
        assert_eq!(find(&h, "baran").unwrap().name, "Baran Deniz");
        assert_eq!(
            rename(&h, "baran", "  Baran  D. ").unwrap().name,
            "Baran D."
        );
        assert!(rename(&h, "baran", " ").is_err());
        set_password(&h, "ada", "uzun bir parola").unwrap();
        remove(&h, "ada").unwrap();
        assert!(find(&h, "ada").is_none());
        assert!(!has_password(&h, "ada"));
        assert!(remove(&h, "ada").is_err());
        std::fs::remove_dir_all(h).unwrap();
    }

    #[test]
    fn the_list_kept_the_old_way_is_read_and_moved_on() {
        let h = home("old");
        std::fs::create_dir_all(h.join("gate/secret")).unwrap();
        std::fs::write(
            h.join("gate/people"),
            "ada\towner\tAda Yıldız\tada@example.com\nbaran\twriter\tBaran\t\n",
        )
        .unwrap();
        let later = now() + 3600;
        std::fs::write(
            h.join("gate/secret/invites"),
            format!("eski {later} baran\n"),
        )
        .unwrap();
        let ada = find(&h, "ada").unwrap();
        assert!(ada.owner() && ada.checked);
        assert_eq!(find(&h, "baran").unwrap().role, "member");
        // An old sign-in link sets a password now.
        assert_eq!(
            peek(&h, "eski"),
            Some(Errand::Parola { id: "baran".into() })
        );
        rename(&h, "baran", "Baran Deniz").unwrap();
        assert!(!h.join("gate/people").exists());
        assert_eq!(list(&h).len(), 2);
        new_password(&h, "eski", "baranin parolasi").unwrap();
        assert!(!h.join("gate/secret/invites").exists());
        assert!(check_password(&h, "baran", "baranin parolasi").is_some());
        std::fs::remove_dir_all(h).unwrap();
    }

    #[test]
    fn a_password_lets_its_person_in_and_no_one_else() {
        let h = home("passwords");
        add(&h, "ada", "Ada", true, Some("ada@example.com")).unwrap();
        add(&h, "baran", "Baran", false, None).unwrap();
        assert!(set_password(&h, "ada", "kisa").is_err());
        set_password(&h, "ada", "ada'nın parolası").unwrap();
        assert_eq!(mode(&passwords_file(&h)), 0o600);
        // Kept hashed: the file never has the password.
        let file = std::fs::read_to_string(passwords_file(&h)).unwrap();
        assert!(!file.contains("parolası") && file.contains("$argon2id$"));
        assert_eq!(
            check_password(&h, "ada", "ada'nın parolası").unwrap().id,
            "ada"
        );
        assert!(check_password(&h, "ADA@example.com", "ada'nın parolası").is_some());
        assert!(check_password(&h, "ada", "ada'nın parolasi").is_none());
        // No password, no way in by one; no name, no way in at all.
        assert!(check_password(&h, "baran", "").is_none());
        assert!(check_password(&h, "kimse", "ada'nın parolası").is_none());
        set_password(&h, "ada", "yepyeni bir parola").unwrap();
        assert!(check_password(&h, "ada", "ada'nın parolası").is_none());
        assert!(check_password(&h, "ada", "yepyeni bir parola").is_some());
        std::fs::remove_dir_all(h).unwrap();
    }

    #[test]
    fn an_invitation_makes_one_account_once() {
        let h = home("invite");
        add(&h, "ada", "Ada", true, Some("ada@example.com")).unwrap();
        // Not to someone who has an account.
        assert!(invite(&h, "ADA@example.com", "", "Ada").is_err());
        let code = invite(&h, "Baran@Example.com", "Baran Deniz", "Ada").unwrap();
        assert_eq!(mode(&codes_file(&h)), 0o600);
        assert!(
            matches!(peek(&h, &code), Some(Errand::Davet { mail, .. }) if mail == "baran@example.com")
        );
        assert_eq!(invitations(&h).len(), 1);
        let try_with = |id: &str, mail: &str, password: Option<&str>| {
            accept(
                &h,
                &code,
                Account {
                    id,
                    name: "Baran",
                    mail,
                    password,
                    google: None,
                },
            )
        };
        // What isn't right is said, and the invitation stays.
        assert!(try_with("ada", "baran@example.com", Some("uzun bir parola")).is_err());
        assert!(try_with("Baran!", "baran@example.com", Some("uzun bir parola")).is_err());
        assert!(try_with("baran", "baran@example.com", Some("kisa")).is_err());
        assert!(try_with("baran", "baran@example.com", None).is_err());
        assert!(try_with("baran", "ada@example.com", Some("uzun bir parola")).is_err());
        let baran = try_with("baran", "baran@example.com", Some("uzun bir parola")).unwrap();
        // The address the invitation went to is theirs.
        assert!(baran.checked && !baran.owner());
        assert!(check_password(&h, "baran", "uzun bir parola").is_some());
        // Once.
        assert!(try_with("baran2", "baran@example.com", Some("uzun bir parola")).is_err());
        assert!(invitations(&h).is_empty());

        // Another address than the invitation's waits to be checked.
        let code = invite(&h, "cem@example.com", "", "Ada").unwrap();
        let cem = accept(
            &h,
            &code,
            Account {
                id: "cem",
                name: "",
                mail: "cem@baska.example",
                password: Some("cemin parolasi"),
                google: None,
            },
        )
        .unwrap();
        assert!(!cem.checked);
        assert_eq!(cem.name, "cem");
        let check = check_address(&h, "cem", "cem@baska.example").unwrap();
        assert!(checked(&h, &check).unwrap().checked);
        assert!(checked(&h, &check).is_err());

        // With Google: Google's address, checked, and no password.
        let code = invite(&h, "deniz@example.com", "", "Ada").unwrap();
        let deniz = accept(
            &h,
            &code,
            Account {
                id: "deniz",
                name: "Deniz",
                mail: "",
                password: None,
                google: Some("Deniz@Gmail.com"),
            },
        )
        .unwrap();
        assert_eq!(
            (deniz.mail.as_str(), deniz.checked),
            ("deniz@gmail.com", true)
        );
        assert!(!has_password(&h, "deniz"));

        // Taken back before it is used.
        invite(&h, "ece@example.com", "", "Ada").unwrap();
        uninvite(&h, "ECE@example.com").unwrap();
        assert!(invitations(&h).is_empty());
        std::fs::remove_dir_all(h).unwrap();
    }

    #[test]
    fn a_new_password_by_code_works_once_and_only_the_newest() {
        let h = home("reset");
        add(&h, "ada", "Ada", true, None).unwrap();
        assert!(reset(&h, "kimse", HOUR).is_err());
        let first = reset(&h, "ada", HOUR).unwrap();
        let second = reset(&h, "ada", HOUR).unwrap();
        assert!(new_password(&h, &first, "yeni parola bu").is_err());
        assert!(new_password(&h, &second, "kisa").is_err());
        assert_eq!(
            new_password(&h, &second, "yeni parola bu").unwrap().id,
            "ada"
        );
        assert!(new_password(&h, &second, "yeni parola bu").is_err());
        // A code for something else isn't one for a password.
        let invitation = invite(&h, "baran@example.com", "", "Ada").unwrap();
        assert!(new_password(&h, &invitation, "yeni parola bu").is_err());
        assert!(peek(&h, &invitation).is_some());
        // Ended: not a code any more.
        std::fs::write(
            codes_file(&h),
            r#"[{"code":"eski","ends":5,"kind":"parola","id":"ada"}]"#,
        )
        .unwrap();
        assert!(peek(&h, "eski").is_none());
        std::fs::remove_dir_all(h).unwrap();
    }
}
