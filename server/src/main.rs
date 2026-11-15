//! `dactile-server`: one person's Dactile in the browser.
//!
//! - `DACTILE_DATA`: their folder (plays, uploads, settings).
//! - `DACTILE_PASSWORD`: the password to sign in with.
//! - `DACTILE_GATE_WORD`: instead of a password, behind `dactile-gate`: the
//!   word the gate sends with every request. The gate starts such a server
//!   for each person itself.
//! - `DACTILE_WEB`: the built interface (`web/dist`), if this server serves it.
//! - `DACTILE_ADDR`: where to listen, `127.0.0.1:1430` unless said.
//! - `DACTILE_SECURE=1`: behind HTTPS, the cookie only goes over it.
//! - `DACTILE_PROJECTS`: behind the gate, the projects the person is in
//!   (`projects::Reach`, as JSON): their plays' folders are reached too.
//! - `DACTILE_WHO`: their name, which a play's lock shows the others.
//! - `DACTILE_SANDBOX=bwrap`: XeLaTeX runs boxed in (`dactile::build::tex`).
//!   Needed before anyone but the owner writes here: a play can read what
//!   XeLaTeX can.

use std::path::PathBuf;
use std::sync::Arc;

fn need(name: &str) -> anyhow::Result<String> {
    std::env::var(name).map_err(|_| anyhow::anyhow!("{name} is not set"))
}

fn main() -> anyhow::Result<()> {
    let root = dactile_server::prepare(&PathBuf::from(need("DACTILE_DATA")?))?;
    let gate = std::env::var("DACTILE_GATE_WORD")
        .ok()
        .filter(|w| !w.is_empty());
    let password = match &gate {
        Some(_) => String::new(),
        None => need("DACTILE_PASSWORD")?,
    };
    anyhow::ensure!(
        gate.is_some() || password.len() >= 12,
        "DACTILE_PASSWORD needs 12 characters or more"
    );
    let web = std::env::var_os("DACTILE_WEB").map(PathBuf::from);
    let addr = std::env::var("DACTILE_ADDR").unwrap_or_else(|_| "127.0.0.1:1430".into());
    let secure = std::env::var("DACTILE_SECURE").is_ok_and(|v| v == "1");
    sandbox(&root)?;
    if gate.is_some() {
        leave_with_the_gate();
    }
    let server = Arc::new(
        match gate {
            Some(word) => dactile_server::Server::behind(root, word),
            None => dactile_server::Server::new(root, password, secure),
        }
        .with_projects(projects()),
    );
    dactile_backend::spell::load(server.ctx.spell.clone());
    tokio::runtime::Runtime::new()?.block_on(async {
        let listener = tokio::net::TcpListener::bind(&addr).await?;
        eprintln!("dactile-server on http://{addr}");
        axum::serve(listener, dactile_server::router(server, web)).await?;
        Ok(())
    })
}

fn projects() -> Vec<dactile_server::projects::Reach> {
    std::env::var("DACTILE_PROJECTS")
        .ok()
        .and_then(|j| serde_json::from_str(&j).ok())
        .unwrap_or_default()
}

/// Says how XeLaTeX will run, and stops if it was asked to be boxed in and
/// the box doesn't work here (a system that keeps `bwrap` from its users).
fn sandbox(root: &std::path::Path) -> anyhow::Result<()> {
    if !dactile::build::boxed() {
        eprintln!("XeLaTeX runs unboxed: fine for the owner alone, not for anyone else's plays");
        return Ok(());
    }
    let works = dactile::build::boxed_tex(&root.join("cache"), &[])
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .output();
    match works {
        Ok(o) if o.status.success() => Ok(()),
        Ok(o) => anyhow::bail!(
            "DACTILE_SANDBOX=bwrap, but the box doesn't start: {}",
            String::from_utf8_lossy(&o.stderr).trim()
        ),
        Err(e) => anyhow::bail!("DACTILE_SANDBOX=bwrap, but bwrap doesn't run: {e}"),
    }
}

/// Behind the gate, this server is the gate's to stop. Should the gate go
/// without stopping it, the pipe it held open closes, and this goes too:
/// left behind it would keep the person's play locked against their next
/// server.
fn leave_with_the_gate() {
    std::thread::spawn(|| {
        use std::io::Read;
        let mut sink = [0u8; 64];
        let mut from = std::io::stdin();
        while matches!(from.read(&mut sink), Ok(n) if n > 0) {}
        std::process::exit(0);
    });
}
