use anyhow::bail;
use std::fs;
use std::path::Path;

use crate::config::validate_project_name;
use crate::scaffold::{default_std, header_ext, lib_files, main_file};
use crate::util;

pub fn exec(name: &str, lang: &str, lib: bool) -> anyhow::Result<()> {
    validate_project_name(name)?;

    let dir = Path::new(name);
    if dir.exists() {
        bail!("directory '{}' already exists", name);
    }

    let std = default_std(lang);
    let pkg_type = if lib { "lib" } else { "bin" };

    fs::create_dir_all(dir.join("src"))?;
    fs::create_dir_all(dir.join("include"))?;

    let type_line = if lib {
        "type = \"lib\"\n".to_string()
    } else {
        String::new()
    };

    fs::write(
        dir.join("Mojo.toml"),
        format!(
            r#"[package]
name = "{name}"
version = "0.1.0"
lang = "{lang}"
std = "{std}"
{type_line}
[build]
compiler = "auto"

[dependencies]
"#
        ),
    )?;

    if lib {
        let (ext, header_content, src_content) = lib_files(name, lang);
        fs::write(
            dir.join("include").join(format!("{}.{}", name, header_ext(lang))),
            header_content,
        )?;
        fs::write(
            dir.join("src").join(format!("{}.{}", name, ext)),
            src_content,
        )?;
    } else {
        let (ext, main_content) = main_file(lang);
        fs::write(dir.join("src").join(format!("main.{}", ext)), main_content)?;
    }

    fs::write(dir.join(".gitignore"), "/build/\n/deps/\n")?;

    util::status("Created", &format!("{} {} `{}`", lang, pkg_type, name));
    Ok(())
}
