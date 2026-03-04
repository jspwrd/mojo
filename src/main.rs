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
        Command::Check {
            release,
            jobs,
            sanitize,
            profile,
            target,
        } => commands::check::exec(release, jobs, &sanitize, profile.as_deref(), target.as_deref()),
        Command::Build {
            release,
            jobs,
            sanitize,
            profile,
            target,
        } => commands::build::exec(release, jobs, &sanitize, profile.as_deref(), target.as_deref()),
        Command::Run {
            release,
            jobs,
            sanitize,
            profile,
            target,
            args,
        } => commands::run::exec(release, jobs, &sanitize, profile.as_deref(), target.as_deref(), &args),
        Command::Test {
            release,
            jobs,
            sanitize,
            profile,
            target,
            filter,
        } => commands::test::exec(release, jobs, &sanitize, profile.as_deref(), target.as_deref(), filter.as_deref()),
        Command::Fmt { check } => commands::fmt::exec(check),
        Command::Add {
            name,
            path,
            git,
            tag,
            branch,
            rev,
        } => commands::add::exec(
            &name,
            path.as_deref(),
            git.as_deref(),
            tag.as_deref(),
            branch.as_deref(),
            rev.as_deref(),
        ),
        Command::Tree => commands::tree::exec(),
        Command::Install { prefix, profile } => {
            commands::install::exec(prefix.as_deref(), profile.as_deref())
        }
        Command::Clean => commands::clean::exec(),
        Command::Update => commands::update::exec(),
    }
}
