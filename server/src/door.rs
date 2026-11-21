//! Who is signed in, and who has guessed too often.
//!
//! A session is a random token in the browser's cookie. The tokens are kept
//! in a file of the person's cache folder (which no backup takes), so
//! putting a new server up signs nobody out; each lasts as long as its
//! cookie.

use std::collections::HashMap;
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// A session's life, and its cookie's.
pub const SESSION: Duration = Duration::from_secs(30 * 24 * 3600);
/// A linked computer's: it asks again in a year, or when taken off.
pub const DEVICE: Duration = Duration::from_secs(365 * 24 * 3600);
/// Wrong passwords from one address before it has to wait.
const GUESSES: u32 = 5;
const WAIT: Duration = Duration::from_secs(15 * 60);
/// Addresses remembered at once: a flood of them forgets the oldest waits
/// rather than filling the memory.
const ADDRESSES: usize = 10_000;

/// 64 hex digits no one can guess.
pub fn random() -> std::io::Result<String> {
    use std::io::Read;
    let mut b = [0u8; 32];
    std::fs::File::open("/dev/urandom")?.read_exact(&mut b)?;
    Ok(b.iter().map(|x| format!("{x:02x}")).collect())
}

/// Whether two secrets are the same, in a time that says nothing of where
/// they differ.
pub fn same(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0, |d, (x, y)| d | (x ^ y)) == 0
}

pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// A session: the second it ends, whose it is (no one's name where one
/// password is the only way in), and the computer's name for a linked
/// desktop (empty for a browser).
#[derive(Clone)]
struct Session {
    ends: u64,
    who: String,
    device: String,
}

/// A linked computer, as its person sees it in their list.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Device {
    /// Enough of the token to take it off by; not enough to sign in with.
    pub id: String,
    pub name: String,
    pub ends: u64,
}

/// How much of a token names a device.
const SHORT: usize = 12;

pub struct Door {
    file: PathBuf,
    sessions: Mutex<HashMap<String, Session>>,
    /// Each address's wrong guesses, and the second they are forgotten.
    guesses: Mutex<HashMap<String, (u32, u64)>>,
}

impl Door {
    /// The sessions `file` has, without those that have ended.
    pub fn open(file: PathBuf) -> Door {
        let now = now();
        let sessions = std::fs::read_to_string(&file)
            .unwrap_or_default()
            .lines()
            .filter_map(|l| {
                let mut parts = l.splitn(4, ' ');
                let token = parts.next()?;
                let ends: u64 = parts.next()?.parse().ok()?;
                let who = parts.next().unwrap_or("").to_string();
                let device = parts.next().unwrap_or("").to_string();
                (ends > now).then(|| (token.to_string(), Session { ends, who, device }))
            })
            .collect();
        Door {
            file,
            sessions: Mutex::new(sessions),
            guesses: Mutex::new(HashMap::new()),
        }
    }

