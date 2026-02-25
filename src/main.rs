mod build;
mod cli;
mod commands;
mod compiler;
mod config;
mod deps;
mod incremental;
mod project;
mod util;

use clap::Parser;
use cli::{Cli, Command};

fn main() {
    if let Err(e) = run() {
        util::error(&format!("{:#}", e));
        std::process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::New { name, lang, lib } => commands::new::exec(&name, &lang, lib),
        Command::Init { lang, lib } => commands::init::exec(&lang, lib),
        Command::Build { release } => commands::build::exec(release),
        Command::Run { release, args } => commands::run::exec(release, &args),
        Command::Clean => commands::clean::exec(),
    }
}
