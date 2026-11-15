//! Mail the gate sends: an invitation, a link for a new password, an
//! address to check. A mail service's SMTP sends it; the system's `curl`
//! speaks to the service, as it speaks to Google (`google.rs`), with the
//! service's password on its stdin, never on a command line.
//!
//! With `DACTILE_MAIL_URL=file:///some/folder` nothing is sent: each mail
//! is written there as a `.eml` file, for tests and for trying the site out.

use dactile::build::outside;
use std::io::Write;
use std::path::PathBuf;
use std::process::Stdio;

#[derive(Debug, Clone)]
pub struct Mail {
    /// `smtps://smtp.example.com:465`, `smtp://…:587` (made secure before
    /// anything is said), or `file:///a/folder`.
    pub url: String,
    pub user: String,
    pub pass: String,
    /// The sender as the mail shows it: `Dactile <dactile@example.com>`.
    pub from: String,
}

impl Mail {
    /// From `DACTILE_MAIL_URL`, `DACTILE_MAIL_USER`, `DACTILE_MAIL_PASS` and
    /// `DACTILE_MAIL_FROM`; with no URL, no mail goes from here.
    pub fn from_env() -> Option<Mail> {
        let get = |k: &str| std::env::var(k).ok().map(|v| v.trim().to_string());
        let url = get("DACTILE_MAIL_URL").filter(|u| !u.is_empty())?;
        Some(Mail {
            url,
            user: get("DACTILE_MAIL_USER").unwrap_or_default(),
            pass: get("DACTILE_MAIL_PASS").unwrap_or_default(),
            from: get("DACTILE_MAIL_FROM").unwrap_or_else(|| "Dactile <dactile@localhost>".into()),
        })
    }

