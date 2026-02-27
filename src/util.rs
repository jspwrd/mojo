use colored::Colorize;
use std::sync::atomic::{AtomicU8, Ordering};

// 0 = normal, 1 = quiet, 2 = verbose
static VERBOSITY: AtomicU8 = AtomicU8::new(0);

pub fn set_verbosity(verbose: bool, quiet: bool) {
    if quiet {
        VERBOSITY.store(1, Ordering::Relaxed);
    } else if verbose {
        VERBOSITY.store(2, Ordering::Relaxed);
    }
}

pub fn is_verbose() -> bool {
    VERBOSITY.load(Ordering::Relaxed) == 2
}

fn is_quiet() -> bool {
    VERBOSITY.load(Ordering::Relaxed) == 1
}

pub fn status(label: &str, message: &str) {
    if !is_quiet() {
        println!("{:>12} {}", label.green().bold(), message);
    }
}

pub fn verbose(label: &str, message: &str) {
    if is_verbose() {
        println!("{:>12} {}", label.cyan().bold(), message);
    }
}

#[allow(dead_code)]
pub fn warn(message: &str) {
    eprintln!("{:>12} {}", "warning".yellow().bold(), message);
}

pub fn error(message: &str) {
    eprintln!("{:>12} {}", "error".red().bold(), message);
}
