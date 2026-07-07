#![allow(unexpected_cfgs)]

pub mod cli;
pub mod commands;
pub mod diff;

use clap::Parser;

pub fn run() -> anyhow::Result<()> {
    let cli = cli::Cli::parse();
    let mut stdout = std::io::stdout().lock();
    run_with(cli, &mut stdout)
}

/// Run the CLI with an explicit [`Cli`](cli::Cli) value and stdout writer.
///
/// This is exposed so integration tests can exercise error propagation from
/// commands that write to stdout without spawning a separate process.
pub fn run_with<W: std::io::Write>(cli: cli::Cli, _stdout: &mut W) -> anyhow::Result<()> {
    let subscriber = tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_max_level(if cli.verbose {
            tracing::Level::DEBUG
        } else {
            tracing::Level::WARN
        })
        .finish();
    let _ = tracing::subscriber::set_global_default(subscriber);

    match &cli.command {
        cli::Commands::ListKeys(args) => commands::list_keys::run(args),
        cli::Commands::Info(args) => commands::info::run(args, cli.verbose)?,
        cli::Commands::Get(args) => commands::get::run(args, cli.verbose)?,
        cli::Commands::Set(args) => commands::set::run(args, cli.verbose)?,
        cli::Commands::Clear(args) => commands::clear::run(args, cli.verbose)?,
        cli::Commands::Cover(args) => commands::cover::run(args, cli.verbose)?,
        cli::Commands::Apply(args) => commands::apply::run(args, cli.verbose)?,
        cli::Commands::Export(cli::ExportCommands::Metadata(args)) => {
            commands::export_metadata::run(args, cli.verbose)?
        }
        cli::Commands::Update => commands::update::run()?,
    }

    Ok(())
}
