use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use dactile::cast::{Cast, Kind, Row};
use dactile::{source, tex};
use std::path::PathBuf;

/// Dactile — Dynamic Stagewright: write plays for the tiyatro class.
#[derive(Parser)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    cmd: Option<Cmd>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Lists a play's characters as the app reads them (changes nothing).
    Cast { play: PathBuf },
}

fn main() -> Result<()> {
    match Cli::parse().cmd {
        Some(Cmd::Cast { play }) => print_cast(&play),
        None => {
            println!(
                "dactile {} — tiyatro class {}",
                env!("CARGO_PKG_VERSION"),
                dactile::class::version()
            );
            Ok(())
        }
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