    fn sessions(&self) -> std::sync::MutexGuard<'_, HashMap<String, Session>> {
        self.sessions.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn guesses(&self) -> std::sync::MutexGuard<'_, HashMap<String, (u32, u64)>> {
        self.guesses.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Written whole to a file of its own and moved over, readable by the
    /// person alone. A write that fails only means signing in again later.
    fn keep(&self, sessions: &HashMap<String, Session>) {
        let tmp = self.file.with_extension("new");
        let written = (|| {
            if let Some(d) = self.file.parent() {
                std::fs::create_dir_all(d)?;
            }
            let mut f = std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o600)
                .open(&tmp)?;
            for (token, s) in sessions {
                let (ends, who, device) = (s.ends, &s.who, &s.device);
                if device.is_empty() {
                    writeln!(f, "{token} {ends} {who}")?;
                } else {
                    writeln!(f, "{token} {ends} {who} {device}")?;
                }
            }
            f.sync_all()?;
            std::fs::rename(&tmp, &self.file)
        })();
        if let Err(e) = written {
            eprintln!("sessions not kept: {e}");
        }
    }

    /// A new session's token, for `who` (a name with no space or line
    /// break in it, or none).
    pub fn sign_in(&self, who: &str) -> std::io::Result<String> {
        self.start(who, "", SESSION)
    }

    /// A linked computer's token, for `who`; `name` is what the computer
    /// calls itself, kept to one line.
    pub fn link(&self, who: &str, name: &str) -> std::io::Result<String> {
        let name: String = name
            .chars()
            .map(|c| if c.is_control() { ' ' } else { c })
            .take(60)
            .collect();
        let name = name.trim();
        self.start(who, if name.is_empty() { "?" } else { name }, DEVICE)
    }

    fn start(&self, who: &str, device: &str, life: Duration) -> std::io::Result<String> {
        let token = random()?;
        let now = now();
        let mut sessions = self.sessions();
        sessions.retain(|_, s| s.ends > now);
        let s = Session {
            ends: now + life.as_secs(),
            who: who.to_string(),
            device: device.to_string(),
        };
        sessions.insert(token.clone(), s);
        self.keep(&sessions);
        Ok(token)
    }

    /// The computers linked to `who`'s account.
    pub fn devices(&self, who: &str) -> Vec<Device> {
        let now = now();
        let mut all: Vec<Device> = self
            .sessions()
            .iter()
            .filter(|(_, s)| s.who == who && !s.device.is_empty() && s.ends > now)
            .map(|(t, s)| Device {
                id: t[..SHORT.min(t.len())].to_string(),
                name: s.device.clone(),
                ends: s.ends,
            })
            .collect();
        all.sort_by_key(|d| std::cmp::Reverse(d.ends));
        all
    }

    /// Takes one of `who`'s computers off; false if there was no such one.
    pub fn unlink(&self, who: &str, id: &str) -> bool {
        if id.len() != SHORT {
            return false;
        }
        let mut sessions = self.sessions();
        let before = sessions.len();
        sessions.retain(|t, s| !(s.who == who && !s.device.is_empty() && t.starts_with(id)));
        let gone = sessions.len() != before;
        if gone {
            self.keep(&sessions);
        }
        gone
    }

    pub fn signed_in(&self, token: &str) -> bool {
        self.who(token).is_some()
    }

    /// Whose session `token` is, if it is one that hasn't ended.
    pub fn who(&self, token: &str) -> Option<String> {
        self.sessions()
            .get(token)
            .filter(|s| s.ends > now())
            .map(|s| s.who.clone())
    }

    /// Every session of `who` ends: they were taken off the list.
    pub fn sign_out_all(&self, who: &str) {
        let mut sessions = self.sessions();
        let before = sessions.len();
        sessions.retain(|_, s| s.who != who);
        if sessions.len() != before {
            self.keep(&sessions);
        }
    }

    pub fn sign_out(&self, token: &str) {
        let mut sessions = self.sessions();
        if sessions.remove(token).is_some() {
            self.keep(&sessions);
        }
    }

    /// Whether `address` has guessed wrong too often to try again yet.
    pub fn waiting(&self, address: &str) -> bool {
        self.guesses()
            .get(address)
            .is_some_and(|(n, until)| *n >= GUESSES && *until > now())
    }

    /// A wrong password from `address`.
    pub fn wrong(&self, address: &str) {
        let now = now();
        let mut guesses = self.guesses();
        guesses.retain(|_, (_, until)| *until > now);
        if guesses.len() >= ADDRESSES {
            guesses.clear();
        }
        let g = guesses.entry(address.to_string()).or_insert((0, 0));
        g.0 += 1;
        g.1 = now + WAIT.as_secs();
    }

    /// The right password from `address`: its wrong guesses are forgotten.
    pub fn right(&self, address: &str) {
        self.guesses().remove(address);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("dactile-door-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d.join("cache/sessions")
    }

    #[test]
    fn a_session_outlives_the_server_and_ends_when_signed_out() {
        let f = file("sessions");
        let door = Door::open(f.clone());
        let token = door.sign_in("ada").unwrap();
        assert!(door.signed_in(&token));
        assert_eq!(door.who(&token).as_deref(), Some("ada"));
        assert!(!door.signed_in("someone-else"));
        // A new server on the same folder.
        let again = Door::open(f.clone());
        assert_eq!(again.who(&token).as_deref(), Some("ada"));
        // Off the list: all of theirs end, and no one else's.
        let other = again.sign_in("baran").unwrap();
        again.sign_out_all("ada");
        assert!(!again.signed_in(&token) && again.signed_in(&other));
        let token = again.sign_in("ada").unwrap();
        again.sign_out(&token);
        assert!(!Door::open(f.clone()).signed_in(&token));
        // Readable by the person alone.
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&f).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    #[test]
    fn a_linked_computer_lasts_is_listed_and_can_be_taken_off() {
        let f = file("devices");
        let door = Door::open(f.clone());
        let browser = door.sign_in("ada").unwrap();
        let desk = door.link("ada", "Ada'nın\nmasaüstü").unwrap();
        door.link("baran", "Baran").unwrap();
        // A year, not a month; read back after a restart, name and all.
        let again = Door::open(f.clone());
        assert_eq!(again.who(&desk).as_deref(), Some("ada"));
        let mine = again.devices("ada");
        assert_eq!(mine.len(), 1);
        assert_eq!(mine[0].name, "Ada'nın masaüstü");
        assert!(mine[0].ends > now() + SESSION.as_secs());
        assert!(desk.starts_with(&mine[0].id) && mine[0].id.len() < desk.len());
        // Only one's own, by its short name; the browser stays.
        assert!(!again.unlink("baran", &mine[0].id));
        assert!(!again.unlink("ada", "ab"));
        assert!(again.unlink("ada", &mine[0].id));
        assert!(!again.signed_in(&desk) && again.signed_in(&browser));
        assert!(again.devices("ada").is_empty());
        // Off the list, the computers go with the browsers.
        let desk = again.link("ada", "yine").unwrap();
        again.sign_out_all("ada");
        assert!(!again.signed_in(&desk));
    }

    #[test]
    fn a_session_that_has_ended_is_not_read_back() {
        let f = file("ended");
        std::fs::create_dir_all(f.parent().unwrap()).unwrap();
        std::fs::write(&f, format!("old 5\nnew {}\nbroken\n", now() + 60)).unwrap();
        let door = Door::open(f);
        assert!(!door.signed_in("old"));
        assert!(door.signed_in("new"));
        assert!(!door.signed_in("broken"));
    }

    #[test]
    fn five_wrong_guesses_make_an_address_wait_and_no_other() {
        let door = Door::open(file("guesses"));
        for _ in 0..4 {
            door.wrong("203.0.113.7");
        }
        assert!(!door.waiting("203.0.113.7"));
        door.wrong("203.0.113.7");
        assert!(door.waiting("203.0.113.7"));
        assert!(!door.waiting("203.0.113.8"));
        // The right password in between starts the count again.
        door.wrong("203.0.113.9");
        door.right("203.0.113.9");
        assert!(!door.waiting("203.0.113.9"));
    }
}
