use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct LockFile {
    #[serde(default, rename = "dependency")]
    pub dependencies: Vec<LockedDep>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct LockedDep {
    pub name: String,
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rev: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

impl LockFile {
    pub fn load(project_root: &Path) -> anyhow::Result<Option<Self>> {
        let lock_path = project_root.join("Mojo.lock");
        if !lock_path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(&lock_path)
            .with_context(|| format!("failed to read {}", lock_path.display()))?;
        let lock: LockFile = toml::from_str(&content)
            .with_context(|| format!("failed to parse {}", lock_path.display()))?;
        Ok(Some(lock))
    }

    pub fn save(&self, project_root: &Path) -> anyhow::Result<()> {
        let lock_path = project_root.join("Mojo.lock");
        let content = toml::to_string_pretty(self)
            .context("failed to serialize lock file")?;
        std::fs::write(&lock_path, content)
            .with_context(|| format!("failed to write {}", lock_path.display()))?;
        Ok(())
    }

    pub fn find(&self, name: &str) -> Option<&LockedDep> {
        self.dependencies.iter().find(|d| d.name == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let lock = LockFile {
            dependencies: vec![
                LockedDep {
                    name: "fmt".to_string(),
                    source: "git".to_string(),
                    url: Some("https://github.com/fmtlib/fmt".to_string()),
                    rev: Some("abc123".to_string()),
                    path: None,
                },
                LockedDep {
                    name: "mylib".to_string(),
                    source: "path".to_string(),
                    url: None,
                    rev: None,
                    path: Some("../mylib".to_string()),
                },
            ],
        };

        let serialized = toml::to_string_pretty(&lock).unwrap();
        let deserialized: LockFile = toml::from_str(&serialized).unwrap();
        assert_eq!(lock.dependencies.len(), deserialized.dependencies.len());
        assert_eq!(lock.dependencies[0], deserialized.dependencies[0]);
        assert_eq!(lock.dependencies[1], deserialized.dependencies[1]);
    }

    #[test]
    fn find_dep() {
        let lock = LockFile {
            dependencies: vec![LockedDep {
                name: "foo".to_string(),
                source: "git".to_string(),
                url: Some("https://example.com/foo".to_string()),
                rev: Some("deadbeef".to_string()),
                path: None,
            }],
        };

        assert!(lock.find("foo").is_some());
        assert!(lock.find("bar").is_none());
    }
}
