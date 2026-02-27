use crate::deps::resolve_dependencies;
use crate::project::Project;
use crate::util;

pub fn exec() -> anyhow::Result<()> {
    let project = Project::discover()?;

    // Remove deps directory to force re-fetch
    let deps_dir = project.deps_dir();
    if deps_dir.exists() {
        std::fs::remove_dir_all(&deps_dir)?;
        util::status("Removed", "deps directory");
    }

    // Remove lock file
    let lock_path = project.root.join("Mojo.lock");
    if lock_path.exists() {
        std::fs::remove_file(&lock_path)?;
        util::status("Removed", "Mojo.lock");
    }

    // Re-resolve (will re-fetch and write new lock)
    util::status("Updating", &project.config.package.name);
    resolve_dependencies(&project)?;

    util::status("Updated", "all dependencies");
    Ok(())
}
