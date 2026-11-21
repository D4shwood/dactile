fn main() {
    // When this commit was made: a linked site's desktop is newer than this
    // one if its stamp is later (site.rs, "update"). Built from a tree with
    // no git, 0: then any the site has is newer.
    let stamp = std::process::Command::new("git")
        .args(["log", "-1", "--format=%ct"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| s.parse::<u64>().is_ok())
        .unwrap_or_else(|| "0".into());
    println!("cargo:rustc-env=DACTILE_STAMP={stamp}");
    for head in ["../../.git/HEAD", "../../.git/refs/heads"] {
        println!("cargo:rerun-if-changed={head}");
    }
    tauri_build::build()
}
