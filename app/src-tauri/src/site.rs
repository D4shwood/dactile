//! A Dactile site this desktop is linked to: its plays and projects open
//! here as they do in a browser, sent to the site's gate with a token of
//! this computer's own (server/src/gate.rs, "linking a desktop").
//!
//! The link is a file beside the settings (`site.toml`), readable by the
//! person alone: the site's address, the token, and whose it is. No
//! password is ever typed here: the desktop shows a code, the browser opens
//! the site's page for it, and someone signed in there says yes.

use dactile::lang::l;
use dactile_backend::Res;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

/// The link, as kept.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Link {
    /// `https://oyun.example`, no slash at the end.
    pub url: String,
    pub token: String,
    /// The account's user name, and the name it shows.
    pub id: String,
    pub name: String,
}

/// What the window is told of the link: never the token.
#[derive(Debug, Clone, Serialize)]
pub struct Shown {
    pub url: String,
    pub id: String,
    pub name: String,
}

/// A link being made: the site, and the word to wait with.
#[derive(Default)]
pub struct Linking(Mutex<Option<(String, String)>>);

fn file() -> PathBuf {
    dactile::config::path().with_file_name("site.toml")
}

pub fn read() -> Option<Link> {
    let text = std::fs::read_to_string(file()).ok()?;
    toml::from_str::<Link>(&text)
        .ok()
        .filter(|l| !l.url.is_empty() && !l.token.is_empty())
}

/// Written whole and moved over, readable by the person alone.
fn keep(link: &Link) -> Result<(), String> {
    let f = file();
    let text = toml::to_string(link).map_err(|e| e.to_string())?;
    let tmp = f.with_extension("new");
    (|| {
        if let Some(d) = f.parent() {
            std::fs::create_dir_all(d)?;
        }
        let mut o = std::fs::OpenOptions::new();
        o.write(true).create(true).truncate(true);
        // On Windows the person's own AppData is theirs alone already.
        #[cfg(unix)]
        std::os::unix::fs::OpenOptionsExt::mode(&mut o, 0o600);
        let mut w = o.open(&tmp)?;
        w.write_all(text.as_bytes())?;
        w.sync_all()?;
        std::fs::rename(&tmp, &f)
    })()
    .map_err(|e| e.to_string())
}

/// A site's address as typed: `oyun.example` is `https://oyun.example`.
pub fn address(typed: &str) -> Result<String, String> {
    let t = typed.trim().trim_end_matches('/');
    let url = if t.starts_with("http://") || t.starts_with("https://") {
        t.to_string()
    } else {
        format!("https://{t}")
    };
    let host = url.split_once("://").map_or("", |(_, h)| h);
    if host.is_empty() || host.contains(char::is_whitespace) || host.contains('/') {
        return Err(l("Bu bir site adresi değil.", "That is not a site's address.").into());
    }
    // A token goes in the clear over http: only to this computer itself.
    let local = ["127.0.0.1", "localhost", "[::1]"]
        .iter()
        .any(|h| host == *h || host.starts_with(&format!("{h}:")));
    if url.starts_with("http://") && !local {
        return Err(l(
            "Site https ile açılmalı.",
            "The site has to open with https.",
        )
        .into());
    }
    Ok(url)
}

/// What the person calls this computer, for their list of linked ones.
fn computer() -> String {
    let host = std::fs::read_to_string("/etc/hostname")
        .ok()
        .map(|h| h.trim().to_string())
        .filter(|h| !h.is_empty())
        .unwrap_or_else(|| "?".into());
    format!("Dactile · {host}")
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .http_status_as_error(false)
        .timeout_global(Some(Duration::from_secs(120)))
        .user_agent(concat!("Dactile/", env!("CARGO_PKG_VERSION")))
        .build()
        .into()
}

fn unreachable(e: impl std::fmt::Display) -> String {
    format!(
        "{}: {e}",
        l("Siteye ulaşılamadı", "The site can't be reached")
    )
}

