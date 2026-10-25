use clap::Parser;

/// Dactile — Dynamic Stagewright: write plays for the tiyatro class.
#[derive(Parser)]
#[command(version)]
struct Cli {}

fn main() {
    let _ = Cli::parse();
    println!(
        "dactile {} — tiyatro class {}",
        env!("CARGO_PKG_VERSION"),
        dactile::class::version()
    );
}
