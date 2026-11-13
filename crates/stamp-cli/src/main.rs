//! Command-line interface to STAMP. Subcommands are added as the core is ported.

use clap::Parser;

/// Statistical analysis of taxonomic and functional profiles.
#[derive(Parser)]
#[command(name = "stamp-cli", version = stamp_core::VERSION)]
struct Cli {}

fn main() {
    let _cli = Cli::parse();
    eprintln!(
        "stamp-cli {}: no commands are implemented yet.",
        stamp_core::VERSION
    );
    std::process::exit(2);
}
