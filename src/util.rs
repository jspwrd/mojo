use colored::Colorize;

pub fn status(label: &str, message: &str) {
    println!("{:>12} {}", label.green().bold(), message);
}

pub fn error(message: &str) {
    eprintln!("{:>12} {}", "error".red().bold(), message);
}
