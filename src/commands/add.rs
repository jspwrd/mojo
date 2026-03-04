use anyhow::{bail, Context};

use crate::config::MojoConfig;
use crate::util;

pub fn exec(
    name: &str,
    path: Option<&str>,
    git: Option<&str>,
    tag: Option<&str>,
    branch: Option<&str>,
    rev: Option<&str>,
) -> anyhow::Result<()> {
    // Validate exactly one of path or git
    if path.is_none() && git.is_none() {
        bail!("exactly one of --path or --git is required");
    }
    if path.is_some() && git.is_some() {
        bail!("cannot specify both --path and --git");
    }

    let cwd = std::env::current_dir()?;
    let config_path = cwd.join("Mojo.toml");
    if !config_path.exists() {
        bail!("Mojo.toml not found in current directory");
    }

    let content =
        std::fs::read_to_string(&config_path).context("failed to read Mojo.toml")?;

    // Check for duplicate
    let config: MojoConfig = toml::from_str(&content).context("failed to parse Mojo.toml")?;
    if config.dependencies.contains_key(name) {
        bail!("dependency '{}' already exists in Mojo.toml", name);
    }

    // Build TOML inline table
    let dep_str = if let Some(p) = path {
        format!("{} = {{ path = \"{}\" }}", name, p)
    } else {
        let git_url = git.unwrap();
        let mut parts = format!("git = \"{}\"", git_url);
        if let Some(t) = tag {
            parts.push_str(&format!(", tag = \"{}\"", t));
        }
        if let Some(b) = branch {
            parts.push_str(&format!(", branch = \"{}\"", b));
        }
        if let Some(r) = rev {
            parts.push_str(&format!(", rev = \"{}\"", r));
        }
        format!("{} = {{ {} }}", name, parts)
    };

    // String-based insertion
    let new_content = insert_dependency(&content, &dep_str);
    std::fs::write(&config_path, new_content).context("failed to write Mojo.toml")?;

    util::status("Added", &format!("dependency '{}'", name));
    Ok(())
}

fn insert_dependency(content: &str, dep_line: &str) -> String {
    if let Some(deps_start) = content.find("[dependencies]") {
        // Find end of [dependencies] header line
        let header_end = content[deps_start..]
            .find('\n')
            .map(|i| deps_start + i + 1)
            .unwrap_or(content.len());

        // Scan forward to find next section header (line starting with '[')
        let mut insert_pos = content.len();
        let mut pos = header_end;
        for line in content[header_end..].lines() {
            if line.trim_start().starts_with('[') {
                insert_pos = pos;
                break;
            }
            pos += line.len() + 1; // +1 for \n
        }

        let mut result = String::with_capacity(content.len() + dep_line.len() + 2);
        result.push_str(&content[..insert_pos]);
        if !result.ends_with('\n') {
            result.push('\n');
        }
        result.push_str(dep_line);
        result.push('\n');
        if insert_pos < content.len() {
            result.push_str(&content[insert_pos..]);
        }
        result
    } else {
        // No [dependencies] section, append one
        let mut result = content.trim_end().to_string();
        result.push_str("\n\n[dependencies]\n");
        result.push_str(dep_line);
        result.push('\n');
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_into_existing_deps() {
        let content = r#"[package]
name = "test"
version = "0.1.0"

[dependencies]
foo = { path = "../foo" }

[profile.debug]
opt_level = "0"
"#;
        let result = insert_dependency(content, "bar = { path = \"../bar\" }");
        assert!(result.contains("bar = { path = \"../bar\" }"));
        assert!(result.contains("foo = { path = \"../foo\" }"));
        // bar should be before [profile.debug]
        let bar_pos = result.find("bar =").unwrap();
        let profile_pos = result.find("[profile.debug]").unwrap();
        assert!(bar_pos < profile_pos);
    }

    #[test]
    fn insert_with_no_deps_section() {
        let content = r#"[package]
name = "test"
version = "0.1.0"
"#;
        let result = insert_dependency(content, "foo = { path = \"../foo\" }");
        assert!(result.contains("[dependencies]"));
        assert!(result.contains("foo = { path = \"../foo\" }"));
    }

    #[test]
    fn insert_into_empty_deps() {
        let content = r#"[package]
name = "test"
version = "0.1.0"

[dependencies]
"#;
        let result = insert_dependency(content, "foo = { git = \"https://example.com/foo\" }");
        assert!(result.contains("foo = { git = \"https://example.com/foo\" }"));
    }
}
