#![allow(unexpected_cfgs)]

pub mod cli;
pub mod commands;
pub mod completions;
pub mod diff;
pub mod report;

use clap::Parser;

pub fn run() -> anyhow::Result<()> {
    let cli = cli::Cli::parse();

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
        cli::Commands::ListKeys(args) => commands::list_keys::run(args)?,
        cli::Commands::Info(args) => commands::info::run(args, cli.verbose)?,
        cli::Commands::Get(args) => commands::get::run(args, cli.verbose)?,
        cli::Commands::Set(args) => commands::set::run(args, cli.verbose)?,
        cli::Commands::Clear(args) => commands::clear::run(args, cli.verbose)?,
        cli::Commands::Cover(args) => commands::cover::run(args, cli.verbose)?,
        cli::Commands::Apply(args) => commands::apply::run(args, cli.verbose)?,
        cli::Commands::InitManifest(args) => commands::init_manifest::run(args)?,
        cli::Commands::Export(cli::ExportCommands::Metadata(args)) => {
            commands::export_metadata::run(args, cli.verbose)?
        }
        cli::Commands::Completions(args) => {
            let mut stdout = std::io::stdout().lock();
            completions::generate_completions(args.shell, &mut stdout)?
        }
        cli::Commands::Man => {
            let mut stdout = std::io::stdout().lock();
            completions::generate_man(&mut stdout)?
        }
        cli::Commands::Update => commands::update::run()?,
    }

    Ok(())
}