/// The answer's body; the site's error as its message; "signed out" when
/// the site no longer lets this computer in.
fn answer(mut r: ureq::http::Response<ureq::Body>) -> Result<Vec<u8>, String> {
    let status = r.status().as_u16();
    let body = r
        .body_mut()
        .with_config()
        .limit(512 * 1024 * 1024)
        .read_to_vec()
        .map_err(unreachable)?;
    if status == 401 {
        return Err("signed out".into());
    }
    if !(200..300).contains(&status) {
        return Err(match serde_json::from_slice::<Value>(&body) {
            Ok(Value::String(s)) => s,
            Ok(v) => v.to_string(),
            Err(_) => format!("{status}"),
        });
    }
    Ok(body)
}

fn json_of(body: &[u8]) -> Result<Value, String> {
    if body.is_empty() {
        return Ok(Value::Null);
    }
    serde_json::from_slice(body).map_err(|e| e.to_string())
}

/// One request to the linked site, as its interface sends it.
fn send(link: &Link, method: &str, path: &str, body: Option<&Value>) -> Result<Vec<u8>, String> {
    let url = format!("{}{path}", link.url);
    let bearer = format!("Bearer {}", link.token);
    let a = agent();
    let r = if method == "GET" {
        a.get(&url).header("authorization", &bearer).call()
    } else {
        let text = body.map_or_else(|| "{}".to_string(), Value::to_string);
        a.post(&url)
            .header("authorization", &bearer)
            .header("x-dactile", "1")
            .header("content-type", "application/json")
            .send(text.as_bytes())
    };
    answer(r.map_err(unreachable)?)
}

fn linked() -> Result<Link, String> {
    read().ok_or_else(|| "signed out".to_string())
}

async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| e.to_string())?
}

// ---------------------------------------------------------------- commands

/// The site this computer is linked to, if one.
#[tauri::command]
pub fn site_status() -> Option<Shown> {
    read().map(|l| Shown {
        url: l.url,
        id: l.id,
        name: l.name,
    })
}

/// What a site says of itself, linked or not: its version and API number.
#[tauri::command]
pub async fn site_door(url: String) -> Res<Value> {
    let url = address(&url)?;
    blocking(move || {
        let r = agent()
            .get(format!("{url}/api/door"))
            .call()
            .map_err(unreachable)?;
        let v = json_of(&answer(r)?)?;
        if v["kind"] != "accounts" {
            return Err(l(
                "Bu adreste hesaplı bir Dactile sitesi yok.",
                "There is no Dactile site with accounts there.",
            )
            .into());
        }
        Ok(v)
    })
    .await
}

/// Starts linking to `url`: a code to show, and the site's page for it
/// opened in the browser.
#[tauri::command]
pub async fn site_connect(linking: tauri::State<'_, Linking>, url: String) -> Res<Value> {
    let url = address(&url)?;
    let from = url.clone();
    let asked = blocking(move || {
        let r = agent()
            .post(format!("{from}/api/cihaz"))
            .header("content-type", "application/json")
            .send(json!({ "name": computer() }).to_string().as_bytes())
            .map_err(unreachable)?;
        json_of(&answer(r)?)
    })
    .await?;
    let (Some(poll), Some(code), Some(page)) = (
        asked["poll"].as_str(),
        asked["code"].as_str(),
        asked["page"].as_str(),
    ) else {
        return Err(l(
            "Site beklenmedik bir yanıt verdi.",
            "The site gave an odd answer.",
        )
        .into());
    };
    *linking.0.lock().unwrap_or_else(|e| e.into_inner()) = Some((url.clone(), poll.to_string()));
    let page = format!("{url}{page}");
    let _ = dactile::build::open_url(&page);
    Ok(json!({ "code": code, "page": page }))
}

