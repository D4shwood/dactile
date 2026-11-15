//! `dactile-server`: one person's Dactile in the browser.
//!
//! - `DACTILE_DATA`: their folder (plays, uploads, settings).
//! - `DACTILE_PASSWORD`: the password to sign in with.
//! - `DACTILE_WEB`: the built interface (`web/dist`), if this server serves it.
//! - `DACTILE_ADDR`: where to listen, `127.0.0.1:1430` unless said.
//! - `DACTILE_SECURE=1`: behind HTTPS, the cookie only goes over it.

use std::path::PathBuf;
use std::sync::Arc;

fn need(name: &str) -> anyhow::Result<String> {
    std::env::var(name).map_err(|_| anyhow::anyhow!("{name} is not set"))
}

fn main() -> anyhow::Result<()> {
    let root = dactile_server::prepare(&PathBuf::from(need("DACTILE_DATA")?))?;
    let password = need("DACTILE_PASSWORD")?;
    anyhow::ensure!(
        password.len() >= 12,
        "DACTILE_PASSWORD needs 12 characters or more"
    );
    let web = std::env::var_os("DACTILE_WEB").map(PathBuf::from);
    let addr = std::env::var("DACTILE_ADDR").unwrap_or_else(|_| "127.0.0.1:1430".into());
    let secure = std::env::var("DACTILE_SECURE").is_ok_and(|v| v == "1");
    let server = Arc::new(dactile_server::Server::new(root, password, secure));
    dactile_backend::spell::load(server.ctx.spell.clone());
    tokio::runtime::Runtime::new()?.block_on(async {
        let listener = tokio::net::TcpListener::bind(&addr).await?;
        eprintln!("dactile-server on http://{addr}");
        axum::serve(listener, dactile_server::router(server, web)).await?;
        Ok(())
    })
}
