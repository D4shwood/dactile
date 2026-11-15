//! XeLaTeX boxed in, as the web server runs it: a play builds, and reads
//! nothing but its own folder. Needs `bwrap` and `xelatex`; without them
//! the tests pass having checked nothing.

use dactile::build::boxed_tex;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn have(program: &str) -> bool {
    Command::new(program)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// A folder for a play and one beside it the play has no business in.
fn folders(name: &str) -> Option<(PathBuf, PathBuf)> {
    if !have("bwrap") || !have("xelatex") {
        eprintln!("no bwrap or no xelatex: nothing checked");
        return None;
    }
    let root = std::env::temp_dir().join(format!("dactile-box-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let (play, other) = (root.join("play"), root.join("other"));
    fs::create_dir_all(&play).unwrap();
    fs::create_dir_all(&other).unwrap();
    Some((play, other))
}

fn run(work: &Path, reads: &[&Path], tex: &str) -> (bool, String) {
    fs::write(work.join("play.tex"), tex).unwrap();
    // The class beside the play: it names fonts this machine is sure to have.
    for (name, body) in dactile::class::FILES {
        fs::write(work.join(name), body).unwrap();
    }
    let ok = boxed_tex(work, reads)
        .args(["-interaction=nonstopmode", "-halt-on-error", "play.tex"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap()
        .success();
    let log = fs::read_to_string(work.join("play.log")).unwrap_or_default();
    (ok, log)
}

const PLAY: &str = include_str!("../../examples/ornek-oyun.tex");

#[test]
fn a_play_builds_in_the_box() {
    let Some((play, _)) = folders("builds") else {
        return;
    };
    let (ok, log) = run(&play, &[], PLAY);
    assert!(ok, "{log}");
    assert!(play.join("play.pdf").exists());
    fs::remove_dir_all(play.parent().unwrap()).unwrap();
}

#[test]
fn a_play_reads_its_folder_and_what_it_is_given_and_nothing_else() {
    let Some((play, other)) = folders("reads") else {
        return;
    };
    fs::write(play.join("mine.tex"), "Benim.").unwrap();
    fs::write(other.join("secret.tex"), "GIZLI-PAROLA").unwrap();
    let ask = |file: &str| {
        format!("\\documentclass{{tiyatro}}\\begin{{document}}\\input{{{file}}}\\end{{document}}")
    };
    // Its own folder, yes.
    let (ok, log) = run(&play, &[], &ask("mine.tex"));
    assert!(ok, "{log}");
    // The folder beside it, the machine's users, this user's home: not there.
    let home = std::env::var("HOME").unwrap_or_default();
    for file in [
        other.join("secret.tex").display().to_string(),
        "../other/secret.tex".to_string(),
        "/etc/passwd".to_string(),
        "/proc/self/environ".to_string(),
        format!("{home}/.bashrc"),
        format!("{home}/.ssh/id_ed25519.pub"),
    ] {
        let (ok, log) = run(&play, &[], &ask(&file));
        assert!(!ok, "{file} was read");
        assert!(!log.contains("GIZLI-PAROLA"), "{file}");
    }
    // Given to read, it is read, and still can't be written.
    let (ok, log) = run(
        &play,
        &[&other],
        &ask(&other.join("secret.tex").display().to_string()),
    );
    assert!(ok, "{log}");
    let write = format!(
        "\\documentclass{{tiyatro}}\\begin{{document}}\\immediate\\openout3={}\\immediate\\write3{{x}}Tamam.\\end{{document}}",
        other.join("written.tex").display()
    );
    let _ = run(&play, &[&other], &write);
    assert!(!other.join("written.tex").exists());
    fs::remove_dir_all(play.parent().unwrap()).unwrap();
}

#[test]
fn the_box_has_no_network_and_none_of_the_servers_environment() {
    let Some((play, _)) = folders("env") else {
        return;
    };
    // What the box's XeLaTeX is told about the world.
    let mut c = boxed_tex(&play, &[]);
    let said: Vec<String> = c
        .get_envs()
        .map(|(k, _)| k.to_string_lossy().into_owned())
        .collect();
    assert!(!said.iter().any(|k| k.starts_with("DACTILE")), "{said:?}");
    let args: Vec<String> = c
        .get_args()
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    assert!(args.contains(&"--unshare-all".to_string()));
    // And it starts at all with nothing but that.
    assert!(
        c.arg("--version")
            .stdout(Stdio::null())
            .status()
            .unwrap()
            .success()
    );
    fs::remove_dir_all(play.parent().unwrap()).unwrap();
}
