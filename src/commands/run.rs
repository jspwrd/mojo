use anyhow::{bail, Context};

use crate::build;
use crate::project::Project;
use crate::util;

pub fn exec(
    release: bool,
    jobs: Option<usize>,
    sanitizers: &[String],
    profile: Option<&str>,
    target: Option<&str>,
    args: &[String],
) -> anyhow::Result<()> {
    let project = Project::discover()?;

    if project.config.is_lib() {
        bail!(
            "cannot run a library project. Use `mojo build` instead, or change type to \"bin\" in Mojo.toml"
        );
    }

    let profile_name = profile.unwrap_or(if release { "release" } else { "debug" });
    let result = build::build(&project, profile_name, jobs, sanitizers, target)?;

    util::status("Running", &format!("`{}`", result.output.display()));

    let status = std::process::Command::new(&result.output)
        .args(args)
        .status()
        .with_context(|| format!("failed to execute `{}`", result.output.display()))?;

    if !status.success() {
        let code = status.code().unwrap_or(1);
        util::error(&format!(
            "process `{}` exited with code {}",
            result.output.display(),
            code
        ));
        std::process::exit(code);
    }

    Ok(())
}
