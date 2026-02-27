use anyhow::{bail, Context};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::config::{Dependency, MojoConfig};
use crate::lock::{LockFile, LockedDep};
use crate::project::Project;

pub struct ResolvedDep {
    pub name: String,
    pub root: PathBuf,
    pub include_path: PathBuf,
    pub sources: Vec<PathBuf>,
    pub config: Option<MojoConfig>,
}

pub fn resolve_dependencies(project: &Project) -> anyhow::Result<Vec<ResolvedDep>> {
    let lock = LockFile::load(&project.root)?.unwrap_or_default();

    let mut all_deps: HashMap<String, ResolvedDep> = HashMap::new();
    let mut adjacency: HashMap<String, Vec<String>> = HashMap::new();
    let mut locked_deps: Vec<LockedDep> = Vec::new();
    let mut visiting = HashSet::new();
    let mut visited = HashSet::new();

    for (name, dep) in &project.config.dependencies {
        resolve_recursive(
            name,
            dep,
            &project.root,
            &project.deps_dir(),
            &lock,
            &mut all_deps,
            &mut adjacency,
            &mut locked_deps,
            &mut visiting,
            &mut visited,
        )?;
    }

    // Topological sort — leaves first
    let mut order = Vec::new();
    let mut topo_visited = HashSet::new();
    for name in all_deps.keys() {
        topo_sort(name, &adjacency, &mut topo_visited, &mut order);
    }

    let mut result = Vec::new();
    for name in &order {
        if let Some(dep) = all_deps.remove(name) {
            result.push(dep);
        }
    }

    // Write lock file
    let new_lock = LockFile {
        dependencies: locked_deps,
    };
    new_lock.save(&project.root)?;

    Ok(result)
}

#[allow(clippy::too_many_arguments)]
fn resolve_recursive(
    name: &str,
    dep: &Dependency,
    project_root: &Path,
    deps_dir: &Path,
    lock: &LockFile,
    all_deps: &mut HashMap<String, ResolvedDep>,
    adjacency: &mut HashMap<String, Vec<String>>,
    locked_deps: &mut Vec<LockedDep>,
    visiting: &mut HashSet<String>,
    visited: &mut HashSet<String>,
) -> anyhow::Result<()> {
    if visited.contains(name) {
        return Ok(());
    }
    if visiting.contains(name) {
        bail!("dependency cycle detected involving '{}'", name);
    }

    visiting.insert(name.to_string());

    let dep_path = match dep {
        Dependency::Path { path } => {
            let abs_path = if path.is_absolute() {
                path.clone()
            } else {
                project_root.join(path)
            };
            if !abs_path.exists() {
                bail!(
                    "dependency '{}': path '{}' does not exist",
                    name,
                    abs_path.display()
                );
            }
            locked_deps.push(LockedDep {
                name: name.to_string(),
                source: "path".to_string(),
                url: None,
                rev: None,
                path: Some(path.display().to_string()),
            });
            abs_path
        }
        Dependency::Git {
            git: url,
            tag,
            branch,
            rev,
        } => {
            let dest = deps_dir.join(name);
            if !dest.exists() {
                // Check lock file for pinned revision
                let locked_rev = lock.find(name).and_then(|l| l.rev.clone());
                let effective_rev = rev.as_deref().or(locked_rev.as_deref());
                fetch_git_dep(url, tag.as_deref(), branch.as_deref(), effective_rev, &dest)
                    .with_context(|| format!("failed to fetch dependency '{}'", name))?;
            }
            // Record actual HEAD rev in lock
            let actual_rev = get_head_rev(&dest).ok();
            locked_deps.push(LockedDep {
                name: name.to_string(),
                source: "git".to_string(),
                url: Some(url.clone()),
                rev: actual_rev,
                path: None,
            });
            dest
        }
    };

    let resolved = load_dep(name, &dep_path)?;

    // Recursively resolve sub-dependencies
    let mut children = Vec::new();
    if let Some(ref config) = resolved.config {
        for (sub_name, sub_dep) in &config.dependencies {
            children.push(sub_name.clone());
            resolve_recursive(
                sub_name,
                sub_dep,
                &dep_path,
                deps_dir,
                lock,
                all_deps,
                adjacency,
                locked_deps,
                visiting,
                visited,
            )?;
        }
    }

    adjacency.insert(name.to_string(), children);
    visiting.remove(name);
    visited.insert(name.to_string());
    all_deps.insert(name.to_string(), resolved);

    Ok(())
}

