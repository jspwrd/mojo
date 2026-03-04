use anyhow::{Context, bail};
use std::path::PathBuf;

use crate::build;
use crate::project::Project;
use crate::util;

pub fn exec(prefix: Option<&str>, profile: Option<&str>) -> anyhow::Result<()> {
    let project = Project::discover()?;

    if project.config.is_lib() {
        bail!("cannot install a library project");
    }

    let profile_name = profile.unwrap_or("release");
    let result = build::build(&project, profile_name, None, &[], None)?;

    let prefix_path = expand_prefix(prefix.unwrap_or("~/.local"))?;
    let bin_dir = prefix_path.join("bin");
    std::fs::create_dir_all(&bin_dir)
        .with_context(|| format!("failed to create {}", bin_dir.display()))?;

    let dest = bin_dir.join(&project.config.package.name);
    std::fs::copy(&result.output, &dest)
        .with_context(|| format!("failed to copy to {}", dest.display()))?;

    // Set executable permissions on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&dest, std::fs::Permissions::from_mode(0o755))?;
    }

    util::status(
        "Installed",
        &format!("{} to {}", project.config.package.name, dest.display()),
    );
    Ok(())
}

fn expand_prefix(prefix: &str) -> anyhow::Result<PathBuf> {
    if let Some(rest) = prefix.strip_prefix("~/") {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .context("could not determine home directory")?;
        Ok(PathBuf::from(home).join(rest))
    } else if prefix == "~" {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .context("could not determine home directory")?;
        Ok(PathBuf::from(home))
    } else {
        Ok(PathBuf::from(prefix))
    }
}