/// Waits for the yes (ten minutes at the most), and keeps the link.
#[tauri::command]
pub async fn site_wait(linking: tauri::State<'_, Linking>) -> Res<Shown> {
    let Some((url, poll)) = linking.0.lock().unwrap_or_else(|e| e.into_inner()).clone() else {
        return Err(l("Bağlanma başlamadı.", "No linking was started.").into());
    };
    let link = blocking(move || {
        let a = agent();
        for _ in 0..(11 * 60 / 2) {
            let r = a
                .get(format!("{url}/api/cihaz/{poll}"))
                .call()
                .map_err(unreachable)?;
            if r.status().as_u16() == 202 {
                std::thread::sleep(Duration::from_secs(2));
                continue;
            }
            let got = json_of(&answer(r)?)?;
            let link = Link {
                url: url.clone(),
                token: got["token"].as_str().unwrap_or_default().to_string(),
                id: got["id"].as_str().unwrap_or_default().to_string(),
                name: got["name"].as_str().unwrap_or_default().to_string(),
            };
            if link.token.is_empty() {
                return Err(l(
                    "Site beklenmedik bir yanıt verdi.",
                    "The site gave an odd answer.",
                )
                .into());
            }
            keep(&link)?;
            return Ok(link);
        }
        Err(l("Kodun süresi doldu.", "The code has run out.").into())
    })
    .await;
    *linking.0.lock().unwrap_or_else(|e| e.into_inner()) = None;
    let link = link?;
    Ok(Shown {
        url: link.url,
        id: link.id,
        name: link.name,
    })
}

/// Unlinks this computer: off the site's list if it can be reached, and
/// the link's file gone either way.
#[tauri::command]
pub async fn site_forget() -> Res<()> {
    if let Some(link) = read() {
        let _ = blocking(move || {
            let id = &link.token[..12.min(link.token.len())];
            send(&link, "POST", &format!("/api/cihazlar/{id}/kaldir"), None)
        })
        .await;
    }
    match std::fs::remove_file(file()) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.to_string()),
        _ => Ok(()),
    }
}

/// One of the site's routes, for the window's site commands
/// (app/src/lib/site/commands.ts).
#[tauri::command]
pub async fn site_request(method: String, path: String, body: Option<Value>) -> Res<Value> {
    if !path.starts_with("/api/") {
        return Err("not a site route".into());
    }
    blocking(move || {
        let link = linked()?;
        json_of(&send(&link, &method, &path, body.as_ref())?)
    })
    .await
}

/// A file of the site's (a PDF, a play): its bytes.
#[tauri::command]
pub async fn site_file(path: String) -> Res<tauri::ipc::Response> {
    blocking(move || {
        let link = linked()?;
        let q = format!("/api/file?path={}", encode(&path));
        send(&link, "GET", &q, None).map(tauri::ipc::Response::new)
    })
    .await
}

/// A file of the site's, kept on this computer for its own programs: where
/// it is now.
#[tauri::command]
pub async fn site_fetch(path: String) -> Res<String> {
    blocking(move || {
        let link = linked()?;
        let q = format!("/api/file?path={}", encode(&path));
        let bytes = send(&link, "GET", &q, None)?;
        let name = std::path::Path::new(&path)
            .file_name()
            .map_or_else(|| "dosya".into(), |n| n.to_string_lossy().into_owned());
        let dir = dactile::build::cache()
            .map_err(|e| e.to_string())?
            .join("site");
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let at = dir.join(name);
        std::fs::write(&at, bytes).map_err(|e| e.to_string())?;
        Ok(at.display().to_string())
    })
    .await
}

/// A file picked here, sent to the site (`play`: a .tex; `import`: a
/// document to read): its path there, or None if none was picked.
#[tauri::command]
pub async fn site_upload(kind: String) -> Res<Option<String>> {
    let dialog = rfd::AsyncFileDialog::new();
    let dialog = if kind == "play" {
        dialog.add_filter("Oyun", &["tex"])
    } else {
        dialog.add_filter(
            l("Oyun belgeleri", "Play documents"),
            &dactile::import::EXTENSIONS,
        )
    };
    let Some(f) = dialog.pick_file().await else {
        return Ok(None);
    };
    let from = f.path().to_path_buf();
    blocking(move || {
        let link = linked()?;
        let bytes = std::fs::read(&from).map_err(|e| e.to_string())?;
        let name = from
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        let r = agent()
            .post(format!("{}/api/upload/{}", link.url, encode(&kind)))
            .header("authorization", &format!("Bearer {}", link.token))
            .header("x-dactile", "1")
            .header("x-name", &encode(&name))
            .send(&bytes[..])
            .map_err(unreachable)?;
        match json_of(&answer(r)?)? {
            Value::String(s) => Ok(Some(s)),
            v => Ok(v.as_str().map(String::from)),
        }
    })
    .await
}

