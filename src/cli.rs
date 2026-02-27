use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "mojo", version, about = "A build tool for C and C++")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
    /// Print verbose output (full compiler commands)
    #[arg(long, short = 'v', global = true)]
    pub verbose: bool,
    /// Suppress all status output
    #[arg(long, short = 'q', global = true)]
    pub quiet: bool,
}

#[derive(Subcommand)]
pub enum Command {
    /// Create a new mojo project
    New {
        /// Project name
        name: String,
        /// Language: "c" or "c++"
        #[arg(long, default_value = "c++", value_parser = ["c", "c++"])]
        lang: String,
        /// Create a library project instead of an executable
        #[arg(long)]
        lib: bool,
    },
    /// Initialize a mojo project in the current directory
    Init {
        /// Language: "c" or "c++"
        #[arg(long, default_value = "c++", value_parser = ["c", "c++"])]
        lang: String,
        /// Create a library project instead of an executable
        #[arg(long)]
        lib: bool,
    },
    /// Build the project
    Build {
        /// Build with release optimizations
        #[arg(long)]
        release: bool,
        /// Number of parallel compilation jobs
        #[arg(long, short = 'j')]
        jobs: Option<usize>,
    },
    /// Build and run the project
    Run {
        /// Build with release optimizations
        #[arg(long)]
        release: bool,
        /// Number of parallel compilation jobs
        #[arg(long, short = 'j')]
        jobs: Option<usize>,
        /// Arguments to pass to the executable
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Remove build artifacts
    Clean,
    /// Update dependencies (re-fetch and regenerate Mojo.lock)
    Update,
}
