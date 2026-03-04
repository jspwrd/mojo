use crate::build;
use crate::project::Project;

pub fn exec(
    release: bool,
    jobs: Option<usize>,
    sanitizers: &[String],
    profile: Option<&str>,
    target: Option<&str>,
    filter: Option<&str>,
) -> anyhow::Result<()> {
    let project = Project::discover()?;
    let profile_name = profile.unwrap_or(if release { "release" } else { "debug" });
    let result = build::test(&project, profile_name, jobs, sanitizers, filter, target)?;

    if result.failed > 0 {
        std::process::exit(1);
    }

    Ok(())
}
