use crate::build;
use crate::project::Project;

pub fn exec(release: bool) -> anyhow::Result<()> {
    let project = Project::discover()?;
    build::build(&project, release)?;
    Ok(())
}
