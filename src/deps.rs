use anyhow::{bail, Context};
use std::path::{Path, PathBuf};

use crate::config::{Dependency, MojoConfig};
use crate::project::Project;

pub struct ResolvedDep {
    pub name: String,
    pub root: PathBuf,
    pub include_path: PathBuf,
    pub sources: Vec<PathBuf>,
    pub config: Option<MojoConfig>,
}

pub fn resolve_dependencies(project: &Project) -> anyhow::Result<Vec<ResolvedDep>> {
    let mut resolved = Vec::new();

    for (name, dep) in &project.config.dependencies {
        match dep {
            Dependency::Path { path } => {
                let abs_path = if path.is_absolute() {
                    path.clone()
                } else {
                    project.root.join(path)
                };
                if !abs_path.exists() {
                    bail!(
                        "dependency '{}': path '{}' does not exist",
                        name,
                        abs_path.display()
                    );
                }
                resolved.push(load_dep(name, &abs_path)?);
            }
            Dependency::Git {
                git: url,
                tag,
                branch,
                rev,
            } => {
                let dest = project.deps_dir().join(name);
                if !dest.exists() {
                    fetch_git_dep(url, tag.as_deref(), branch.as_deref(), rev.as_deref(), &dest)
                        .with_context(|| format!("failed to fetch dependency '{}'", name))?;
                }
                resolved.push(load_dep(name, &dest)?);
            }
        }
    }

    Ok(resolved)
}

fn fetch_git_dep(
    url: &str,
    tag: Option<&str>,
    branch: Option<&str>,
    rev: Option<&str>,
    dest: &Path,
) -> anyhow::Result<()> {
    use crate::util;

    util::status("Fetching", &format!("{} from {}", dest.file_name().unwrap().to_string_lossy(), url));

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
        if entry.file_type().is_file() {
            if let Some(ext) = entry.path().extension().and_then(|e| e.to_str()) {
                if Language::from_extension(ext).is_some() {
                    sources.push(entry.into_path());
                }
            }
        }
    }

    Ok(sources)
}
