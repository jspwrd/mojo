use anyhow::{bail, Context};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
pub struct MojoConfig {
    pub package: Package,
    #[serde(default)]
    pub build: BuildConfig,
    #[serde(default)]
    pub profile: ProfileMap,
    #[serde(default)]
    pub dependencies: HashMap<String, Dependency>,
}

#[derive(Debug, Deserialize)]
pub struct Package {
    pub name: String,
    pub version: String,
    #[serde(default = "default_lang")]
    pub lang: String,
    pub std: Option<String>,
    #[serde(default = "default_type")]
    #[serde(rename = "type")]
    pub pkg_type: String,
    #[serde(default = "default_lib_type")]
    #[serde(rename = "lib-type")]
    pub lib_type: String,
}

fn default_lang() -> String {
    "c++".to_string()
}

fn default_type() -> String {
    "bin".to_string()
}

fn default_lib_type() -> String {
    "static".to_string()
}

#[derive(Debug, Deserialize)]
pub struct BuildConfig {
    #[serde(default = "default_compiler")]
    pub compiler: String,
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            compiler: "auto".to_string(),
        }
    }
}

fn default_compiler() -> String {
    "auto".to_string()
}

#[derive(Debug, Deserialize, Default)]
pub struct ProfileMap {
    pub debug: Option<Profile>,
    pub release: Option<Profile>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Profile {
    #[serde(default)]
    pub opt_level: String,
    #[serde(default)]
    pub debug: bool,
    #[serde(default)]
    pub lto: bool,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            opt_level: "0".to_string(),
            debug: true,
            lto: false,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum Dependency {
    Path {
        path: PathBuf,
    },
    Git {
        git: String,
        tag: Option<String>,
        branch: Option<String>,
        rev: Option<String>,
    },
}

impl MojoConfig {
    pub fn load(project_root: &Path) -> anyhow::Result<Self> {
        let config_path = project_root.join("Mojo.toml");
        let content = std::fs::read_to_string(&config_path)
            .with_context(|| format!("failed to read {}", config_path.display()))?;
        let config: MojoConfig = toml::from_str(&content)
            .with_context(|| format!("failed to parse {}", config_path.display()))?;
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> anyhow::Result<()> {
        match self.package.lang.as_str() {
            "c" | "c++" => {}
            other => bail!("invalid lang '{}': expected 'c' or 'c++'", other),
        }

        match self.package.pkg_type.as_str() {
            "bin" | "lib" => {}
            other => bail!("invalid type '{}': expected 'bin' or 'lib'", other),
        }

        if self.package.pkg_type == "lib" {
            match self.package.lib_type.as_str() {
                "static" | "shared" | "both" => {}
                other => bail!(
                    "invalid lib-type '{}': expected 'static', 'shared', or 'both'",
                    other
                ),
            }
        }

        if !matches!(
            self.build.compiler.as_str(),
            "auto" | "gcc" | "clang"
        ) {
            bail!(
                "invalid compiler '{}': expected 'auto', 'gcc', or 'clang'",
                self.build.compiler
            );
        }

        Ok(())
    }

    pub fn is_lib(&self) -> bool {
        self.package.pkg_type == "lib"
    }

    pub fn profile(&self, name: &str) -> Profile {
        match name {
            "debug" => self.profile.debug.clone().unwrap_or_default(),
            "release" => self.profile.release.clone().unwrap_or(Profile {
                opt_level: "3".to_string(),
                debug: false,
                lto: true,
            }),
            _ => Profile::default(),
        }
    }
}
