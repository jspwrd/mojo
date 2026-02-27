mod build;
mod cli;
mod commands;
mod compiler;
mod config;
mod deps;
mod incremental;
mod lock;
mod project;
mod scaffold;
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
    util::set_verbosity(cli.verbose, cli.quiet);

    match cli.command {
        Command::New { name, lang, lib } => commands::new::exec(&name, &lang, lib),
        Command::Init { lang, lib } => commands::init::exec(&lang, lib),
        Command::Build { release, jobs } => commands::build::exec(release, jobs),
        Command::Run { release, jobs, args } => commands::run::exec(release, jobs, &args),
        Command::Clean => commands::clean::exec(),
        Command::Update => commands::update::exec(),
    }
}