/// The address of a play's room, with a ticket good once.
#[tauri::command]
pub async fn site_room(path: String) -> Res<String> {
    blocking(move || {
        let link = linked()?;
        let got = json_of(&send(
            &link,
            "POST",
            "/api/oda/bilet",
            Some(&json!({ "path": path })),
        )?)?;
        let ticket = got["ticket"].as_str().unwrap_or_default();
        let ws = link
            .url
            .replacen("https://", "wss://", 1)
            .replacen("http://", "ws://", 1);
        Ok(format!("{ws}/api/oda/bilet/{ticket}"))
    })
    .await
}

// ------------------------------------------------------------- updating
//
// A site hands out the desktop built from the same commit as itself
// (deploy/deploy.sh puts it in the site's /indir/, signed with a key kept
// off the repository; tauri.conf.json has its public half). A desktop
// linked to the site takes it when its own is older: so the two stay the
// same version without anyone carrying the AppImage by hand.

/// When this desktop's commit was made (build.rs).
const STAMP: &str = env!("DACTILE_STAMP");

/// A release's stamp: the build part of its version (`0.3.0+1795…`).
fn stamp_of(v: &semver::Version) -> u64 {
    v.build.as_str().parse().unwrap_or(0)
}

fn updater(app: &tauri::AppHandle) -> Result<Option<tauri_plugin_updater::Updater>, String> {
    use tauri_plugin_updater::UpdaterExt;
    // Only an AppImage can put a new one in its own place on Linux; on
    // Windows the installer does it.
    if !cfg!(windows) && std::env::var_os("APPIMAGE").is_none() {
        return Ok(None);
    }
    let Some(link) = read() else {
        return Ok(None);
    };
    let url =
        tauri::Url::parse(&format!("{}/indir/latest.json", link.url)).map_err(|e| e.to_string())?;
    let mine: u64 = STAMP.parse().unwrap_or(0);
    app.updater_builder()
        .endpoints(vec![url])
        .map_err(|e| e.to_string())?
        .version_comparator(move |_, release| stamp_of(&release.version) > mine)
        .build()
        .map(Some)
        .map_err(|e| e.to_string())
}

/// The site's desktop, if it is newer than this one: its version.
#[tauri::command]
pub async fn site_update(app: tauri::AppHandle) -> Res<Option<String>> {
    let Some(u) = updater(&app)? else {
        return Ok(None);
    };
    let found = u.check().await.map_err(|e| e.to_string())?;
    Ok(found.map(|r| r.version))
}

/// The site's desktop in this one's place, and the window opened anew.
#[tauri::command]
pub async fn site_update_install(app: tauri::AppHandle) -> Res<()> {
    let Some(u) = updater(&app)? else {
        return Err(l(
            "Bu Dactile kendini güncelleyemiyor.",
            "This Dactile can't update itself.",
        )
        .into());
    };
    let Some(release) = u.check().await.map_err(|e| e.to_string())? else {
        return Ok(());
    };
    release
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(|e| e.to_string())?;
    app.restart()
}

/// Percent-encoding for a path's query or a header: all but the plainest.
fn encode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_address_is_https_but_on_this_computer() {
        assert_eq!(address("oyun.example/").unwrap(), "https://oyun.example");
        assert_eq!(
            address(" https://pbe.oyun.example ").unwrap(),
            "https://pbe.oyun.example"
        );
        assert_eq!(
            address("http://127.0.0.1:1430").unwrap(),
            "http://127.0.0.1:1430"
        );
        assert_eq!(
            address("http://localhost:1430/").unwrap(),
            "http://localhost:1430"
        );
        assert!(address("http://oyun.example").is_err());
        assert!(address("").is_err());
        assert!(address("oyun example").is_err());
        assert!(address("https://oyun.example/yol").is_err());
    }

    #[test]
    fn a_release_is_newer_by_its_stamp() {
        let v = |s: &str| semver::Version::parse(s).unwrap();
        assert_eq!(stamp_of(&v("0.3.0-beta.2+1795258000")), 1_795_258_000);
        assert_eq!(stamp_of(&v("0.3.0-beta.2")), 0);
    }

    #[test]
    fn names_are_encoded_whole() {
        assert_eq!(encode("Kar Masalı.tex"), "Kar%20Masal%C4%B1.tex");
        assert_eq!(encode("a/b?c&d"), "a%2Fb%3Fc%26d");
    }
}
