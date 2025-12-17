pub mod check;
pub mod inspect;
pub mod scaffold;

pub use check::*;
pub use inspect::*;
pub use scaffold::*;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "repo-lint")]
#[command(author = "Rika Labs")]
#[command(version)]
#[command(about = "A high-performance filesystem layout linter")]
#[command(
    long_about = "repo-lint enforces filesystem structure, naming conventions, and module boundaries via a TypeScript DSL config."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    #[arg(long, global = true, default_value = "repo-lint.config.ts")]
    pub config: String,

    #[arg(long, global = true)]
    pub json: bool,

    #[arg(long, global = true)]
    pub sarif: bool,

    #[arg(long, global = true)]
    pub agent: bool,

    #[arg(long, global = true)]
    pub trace: bool,

    #[arg(
        long,
        short = 'w',
        global = true,
        help = "Filter workspaces to run (e.g., apps/web or apps/*)"
    )]
    pub workspace: Option<String>,
}

#[derive(Subcommand)]
pub enum Commands {
    Check(CheckArgs),
    Scaffold(ScaffoldArgs),
    Inspect(InspectArgs),
}
