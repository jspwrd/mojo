use anyhow::{Context, bail};
use std::path::PathBuf;

use crate::project::Project;
use crate::util;

const SOURCE_EXTENSIONS: &[&str] = &["c", "cpp", "cxx", "cc", "h", "hpp", "hxx", "hh"];

pub fn exec(check: bool) -> anyhow::Result<()> {
    let project = Project::discover()?;

    let clang_format = which::which("clang-format")
        .context("clang-format not found. Install it to use `mojo fmt`")?;

    let mut files = Vec::new();
    collect_formattable(&project.src_dir(), &mut files)?;
    collect_formattable(&project.include_dir(), &mut files)?;

    if files.is_empty() {
        util::status("Format", "no source files found");
        return Ok(());
    }

    files.sort();

    if check {
        util::status("Checking", &format!("formatting ({} files)", files.len()));
        let mut failures = Vec::new();

        for file in &files {
            let output = std::process::Command::new(&clang_format)
                .arg("--dry-run")
                .arg("--Werror")
                .arg(file)
                .output()
                .with_context(|| format!("failed to run clang-format on {}", file.display()))?;

            if !output.status.success() {
                failures.push(file.display().to_string());
                if !output.stderr.is_empty() {
                    eprint!("{}", String::from_utf8_lossy(&output.stderr));
                }
            }
        }

        if failures.is_empty() {
            util::status("Finished", "all files formatted correctly");
            Ok(())
        } else {
            bail!(
                "{} file(s) need formatting:\n  {}",
                failures.len(),
                failures.join("\n  ")
            );
        }
    } else {
        util::status("Formatting", &format!("{} files", files.len()));

        for file in &files {
            let output = std::process::Command::new(&clang_format)
                .arg("-i")
                .arg(file)
                .output()
                .with_context(|| format!("failed to run clang-format on {}", file.display()))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                bail!("clang-format failed on {}\n{}", file.display(), stderr);
            }
        }

        util::status("Finished", "formatting complete");
        Ok(())
    }
}

fn collect_formattable(dir: &PathBuf, files: &mut Vec<PathBuf>) -> anyhow::Result<()> {
    if !dir.exists() {
        return Ok(());
    }

    for entry in walkdir::WalkDir::new(dir) {
        let entry = entry?;
        if entry.file_type().is_file()
            && entry
                .path()
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|ext| SOURCE_EXTENSIONS.contains(&ext))
        {
            files.push(entry.into_path());
        }
    }

    Ok(())
}
