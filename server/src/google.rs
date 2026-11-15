//! Signing in with a Google account: the gate sends the browser to Google,
//! Google sends it back with a code, and the gate asks Google whose code it
//! is. Whoever's e-mail address is on the list comes in as that person; no
//! one else does.
//!
//! The answer is taken straight from Google over HTTPS, so it is Google's
//! word and needs no signature checked (OpenID Connect Core, 3.1.3.7). The
//! asking is done by `curl`, the system's: its HTTPS is the machine's own,
//! kept up to date with it, and nothing of the kind is built in here.

use dactile::build::outside;
use std::io::Write;
use std::process::Stdio;

const AUTH: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN: &str = "https://oauth2.googleapis.com/token";

#[derive(Debug, Clone)]
pub struct Google {
    /// The OAuth client's id and secret, from the Google Cloud console.
    pub id: String,
    pub secret: String,
    /// The site as browsers reach it, `https://oyun.example`: Google sends
    /// them back to `/giris/google/donus` under it.
    pub site: String,
    /// Where browsers are sent, and where the code is asked about: Google's.
    /// Only a test says otherwise.
    pub auth: String,
    pub token: String,
}

impl Google {
    /// From `DACTILE_GOOGLE_ID`, `DACTILE_GOOGLE_SECRET` and `DACTILE_SITE`;
    /// none, and nobody signs in with Google here.
    pub fn from_env() -> Option<Google> {
        let get = |k: &str| std::env::var(k).ok().filter(|v| !v.trim().is_empty());
        Some(Google {
            id: get("DACTILE_GOOGLE_ID")?,
            secret: get("DACTILE_GOOGLE_SECRET")?,
            site: get("DACTILE_SITE")?.trim_end_matches('/').to_string(),
            auth: AUTH.into(),
            token: TOKEN.into(),
        })
    }

    pub fn back(&self) -> String {
        format!("{}/giris/google/donus", self.site)
    }

    /// Where the browser goes to choose its account. `state` comes back with
    /// it, `nonce` comes back inside Google's answer: both must be the ones
    /// this browser was given.
    pub fn start(&self, state: &str, nonce: &str) -> String {
        let q = |v: &str| {
            percent_encoding::utf8_percent_encode(v, percent_encoding::NON_ALPHANUMERIC).to_string()
        };
        format!(
            "{}?response_type=code&scope=openid%20email&client_id={}&redirect_uri={}&state={}&nonce={}",
            self.auth,
            q(&self.id),
            q(&self.back()),
            q(state),
            q(nonce),
        )
    }

