use crate::build;
use crate::project::Project;

pub fn exec(release: bool, jobs: Option<usize>) -> anyhow::Result<()> {
    let project = Project::discover()?;
    build::build(&project, release, jobs)?;
    Ok(())
}
