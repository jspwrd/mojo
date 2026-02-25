use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "mojo", version, about = "A build tool for C and C++")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Create a new mojo project
    New {
        /// Project name
        name: String,
        /// Language: "c" or "c++" (default: c++)
        #[arg(long, default_value = "c++")]
        lang: String,
        /// Create a library project instead of an executable
        #[arg(long)]
        lib: bool,
    },
    /// Initialize a mojo project in the current directory
    Init {
        /// Language: "c" or "c++" (default: c++)
        #[arg(long, default_value = "c++")]
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
    },
    /// Build and run the project
    Run {
        /// Build with release optimizations
        #[arg(long)]
        release: bool,
        /// Arguments to pass to the executable
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Remove build artifacts
    Clean,
}
