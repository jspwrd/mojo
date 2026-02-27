use anyhow::bail;

use crate::build;
use crate::project::Project;
use crate::util;

pub fn exec(release: bool, jobs: Option<usize>, args: &[String]) -> anyhow::Result<()> {
    let project = Project::discover()?;

    if project.config.is_lib() {
        bail!(
            "cannot run a library project. Use `mojo build` instead, or change type to \"bin\" in Mojo.toml"
        );
    }

    let result = build::build(&project, release, jobs)?;

    util::status("Running", &format!("`{}`", result.output.display()));

    let status = std::process::Command::new(&result.output)
        .args(args)
        .status()?;

    std::process::exit(status.code().unwrap_or(1));
}