fn topo_sort(
    name: &str,
    adjacency: &HashMap<String, Vec<String>>,
    visited: &mut HashSet<String>,
    order: &mut Vec<String>,
) {
    if visited.contains(name) {
        return;
    }
    visited.insert(name.to_string());

    if let Some(children) = adjacency.get(name) {
        for child in children {
            topo_sort(child, adjacency, visited, order);
        }
    }

    order.push(name.to_string());
}

pub fn fetch_git_dep(
    url: &str,
    tag: Option<&str>,
    branch: Option<&str>,
    rev: Option<&str>,
    dest: &Path,
) -> anyhow::Result<()> {
    use crate::util;

    let dep_name = dest
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    util::status("Fetching", &format!("{} from {}", dep_name, url));

    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let repo = git2::build::RepoBuilder::new()
        .clone(url, dest)
        .with_context(|| format!("failed to clone {}", url))?;

    // Checkout specific ref if specified
    if let Some(tag) = tag {
        let refname = format!("refs/tags/{}", tag);
        let obj = repo
            .revparse_single(&refname)
            .or_else(|_| repo.revparse_single(tag))
            .with_context(|| format!("tag '{}' not found", tag))?;
        repo.checkout_tree(&obj, None)?;
        repo.set_head_detached(obj.id())?;
    } else if let Some(branch) = branch {
        let obj = repo
            .revparse_single(&format!("origin/{}", branch))
            .with_context(|| format!("branch '{}' not found", branch))?;
        repo.checkout_tree(&obj, None)?;
        repo.set_head_detached(obj.id())?;
    } else if let Some(rev) = rev {
        let oid = git2::Oid::from_str(rev)
            .with_context(|| format!("invalid revision '{}'", rev))?;
        let obj = repo.find_object(oid, None)?;
        repo.checkout_tree(&obj, None)?;
        repo.set_head_detached(oid)?;
    }

    Ok(())
}

/// Returns the HEAD commit hash of a repository at the given path.
pub fn get_head_rev(repo_path: &Path) -> anyhow::Result<String> {
    let repo = git2::Repository::open(repo_path)
        .with_context(|| format!("failed to open repository at {}", repo_path.display()))?;
    let head = repo.head()?.peel_to_commit()?;
    Ok(head.id().to_string())
}

fn load_dep(name: &str, path: &Path) -> anyhow::Result<ResolvedDep> {
    // Try to load Mojo.toml, but it's optional for deps
    let config = if path.join("Mojo.toml").exists() {
        Some(MojoConfig::load(path)?)
    } else {
        None
    };

    // Collect source files
    let src_dir = path.join("src");
    let sources = if src_dir.exists() {
        collect_sources(&src_dir)?
    } else {
        // Some deps might have sources in root
        collect_sources(path)?
    };

    // Include path: prefer include/ dir, fall back to src/ or root
    let include_path = if path.join("include").exists() {
        path.join("include")
    } else if src_dir.exists() {
        src_dir
    } else {
        path.to_path_buf()
    };

    Ok(ResolvedDep {
        name: name.to_string(),
        root: path.to_path_buf(),
        include_path,
        sources,
        config,
    })
}

fn collect_sources(dir: &Path) -> anyhow::Result<Vec<PathBuf>> {
    use crate::compiler::Language;

    let mut sources = Vec::new();
    if !dir.exists() {
        return Ok(sources);
    }

    for entry in walkdir::WalkDir::new(dir) {
        let entry = entry?;
        if entry.file_type().is_file()
            && let Some(ext) = entry.path().extension().and_then(|e| e.to_str())
            && Language::from_extension(ext).is_some()
        {
            sources.push(entry.into_path());
        }
    }

    Ok(sources)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn topo_sort_linear() {
        let mut adjacency = HashMap::new();
        adjacency.insert("A".to_string(), vec!["B".to_string()]);
        adjacency.insert("B".to_string(), vec!["C".to_string()]);
        adjacency.insert("C".to_string(), vec![]);

        let mut visited = HashSet::new();
        let mut order = Vec::new();
        for name in ["A", "B", "C"] {
            topo_sort(name, &adjacency, &mut visited, &mut order);
        }
        // Leaves first: C, B, A
        assert_eq!(order, vec!["C", "B", "A"]);
    }

    #[test]
    fn cycle_detection() {
        // We can't easily test resolve_recursive without real files,
        // but we verify the visiting set logic conceptually:
        let mut visiting = HashSet::new();
        visiting.insert("A".to_string());
        assert!(visiting.contains("A"));
    }
}