    /// Asks Google whose `code` it is: their e-mail address, if Google has
    /// checked it is theirs. Runs a program and waits for the network.
    pub fn whose(&self, code: &str, nonce: &str) -> Result<String, String> {
        // It goes into a file of curl's own syntax: nothing but what a code
        // is made of.
        let plain = |c: char| c.is_ascii_alphanumeric() || "-._~/+=%".contains(c);
        if code.is_empty() || code.len() > 2048 || !code.chars().all(plain) {
            return Err("not a code".into());
        }
        let quoted = |v: &str| v.replace('\\', "\\\\").replace('"', "\\\"");
        // Through its stdin: the secret is on no command line.
        let config = format!(
            "url = \"{}\"\ndata-urlencode = \"grant_type=authorization_code\"\ndata-urlencode = \"code={code}\"\ndata-urlencode = \"client_id={}\"\ndata-urlencode = \"client_secret={}\"\ndata-urlencode = \"redirect_uri={}\"\n",
            quoted(&self.token),
            quoted(&self.id),
            quoted(&self.secret),
            quoted(&self.back()),
        );
        let mut child = outside("curl")
            .args([
                "--silent",
                "--show-error",
                "--max-time",
                "15",
                "--config",
                "-",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("curl: {e}"))?;
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(config.as_bytes());
        }
        let out = child.wait_with_output().map_err(|e| format!("curl: {e}"))?;
        if !out.status.success() {
            return Err(format!(
                "curl: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ));
        }
        let answer: serde_json::Value =
            serde_json::from_slice(&out.stdout).map_err(|_| "no answer from Google".to_string())?;
        let token = answer["id_token"].as_str().ok_or_else(|| {
            // Google's own reason ("invalid_grant"), never the secret.
            format!(
                "Google: {}",
                answer["error"].as_str().unwrap_or("no id_token")
            )
        })?;
        said(token, &self.id, nonce, crate::door::now())
    }
}

/// The 64 letters of base64 as URLs have it, without padding.
fn base64url(text: &str) -> Option<Vec<u8>> {
    let value = |c: u8| match c {
        b'A'..=b'Z' => Some(c - b'A'),
        b'a'..=b'z' => Some(c - b'a' + 26),
        b'0'..=b'9' => Some(c - b'0' + 52),
        b'-' => Some(62),
        b'_' => Some(63),
        _ => None,
    };
    let text = text.trim_end_matches('=');
    let mut out = Vec::with_capacity(text.len() * 3 / 4);
    let (mut bits, mut have) = (0u32, 0);
    for c in text.bytes() {
        bits = (bits << 6) | value(c)? as u32;
        have += 6;
        if have >= 8 {
            have -= 8;
            out.push((bits >> have) as u8);
        }
    }
    Some(out)
}

/// What Google's answer says: the e-mail address, if the answer is for this
/// site (`client`), for this sign-in (`nonce`), still good at `now`, and of
/// an address Google has checked.
pub fn said(id_token: &str, client: &str, nonce: &str, now: u64) -> Result<String, String> {
    let payload = id_token.split('.').nth(1).ok_or("not a token")?;
    let claims: serde_json::Value = base64url(payload)
        .and_then(|b| serde_json::from_slice(&b).ok())
        .ok_or("not a token")?;
    let text = |k: &str| claims[k].as_str().unwrap_or("");
    if !["https://accounts.google.com", "accounts.google.com"].contains(&text("iss")) {
        return Err("not from Google".into());
    }
    if text("aud") != client {
        return Err("not for this site".into());
    }
    if text("nonce") != nonce || nonce.is_empty() {
        return Err("not for this sign-in".into());
    }
    if claims["exp"].as_u64().is_none_or(|exp| exp <= now) {
        return Err("too late".into());
    }
    // Google says it both ways.
    let checked = claims["email_verified"].as_bool() == Some(true)
        || claims["email_verified"].as_str() == Some("true");
    let mail = text("email").trim().to_lowercase();
    if !checked || !mail.contains('@') {
        return Err("no checked e-mail address".into());
    }
    Ok(mail)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// A token as Google makes them, with these claims: the middle part is
    /// all anyone reads here.
    pub fn token(claims: &serde_json::Value) -> String {
        const LETTERS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
        let bytes = claims.to_string().into_bytes();
        let mut out = String::new();
        for chunk in bytes.chunks(3) {
            let b = [
                chunk[0],
                *chunk.get(1).unwrap_or(&0),
                *chunk.get(2).unwrap_or(&0),
            ];
            let n = (b[0] as u32) << 16 | (b[1] as u32) << 8 | b[2] as u32;
            for i in 0..=chunk.len() {
                out.push(LETTERS[(n >> (18 - 6 * i)) as usize & 63] as char);
            }
        }
        format!("e30.{out}.imza")
    }

    fn good() -> serde_json::Value {
        json!({
            "iss": "https://accounts.google.com",
            "aud": "bizim-site",
            "nonce": "bu-giris",
            "exp": 2000,
            "email": "Ada@Example.com",
            "email_verified": true,
        })
    }

    #[test]
    fn base64_as_urls_have_it() {
        assert_eq!(
            base64url("S2FyIE1hc2FsxLE").unwrap(),
            "Kar Masalı".as_bytes()
        );
        assert_eq!(base64url("YQ==").unwrap(), b"a");
        assert!(base64url("a b").is_none());
    }

    #[test]
    fn a_good_answer_gives_the_address_in_small_letters() {
        let t = token(&good());
        assert_eq!(
            said(&t, "bizim-site", "bu-giris", 1000).unwrap(),
            "ada@example.com"
        );
    }

    #[test]
    fn an_answer_for_someone_or_something_else_gives_nothing() {
        let with = |k: &str, v: serde_json::Value| {
            let mut c = good();
            c[k] = v;
            said(&token(&c), "bizim-site", "bu-giris", 1000)
        };
        assert!(with("iss", json!("https://evil.example")).is_err());
        assert!(with("aud", json!("baska-site")).is_err());
        assert!(with("nonce", json!("baska-giris")).is_err());
        assert!(with("exp", json!(999)).is_err());
        assert!(with("email_verified", json!(false)).is_err());
        assert!(with("email", json!("")).is_err());
        assert!(with("email_verified", json!("true")).is_ok());
        assert!(said("not.a-token", "bizim-site", "bu-giris", 1000).is_err());
        // No nonce asked for is no sign-in of ours.
        let mut c = good();
        c["nonce"] = json!("");
        assert!(said(&token(&c), "bizim-site", "", 1000).is_err());
    }

    #[test]
    fn the_browser_is_sent_with_what_must_come_back() {
        let g = Google {
            id: "bizim-site".into(),
            secret: "gizli".into(),
            site: "https://oyun.example".into(),
            auth: AUTH.into(),
            token: TOKEN.into(),
        };
        let url = g.start("durum", "bu-giris");
        assert!(url.starts_with("https://accounts.google.com/"));
        assert!(url.contains("state=durum") && url.contains("nonce=bu%2Dgiris"));
        assert!(url.contains("redirect_uri=https%3A%2F%2Foyun%2Eexample%2Fgiris%2Fgoogle%2Fdonus"));
        assert!(!url.contains("gizli"));
        // What isn't a code never reaches curl's file.
        assert!(
            g.whose("a\"\nurl = \"http://evil.example", "bu-giris")
                .is_err()
        );
        assert!(g.whose("", "bu-giris").is_err());
    }
}