    /// Sends `text` to `to` (an address `people::address` let through).
    /// Runs a program and waits for the network.
    pub fn send(&self, to: &str, subject: &str, text: &str) -> Result<(), String> {
        if to.contains(|c: char| c.is_whitespace() || c.is_control() || c == '<' || c == '>') {
            return Err(format!("{to}: not an address to send to"));
        }
        let message = message(&self.from, to, subject, text);
        if let Some(dir) = self.url.strip_prefix("file://") {
            let dir = PathBuf::from(dir);
            std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            let name = format!(
                "{}-{}.eml",
                crate::door::now(),
                &crate::door::random().map_err(|e| e.to_string())?[..8]
            );
            return std::fs::write(dir.join(name), message).map_err(|e| e.to_string());
        }
        // The message waits in a file of the gate's own; curl's settings,
        // with the password, go through its stdin.
        let dir = std::env::temp_dir();
        let file = dir.join(format!(
            "dactile-mail-{}",
            crate::door::random().map_err(|e| e.to_string())?
        ));
        {
            use std::os::unix::fs::OpenOptionsExt;
            let mut f = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&file)
                .map_err(|e| e.to_string())?;
            f.write_all(message.as_bytes()).map_err(|e| e.to_string())?;
        }
        let sent = self.curl(to, &file);
        let _ = std::fs::remove_file(&file);
        sent
    }

    fn curl(&self, to: &str, file: &std::path::Path) -> Result<(), String> {
        let quoted = |v: &str| v.replace('\\', "\\\\").replace('"', "\\\"");
        let bare = |v: &str| {
            let v = v.trim();
            match (v.rfind('<'), v.rfind('>')) {
                (Some(a), Some(b)) if a < b => v[a + 1..b].to_string(),
                _ => v.to_string(),
            }
        };
        let mut config = format!(
            "url = \"{}\"\nssl-reqd\nmail-from = \"{}\"\nmail-rcpt = \"{}\"\nupload-file = \"{}\"\n",
            quoted(&self.url),
            quoted(&bare(&self.from)),
            quoted(to),
            quoted(&file.to_string_lossy()),
        );
        if !self.user.is_empty() {
            config.push_str(&format!(
                "user = \"{}:{}\"\n",
                quoted(&self.user),
                quoted(&self.pass)
            ));
        }
        let mut child = outside("curl")
            .args([
                "--silent",
                "--show-error",
                "--max-time",
                "30",
                "--config",
                "-",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("curl: {e}"))?;
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(config.as_bytes());
        }
        let out = child.wait_with_output().map_err(|e| format!("curl: {e}"))?;
        if out.status.success() {
            Ok(())
        } else {
            Err(format!(
                "mail: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ))
        }
    }
}

/// A mail as it travels: headers, a blank line, the text. The subject in
/// base64, as headers are ASCII; the text as it is, in UTF-8.
fn message(from: &str, to: &str, subject: &str, text: &str) -> String {
    let one = |v: &str| v.replace(['\r', '\n'], " ");
    let body: String = text
        .replace("\r\n", "\n")
        .lines()
        .map(|l| format!("{l}\r\n"))
        .collect();
    format!(
        "From: {}\r\nTo: {}\r\nSubject: =?UTF-8?B?{}?=\r\nMIME-Version: 1.0\r\nContent-Type: text/plain; charset=UTF-8\r\nContent-Transfer-Encoding: 8bit\r\nAuto-Submitted: auto-generated\r\n\r\n{body}",
        one(from),
        one(to),
        base64(one(subject).as_bytes()),
    )
}

fn base64(bytes: &[u8]) -> String {
    const LETTERS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (b[0] as u32) << 16 | (b[1] as u32) << 8 | b[2] as u32;
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(LETTERS[(n >> (18 - 6 * i)) as usize & 63] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// What the gate says in its mail, in Turkish: the site is Turkish first.
pub mod say {
    pub fn invitation(by: &str, link: &str) -> (String, String) {
        let by = if by.is_empty() { "Site sahibi" } else { by };
        (
            "Dactile'a davet".to_string(),
            format!(
                "Merhaba,\n\n{by} seni Dactile'a çağırıyor: tiyatro oyunları yazmak ve birlikte çalışmak için bir site.\n\nHesabını açmak için bu bağlantıyı aç:\n{link}\n\nBağlantı yedi gün geçerli ve bir kez çalışır. Beklemediğin bir davetse bu postayı silebilirsin.\n\nDactile"
            ),
        )
    }

    pub fn new_password(name: &str, link: &str) -> (String, String) {
        (
            "Dactile: yeni parola".to_string(),
            format!(
                "Merhaba {name},\n\nDactile hesabın için yeni bir parola istendi. Yeni parolanı bu bağlantıdan belirleyebilirsin:\n{link}\n\nBağlantı bir saat geçerli ve bir kez çalışır. Sen istemediysen bu postayı silebilirsin; parolan değişmez.\n\nDactile"
            ),
        )
    }

    pub fn check(name: &str, link: &str) -> (String, String) {
        (
            "Dactile: e-posta adresini doğrula".to_string(),
            format!(
                "Merhaba {name},\n\nBu adresi Dactile hesabına eklemek için bağlantıyı aç:\n{link}\n\nSen istemediysen bu postayı silebilirsin.\n\nDactile"
            ),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_mail_is_headers_and_text_with_the_subject_in_base64() {
        assert_eq!(base64(b"Kar Masal"), "S2FyIE1hc2Fs");
        assert_eq!(base64(b"a"), "YQ==");
        let m = message(
            "Dactile <d@example.com>",
            "ada@example.com",
            "Şiir\nBcc: x",
            "Bir\nİki",
        );
        assert!(m.starts_with("From: Dactile <d@example.com>\r\nTo: ada@example.com\r\n"));
        // A subject can't add a header.
        assert!(!m.contains("\nBcc"));
        assert!(m.ends_with("\r\n\r\nBir\r\nİki\r\n"));
    }

    #[test]
    fn to_a_folder_nothing_is_sent_and_a_file_is_written() {
        let dir = std::env::temp_dir().join(format!("dactile-mail-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let m = Mail {
            url: format!("file://{}", dir.display()),
            user: String::new(),
            pass: String::new(),
            from: "Dactile <d@example.com>".into(),
        };
        let (subject, text) = say::invitation("Ada", "https://oyun.example/davet/abc");
        m.send("baran@example.com", &subject, &text).unwrap();
        assert!(
            m.send("baran@example.com\r\nBcc: x@example.com", "x", "y")
                .is_err()
        );
        let files: Vec<_> = std::fs::read_dir(&dir).unwrap().collect();
        assert_eq!(files.len(), 1);
        let eml = std::fs::read_to_string(files[0].as_ref().unwrap().path()).unwrap();
        assert!(eml.contains("To: baran@example.com") && eml.contains("/davet/abc"));
        std::fs::remove_dir_all(dir).unwrap();
    }
}
