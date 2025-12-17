use clap::Parser;
use repo_lint::cli::{CheckCommand, Cli, Commands, InspectCommand, ScaffoldCommand};
use repo_lint::output::OutputFormat;

fn main() {
    let cli = Cli::parse();

    let output_format = if cli.sarif {
        OutputFormat::Sarif
    } else if cli.json {
        OutputFormat::Json
    } else {
        OutputFormat::Console
    };

    let result = match &cli.command {
        Commands::Check(args) => CheckCommand::run_with_workspace(
            args,
            &cli.config,
            output_format,
            cli.workspace.as_deref(),
        ),
        Commands::Scaffold(args) => ScaffoldCommand::run(args, &cli.config, cli.json),
        Commands::Inspect(args) => InspectCommand::run(args, &cli.config, cli.json),
    };

    match result {
        Ok(code) => std::process::exit(code),
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}
