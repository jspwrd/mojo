use anyhow::bail;
use std::fs;
use std::path::Path;

use crate::cli::Framework;
use crate::config::validate_project_name;
use crate::frameworks::framework_config;
use crate::scaffold::{default_std, header_ext, lib_files, main_file, test_file};
use crate::util;

pub fn exec(name: &str, lang: &str, lib: bool, framework: Option<Framework>) -> anyhow::Result<()> {
    validate_project_name(name)?;

    let dir = Path::new(name);
    if dir.exists() {
        bail!("directory '{}' already exists", name);
    }

    let pkg_type = if lib { "lib" } else { "bin" };

    fs::create_dir_all(dir.join("src"))?;
    fs::create_dir_all(dir.join("include"))?;
    fs::create_dir_all(dir.join("tests"))?;

    if let Some(fw) = framework {
        let cfg = framework_config(fw);
        let actual_lang = if cfg.force_lang.is_empty() { lang } else { cfg.force_lang };
        let actual_std = if cfg.force_std.is_empty() {
            default_std(actual_lang).to_string()
        } else {
            cfg.force_std.to_string()
        };

        let type_line = if lib {
            "type = \"lib\"\n".to_string()
        } else {
            String::new()
        };

        let toml_content = format!(
            r#"[package]
name = "{name}"
version = "0.1.0"
lang = "{lang}"
std = "{std}"
{type_line}
[build]
compiler = "auto"
{build_extra}
[dependencies]
{extra}"#,
            name = name,
            lang = actual_lang,
            std = actual_std,
            type_line = type_line,
            build_extra = cfg.build_toml,
            extra = cfg.extra_toml,
        );
        fs::write(dir.join("Mojo.toml"), toml_content)?;

        // Write framework main source
        if !lib {
            fs::write(
                dir.join("src").join(format!("main.{}", cfg.src_ext)),
                cfg.main_content,
            )?;
        } else {
            let (ext, header_content, src_content) = lib_files(name, actual_lang);
            fs::write(
                dir.join("include").join(format!("{}.{}", name, header_ext(actual_lang))),
                header_content,
            )?;
            fs::write(
                dir.join("src").join(format!("{}.{}", name, ext)),
                src_content,
            )?;
        }

        // Write extra framework files
        for (path, content) in cfg.extra_files {
            let file_path = dir.join(path);
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(file_path, content)?;
        }

        // Generate sample test file
        let (test_ext, test_content) = test_file(name, actual_lang);
        fs::write(
            dir.join("tests").join(format!("test_basic.{}", test_ext)),
            test_content,
        )?;

        fs::write(dir.join(".gitignore"), "/build/\n/deps/\n")?;

        util::status("Created", &format!("{} {} `{}` with {} support", actual_lang, pkg_type, name, fw));
        util::status("Hint", cfg.hint);
    } else {
        // Default path (no framework)
        let std = default_std(lang);
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

        let (test_ext, test_content) = test_file(name, lang);
        fs::write(
            dir.join("tests").join(format!("test_basic.{}", test_ext)),
            test_content,
        )?;

        fs::write(dir.join(".gitignore"), "/build/\n/deps/\n")?;

        util::status("Created", &format!("{} {} `{}`", lang, pkg_type, name));
    }

    Ok(())
}
