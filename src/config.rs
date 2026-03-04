use anyhow::{bail, Context};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub const VALID_SANITIZERS: &[&str] = &["address", "undefined", "thread", "memory", "leak"];

pub fn validate_project_name(name: &str) -> anyhow::Result<()> {
    if name.is_empty() {
        bail!("project name cannot be empty");
    }
    if name.contains('/') || name.contains('\\') || name.contains("..") {
        bail!("project name '{}' contains invalid characters", name);
    }
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        bail!(
            "project name '{}' must contain only alphanumeric characters, hyphens, or underscores",
            name
        );
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct MojoConfig {
    pub package: Package,
    #[serde(default)]
    pub build: BuildConfig,
    #[serde(default)]
    pub profile: HashMap<String, Profile>,
    #[serde(default)]
    pub dependencies: HashMap<String, Dependency>,
    #[serde(default)]
    pub scripts: Scripts,
    #[serde(default)]
    pub target: HashMap<String, TargetConfig>,
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
    #[serde(default)]
    pub cflags: Vec<String>,
    #[serde(default)]
    pub ldflags: Vec<String>,
    #[serde(default)]
    pub libs: Vec<String>,
    #[serde(default)]
    pub jobs: Option<usize>,
    #[serde(default)]
    pub sanitizers: Vec<String>,
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            compiler: "auto".to_string(),
            cflags: Vec::new(),
            ldflags: Vec::new(),
            libs: Vec::new(),
            jobs: None,
            sanitizers: Vec::new(),
        }
    }
}

fn default_compiler() -> String {
    "auto".to_string()
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

#[derive(Debug, Deserialize, Default)]
pub struct Scripts {
    pub pre_build: Option<String>,
    pub post_build: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TargetConfig {
    pub cc: Option<String>,
    pub cxx: Option<String>,
    pub ar: Option<String>,
    #[serde(default)]
    pub cflags: Vec<String>,
    #[serde(default)]
    pub ldflags: Vec<String>,
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
        validate_project_name(&self.package.name)?;

        if self.package.version.is_empty() {
            bail!("package version cannot be empty");
        }

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

        // Validate profile opt_levels
        for (name, profile) in &self.profile {
            if !profile.opt_level.is_empty() {
                validate_opt_level(&profile.opt_level)
                    .with_context(|| format!("in profile '{}'", name))?;
            }
        }

        // Validate config sanitizers
        for s in &self.build.sanitizers {
            if !VALID_SANITIZERS.contains(&s.as_str()) {
                bail!(
                    "invalid sanitizer '{}'. Valid sanitizers: {}",
                    s,
                    VALID_SANITIZERS.join(", ")
                );
            }
        }

        Ok(())
    }

    pub fn is_lib(&self) -> bool {
        self.package.pkg_type == "lib"
    }

    pub fn profile(&self, name: &str) -> Profile {
        if let Some(p) = self.profile.get(name) {
            return p.clone();
        }
        match name {
            "debug" => Profile::default(),
            "release" => Profile {
                opt_level: "3".to_string(),
                debug: false,
                lto: true,
            },
            _ => Profile::default(),
        }
    }
}

fn validate_opt_level(level: &str) -> anyhow::Result<()> {
    match level {
        "0" | "1" | "2" | "3" | "s" | "z" => Ok(()),
        other => bail!(
            "invalid opt_level '{}': expected 0, 1, 2, 3, s, or z",
            other
        ),
    }
}

pub fn validate_sanitizers(sanitizers: &[String]) -> anyhow::Result<()> {
    for s in sanitizers {
        if !VALID_SANITIZERS.contains(&s.as_str()) {
            bail!(
                "invalid sanitizer '{}'. Valid sanitizers: {}",
                s,
                VALID_SANITIZERS.join(", ")
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_project_names() {
        assert!(validate_project_name("hello").is_ok());
        assert!(validate_project_name("my-lib").is_ok());
        assert!(validate_project_name("my_lib").is_ok());
        assert!(validate_project_name("lib123").is_ok());
    }

    #[test]
    fn invalid_project_names() {
        assert!(validate_project_name("").is_err());
        assert!(validate_project_name("../bad").is_err());
        assert!(validate_project_name("a/b").is_err());
        assert!(validate_project_name("a\\b").is_err());
        assert!(validate_project_name("hello world").is_err());
        assert!(validate_project_name("hello!").is_err());
    }

    #[test]
    fn valid_opt_levels() {
        for level in &["0", "1", "2", "3", "s", "z"] {
            assert!(validate_opt_level(level).is_ok());
        }
    }

    #[test]
    fn invalid_opt_levels() {
        assert!(validate_opt_level("4").is_err());
        assert!(validate_opt_level("fast").is_err());
        assert!(validate_opt_level("").is_err());
    }

    #[test]
    fn valid_sanitizers() {
        assert!(validate_sanitizers(&["address".into(), "undefined".into()]).is_ok());
        assert!(validate_sanitizers(&["thread".into()]).is_ok());
        assert!(validate_sanitizers(&[]).is_ok());
    }

    #[test]
    fn invalid_sanitizers() {
        assert!(validate_sanitizers(&["bogus".into()]).is_err());
        assert!(validate_sanitizers(&["address".into(), "invalid".into()]).is_err());
    }

    #[test]
    fn profile_defaults() {
        let config_str = r#"
[package]
name = "test"
version = "0.1.0"
"#;
        let config: MojoConfig = toml::from_str(config_str).unwrap();
        let debug = config.profile("debug");
        assert_eq!(debug.opt_level, "0");
        assert!(debug.debug);
        assert!(!debug.lto);

        let release = config.profile("release");
        assert_eq!(release.opt_level, "3");
        assert!(!release.debug);
        assert!(release.lto);
    }

    #[test]
    fn custom_profile() {
        let config_str = r#"
[package]
name = "test"
version = "0.1.0"

[profile.custom]
opt_level = "2"
debug = true
lto = true
"#;
        let config: MojoConfig = toml::from_str(config_str).unwrap();
        let custom = config.profile("custom");
        assert_eq!(custom.opt_level, "2");
        assert!(custom.debug);
        assert!(custom.lto);
    }
}
