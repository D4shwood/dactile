//! The settings a test sees are never the real ones: `.cargo/config.toml`
//! points them into `target`. This only looks; it writes nothing.

#[test]
fn settings_stay_out_of_the_home() {
    let path = dactile::config::path();
    let dir = dactile::config::dir();
    let real = dirs::home_dir().unwrap().join(".config/dactile");
    assert!(!path.starts_with(&real), "{}", path.display());
    assert!(!dir.starts_with(&real), "{}", dir.display());
}
