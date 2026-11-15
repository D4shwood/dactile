//! `dactile-gate`: one site for many people, each with a Dactile of their
//! own (`dactile_server::gate`).
//!
//! With no arguments it serves:
//! - `DACTILE_HOME`: its folder (`gate/`, and `people/<id>/` for each).
//! - `DACTILE_WEB`: the built interface, if this serves it.
//! - `DACTILE_ADDR`: where to listen, `127.0.0.1:1430` unless said.
//! - `DACTILE_SECURE=1`: behind HTTPS, the cookie only goes over it.
//! - `DACTILE_GOOGLE_ID`, `DACTILE_GOOGLE_SECRET`, `DACTILE_SITE`: with all
//!   three, people whose Google address is on the list sign in with Google.
//!   `DACTILE_SITE` is the site as browsers reach it, `https://oyun.example`.
//! - `DACTILE_SERVER`: the `dactile-server` program, beside this one unless
//!   said. `DACTILE_SANDBOX` and the rest go on to it as they are.
//!
//! The owner's commands, on the same `DACTILE_HOME`, while it serves or not:
//!
//!     dactile-gate add ada "Ada Yıldız" [--owner] [--mail ada@example.com]
//!                                         --mail: their Google account, to
//!                                         sign in with it
//!     dactile-gate invite ada [days]      a link good once, 7 days unless said
//!     dactile-gate people
//!     dactile-gate remove ada             off the list; their plays stay

use dactile_server::{gate, people};
use std::path::PathBuf;
use std::sync::Arc;

fn home() -> anyhow::Result<PathBuf> {
    let home =
        std::env::var("DACTILE_HOME").map_err(|_| anyhow::anyhow!("DACTILE_HOME is not set"))?;
    std::fs::create_dir_all(&home)?;
    Ok(PathBuf::from(home).canonicalize()?)
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let words: Vec<&str> = args.iter().map(String::as_str).collect();
    let said = |r: Result<(), String>| r.map_err(|e| anyhow::anyhow!(e));
    match words.as_slice() {
        [] => serve(),
        ["add", id, rest @ ..] => {
            let owner = rest.contains(&"--owner");
            let at = rest.iter().position(|w| *w == "--mail");
            let mail = at.and_then(|i| rest.get(i + 1)).copied();
            let name: Vec<&str> = rest
                .iter()
                .enumerate()
                .filter(|(i, w)| {
                    **w != "--owner" && Some(*i) != at && Some(*i) != at.map(|a| a + 1)
                })
                .map(|(_, w)| *w)
                .collect();
            said(people::add(&home()?, id, &name.join(" "), owner, mail))?;
            println!("{id} is on the list. A link for them: dactile-gate invite {id}");
            Ok(())
        }
        ["invite", id] | ["invite", id, _] => {
            let days = words.get(2).and_then(|d| d.parse().ok()).unwrap_or(7);
            let token = people::invite(&home()?, id, days).map_err(|e| anyhow::anyhow!(e))?;
            let site = std::env::var("DACTILE_SITE").unwrap_or_default();
            println!("{}/giris/{token}", site.trim_end_matches('/'));
            eprintln!("Good once, for {days} days. Whoever opens it comes in as {id}.");
            Ok(())
        }
        ["people"] => {
            for p in people::list(&home()?) {
                println!("{}\t{}\t{}\t{}", p.id, p.role, p.name, p.mail);
            }
            Ok(())
        }
        ["remove", id] => {
            said(people::remove(&home()?, id))?;
            println!("{id} is off the list. Their plays stay in people/{id}.");
            Ok(())
        }
        _ => anyhow::bail!(
            "dactile-gate [add <id> <name> [--owner] [--mail <address>] | invite <id> [days] | people | remove <id>]"
        ),
    }
}

fn serve() -> anyhow::Result<()> {
    let home = home()?;
    let server = match std::env::var_os("DACTILE_SERVER") {
        Some(s) => PathBuf::from(s),
        None => std::env::current_exe()?.with_file_name("dactile-server"),
    };
    anyhow::ensure!(server.exists(), "{} is not there", server.display());
    let web = std::env::var_os("DACTILE_WEB").map(PathBuf::from);
    let addr = std::env::var("DACTILE_ADDR").unwrap_or_else(|_| "127.0.0.1:1430".into());
    let secure = std::env::var("DACTILE_SECURE").is_ok_and(|v| v == "1");
    let mut gate = gate::Gate::new(home, server, secure);
    gate.google = dactile_server::google::Google::from_env();
    if gate.google.is_some() {
        eprintln!("signing in with Google is on");
    }
    let gate = Arc::new(gate);
    tokio::runtime::Runtime::new()?.block_on(async {
        let listener = tokio::net::TcpListener::bind(&addr).await?;
        eprintln!("dactile-gate on http://{addr}");
        let resting = gate.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                resting.rest().await;
            }
        });
        axum::serve(listener, gate::router(gate, web)).await?;
        Ok(())
    })
}
