use std::path::Path;

use crate::config::{Dependency, MojoConfig};
use crate::project::Project;

pub fn exec() -> anyhow::Result<()> {
    let project = Project::discover()?;
    println!(
        "{} v{}",
        project.config.package.name, project.config.package.version
    );

    let deps: Vec<_> = project.config.dependencies.iter().collect();
    for (i, (name, dep)) in deps.iter().enumerate() {
        let is_last = i == deps.len() - 1;
        let prefix = if is_last {
            "\u{2514}\u{2500}\u{2500} "
        } else {
            "\u{251c}\u{2500}\u{2500} "
        };
        let child_prefix = if is_last { "    " } else { "\u{2502}   " };
        print_dep(name, dep, &project.root, prefix, child_prefix, 1)?;
    }

    Ok(())
}

fn print_dep(
    name: &str,
    dep: &Dependency,
    project_root: &Path,
    prefix: &str,
    child_prefix: &str,
    depth: usize,
) -> anyhow::Result<()> {
    let dep_path = match dep {
        Dependency::Path { path } => {
            let abs = if path.is_absolute() {
                path.clone()
            } else {
                project_root.join(path)
            };
            println!("{}{} (path: {})", prefix, name, path.display());
            abs
        }
        Dependency::Git {
            git, tag, branch, ..
        } => {
            let version_hint = tag.as_deref().or(branch.as_deref()).unwrap_or("latest");
            println!("{}{} ({}) {}", prefix, name, git, version_hint);
            project_root.join("deps").join(name)
        }
    };

    if depth >= 10 {
        return Ok(());
    }

    // Try to load sub-dependency config
    if dep_path.join("Mojo.toml").exists()
        && let Ok(config) = MojoConfig::load(&dep_path)
    {
        let sub_deps: Vec<_> = config.dependencies.iter().collect();
        for (j, (sub_name, sub_dep)) in sub_deps.iter().enumerate() {
            let is_last = j == sub_deps.len() - 1;
            let sub_prefix = format!(
                "{}{}",
                child_prefix,
                if is_last {
                    "\u{2514}\u{2500}\u{2500} "
                } else {
                    "\u{251c}\u{2500}\u{2500} "
                }
            );
            let sub_child = format!(
                "{}{}",
                child_prefix,
                if is_last { "    " } else { "\u{2502}   " }
            );
            print_dep(
                sub_name,
                sub_dep,
                &dep_path,
                &sub_prefix,
                &sub_child,
                depth + 1,
            )?;
        }
    }

    Ok(())
}
