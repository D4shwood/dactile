use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use dactile::cast::{Cast, Kind, Row};
use dactile::{source, tex};
use std::path::PathBuf;

mod install;
mod tui;

/// Dactile — Dynamic Stagewright: write plays for the tiyatro class.
#[derive(Parser)]
#[command(version)]
struct Cli {
    /// The play to open in the writer.
    play: Option<PathBuf>,
    #[command(subcommand)]
    cmd: Option<Cmd>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Starts a new play in the plays folder and opens it.
    New { title: String },
    /// Puts `dactile` in ~/.local/bin, adds a menu entry, and the tiyatro
    /// class where XeLaTeX finds it.
    Install,
    /// Removes what `install` added (plays and settings stay).
    Uninstall,
    /// Lists a play's characters as the app reads them (changes nothing).
    Cast { play: PathBuf },
    /// Lists a play's acts and scenes as the app reads them (changes nothing).
    Outline {
        play: PathBuf,
        /// Print the scene casts as the class writes them to the .aux file.
        #[arg(long)]
        aux: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Some(Cmd::Cast { play }) => print_cast(&play),
        Some(Cmd::Outline { play, aux }) => print_outline(&play, aux),
        Some(Cmd::Install) => {
            for line in install::install()? {
                println!("{line}");
            }
            Ok(())
        }
        Some(Cmd::Uninstall) => {
            for line in install::uninstall()? {
                println!("{line}");
            }
            Ok(())
        }
        Some(Cmd::New { title }) => {
            let cfg = dactile::config::Config::load()?;
            let path = cfg.plays_dir().join(dactile::library::file_name(&title));
            anyhow::ensure!(!path.exists(), "{} already exists", path.display());
            std::fs::create_dir_all(cfg.plays_dir())?;
            let lang = match cfg.lang.as_str() {
                "en" => dactile::lang::Lang::En,
                "tr" => dactile::lang::Lang::Tr,
                _ => dactile::lang::system(),
            };
            dactile::file::write_atomic(
                &path,
                &dactile::library::template(&title, &cfg.author, lang),
            )?;
            tui::run(tui::Start::Play(path))
        }
        None => match cli.play {
            Some(p) => tui::run(tui::Start::Play(p)),
            None => tui::run(tui::Start::Home),
        },
    }
}

fn print_cast(path: &PathBuf) -> Result<()> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let parts = source::split(&text);
    let cast = Cast::parse(&text, parts.preamble.clone());
    let body = parts.body.clone();
    let line = |alias: &str| {
        let (lines, dos) = dactile::cast::uses(&text, body.clone(), alias);
        format!("{lines:>3}× \\{alias:<6} {dos:>2}× do")
    };
    let row = |i: usize, indent: &str| {
        let c = &cast.characters[i];
        let star = if c.star { "★" } else { " " };
        let full = c
            .full
            .as_deref()
            .map(|f| format!(" ({f})"))
            .unwrap_or_default();
        let desc = c.desc.as_deref().unwrap_or("");
        println!(
            "{indent}{star} \\{:<8} {:<16}{} {}  — {desc}",
            c.alias,
            c.script,
            line(&c.alias),
            full
        );
    };
    println!("Kişiler tablosu:");
    for r in cast.rows() {
        match r {
            Row::Character(i) => row(i, "  "),
            Row::Group(g) => {
                println!("  ┌ {}", cast.groups[g].desc);
                for &m in &cast.groups[g].members {
                    row(m, "  │ ");
                }
            }
        }
    }
    let others: Vec<usize> = (0..cast.characters.len())
        .filter(|&i| matches!(cast.characters[i].kind, Kind::Plain | Kind::Hidden))
        .collect();
    if !others.is_empty() {
        println!("Tabloda olmayanlar:");
        for i in others {
            row(i, "  ");
        }
    }
    let total = tex::commands(&text, body).len();
    println!(
        "\n{} karakter, {} grup, gövdede {total} komut.",
        cast.characters.len(),
        cast.groups.len()
    );
    Ok(())
}

fn print_outline(path: &PathBuf, aux: bool) -> Result<()> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let cast = Cast::of_play(&text);
    let blocks = dactile::lex::blocks(&text, source::split(&text).body, &cast);
    let o = dactile::outline::Outline::of(&text, &blocks, &cast);
    if aux {
        for s in o.scenes.iter().filter(|s| !s.cast.is_empty()) {
            println!(
                "\\tiyatrocast{{{}}}{{{}}}",
                s.key(&o.acts),
                s.cast.join(",")
            );
        }
        return Ok(());
    }
    let name = |a: &str| cast.get(a).map_or(a.to_string(), |c| c.script.clone());
    let mut act = None;
    for s in &o.scenes {
        if s.act != act {
            act = s.act;
            if let Some(a) = act {
                let a = &o.acts[a];
                println!("{}. Perde {}", dactile::outline::roman(a.number), a.title);
            }
        }
        let place = if s.place.is_empty() {
            String::new()
        } else {
            format!(" — {}", s.place)
        };
        println!(
            "  Sahne {}{place}  ·  {} replik, {} kelime (+{} yönerge)",
            s.label, s.lines, s.words, s.direction_words
        );
        let names: Vec<String> = s.cast.iter().map(|a| name(a)).collect();
        println!("    {}", names.join(", "));
    }
    let unknown: Vec<String> = blocks
        .iter()
        .filter_map(|b| match &b.kind {
            dactile::lex::Kind::Unknown { name } => {
                Some(format!("{} (satır {})", name, b.line + 1))
            }
            _ => None,
        })
        .collect();
    if !unknown.is_empty() {
        println!(
            "\nTanımsız komutla başlayan satırlar: {}",
            unknown.join(", ")
        );
    }
    println!(
        "\n{} perde, {} sahne, {} numaralı satır.",
        o.acts.len(),
        o.scenes.len(),
        o.numbers.iter().flatten().count()
    );
    Ok(())
}
