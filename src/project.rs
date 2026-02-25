use anyhow::bail;
use std::path::PathBuf;

use crate::config::MojoConfig;

pub struct Project {
    pub root: PathBuf,
    pub config: MojoConfig,
}

impl Project {
    /// Walk up from CWD to find Mojo.toml
    pub fn discover() -> anyhow::Result<Self> {
        let mut dir = std::env::current_dir()?;
        loop {
            if dir.join("Mojo.toml").exists() {
                let config = MojoConfig::load(&dir)?;
                return Ok(Self { root: dir, config });
            }
            if !dir.pop() {
                bail!("could not find Mojo.toml in current directory or any parent");
            }
        }
    }

    pub fn src_dir(&self) -> PathBuf {
        self.root.join("src")
    }

    pub fn include_dir(&self) -> PathBuf {
        self.root.join("include")
    }

    pub fn deps_dir(&self) -> PathBuf {
        self.root.join("deps")
    }

    pub fn build_dir(&self, profile: &str) -> PathBuf {
        self.root.join("build").join(profile)
    }

    pub fn obj_dir(&self, profile: &str) -> PathBuf {
        self.build_dir(profile).join("obj")
    }

    pub fn deps_build_dir(&self, profile: &str) -> PathBuf {
        self.build_dir(profile).join("deps")
    }
}
