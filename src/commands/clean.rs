use crate::project::Project;
use crate::util;

pub fn exec() -> anyhow::Result<()> {
    let project = Project::discover()?;
    let build_dir = project.root.join("build");
    if build_dir.exists() {
        std::fs::remove_dir_all(&build_dir)?;
        util::status("Removed", "build directory");
    } else {
        util::status("Clean", "nothing to clean");
    }

    let deps_dir = project.deps_dir();
    if deps_dir.exists() {
        std::fs::remove_dir_all(&deps_dir)?;
        util::status("Removed", "deps directory");
    }

    Ok(())
}
