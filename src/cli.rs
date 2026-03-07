use clap::{Parser, Subcommand, ValueEnum};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Framework {
    Qt,
    Gtk,
    Libcurl,
    Grpc,
    Gtest,
    Boost,
    Freertos,
    Zephyr,
}

impl fmt::Display for Framework {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Framework::Qt => write!(f, "Qt"),
            Framework::Gtk => write!(f, "GTK"),
            Framework::Libcurl => write!(f, "libcurl"),
            Framework::Grpc => write!(f, "gRPC"),
            Framework::Gtest => write!(f, "Google Test"),
            Framework::Boost => write!(f, "Boost"),
            Framework::Freertos => write!(f, "FreeRTOS"),
            Framework::Zephyr => write!(f, "Zephyr"),
        }
    }
}

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
        /// Scaffold with Qt framework support
        #[arg(long, group = "framework")]
        qt: bool,
        /// Scaffold with GTK framework support
        #[arg(long, group = "framework")]
        gtk: bool,
        /// Scaffold with libcurl HTTP client support
        #[arg(long, group = "framework")]
        libcurl: bool,
        /// Scaffold with gRPC framework support
        #[arg(long, group = "framework")]
        grpc: bool,
        /// Scaffold with Google Test framework support
        #[arg(long, group = "framework")]
        gtest: bool,
        /// Scaffold with Boost library support
        #[arg(long, group = "framework")]
        boost: bool,
        /// Scaffold with FreeRTOS support
        #[arg(long, group = "framework")]
        freertos: bool,
        /// Scaffold with Zephyr RTOS support
        #[arg(long, group = "framework")]
        zephyr: bool,
    },
    /// Initialize a mojo project in the current directory
    Init {
        /// Language: "c" or "c++"
        #[arg(long, default_value = "c++", value_parser = ["c", "c++"])]
        lang: String,
        /// Create a library project instead of an executable
        #[arg(long)]
        lib: bool,
        /// Scaffold with Qt framework support
        #[arg(long, group = "framework")]
        qt: bool,
        /// Scaffold with GTK framework support
        #[arg(long, group = "framework")]
        gtk: bool,
        /// Scaffold with libcurl HTTP client support
        #[arg(long, group = "framework")]
        libcurl: bool,
        /// Scaffold with gRPC framework support
        #[arg(long, group = "framework")]
        grpc: bool,
        /// Scaffold with Google Test framework support
        #[arg(long, group = "framework")]
        gtest: bool,
        /// Scaffold with Boost library support
        #[arg(long, group = "framework")]
        boost: bool,
        /// Scaffold with FreeRTOS support
        #[arg(long, group = "framework")]
        freertos: bool,
        /// Scaffold with Zephyr RTOS support
        #[arg(long, group = "framework")]
        zephyr: bool,
    },
    /// Check sources for errors without building
    Check {
        /// Check with release profile flags
        #[arg(long)]
        release: bool,
        /// Number of parallel check jobs
        #[arg(long, short = 'j')]
        jobs: Option<usize>,
        /// Enable sanitizers (e.g. address, undefined, thread, memory, leak)
        #[arg(long = "sanitize")]
        sanitize: Vec<String>,
        /// Use a custom profile (conflicts with --release)
        #[arg(long, conflicts_with = "release")]
        profile: Option<String>,
        /// Cross-compile for a target triple
        #[arg(long)]
        target: Option<String>,
    },
    /// Build the project
    Build {
        /// Build with release optimizations
        #[arg(long)]
        release: bool,
        /// Number of parallel compilation jobs
        #[arg(long, short = 'j')]
        jobs: Option<usize>,
        /// Enable sanitizers (e.g. address, undefined, thread, memory, leak)
        #[arg(long = "sanitize")]
        sanitize: Vec<String>,
        /// Use a custom profile (conflicts with --release)
        #[arg(long, conflicts_with = "release")]
        profile: Option<String>,
        /// Cross-compile for a target triple
        #[arg(long)]
        target: Option<String>,
    },
    /// Build and run the project
    Run {
        /// Build with release optimizations
        #[arg(long)]
        release: bool,
        /// Number of parallel compilation jobs
        #[arg(long, short = 'j')]
        jobs: Option<usize>,
        /// Enable sanitizers (e.g. address, undefined, thread, memory, leak)
        #[arg(long = "sanitize")]
        sanitize: Vec<String>,
        /// Use a custom profile (conflicts with --release)
        #[arg(long, conflicts_with = "release")]
        profile: Option<String>,
        /// Cross-compile for a target triple
        #[arg(long)]
        target: Option<String>,
        /// Arguments to pass to the executable
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Run tests
    Test {
        /// Build tests with release optimizations
        #[arg(long)]
        release: bool,
        /// Number of parallel compilation jobs
        #[arg(long, short = 'j')]
        jobs: Option<usize>,
        /// Enable sanitizers (e.g. address, undefined, thread, memory, leak)
        #[arg(long = "sanitize")]
        sanitize: Vec<String>,
        /// Use a custom profile (conflicts with --release)
        #[arg(long, conflicts_with = "release")]
        profile: Option<String>,
        /// Cross-compile for a target triple
        #[arg(long)]
        target: Option<String>,
        /// Filter tests by name
        filter: Option<String>,
    },
    /// Format source code using clang-format
    Fmt {
        /// Check formatting without modifying files
        #[arg(long)]
        check: bool,
    },
    /// Add a dependency to Mojo.toml
    Add {
        /// Dependency name
        name: String,
        /// Local path to dependency
        #[arg(long)]
        path: Option<String>,
        /// Git repository URL
        #[arg(long)]
        git: Option<String>,
        /// Git tag (requires --git)
        #[arg(long, requires = "git")]
        tag: Option<String>,
        /// Git branch (requires --git)
        #[arg(long, requires = "git")]
        branch: Option<String>,
        /// Git revision (requires --git)
        #[arg(long, requires = "git")]
        rev: Option<String>,
    },
    /// Show the dependency tree
    Tree,
    /// Install the project binary
    Install {
        /// Installation prefix (default: ~/.local)
        #[arg(long)]
        prefix: Option<String>,
        /// Build profile to use (default: release)
        #[arg(long)]
        profile: Option<String>,
    },
    /// Remove build artifacts
    Clean,
    /// Update dependencies (re-fetch and regenerate Mojo.lock)
    Update,
}

/// Resolve framework bool flags into an `Option<Framework>`.
pub fn resolve_framework(flags: &FrameworkFlags) -> Option<Framework> {
    if flags.qt {
        Some(Framework::Qt)
    } else if flags.gtk {
        Some(Framework::Gtk)
    } else if flags.libcurl {
        Some(Framework::Libcurl)
    } else if flags.grpc {
        Some(Framework::Grpc)
    } else if flags.gtest {
        Some(Framework::Gtest)
    } else if flags.boost {
        Some(Framework::Boost)
    } else if flags.freertos {
        Some(Framework::Freertos)
    } else if flags.zephyr {
        Some(Framework::Zephyr)
    } else {
        None
    }
}

/// Collected framework bool flags from the CLI.
pub struct FrameworkFlags {
    pub qt: bool,
    pub gtk: bool,
    pub libcurl: bool,
    pub grpc: bool,
    pub gtest: bool,
    pub boost: bool,
    pub freertos: bool,
    pub zephyr: bool,
}
