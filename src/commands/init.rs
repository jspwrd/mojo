use anyhow::bail;
use std::fs;

use crate::cli::Framework;
use crate::config::validate_project_name;
use crate::frameworks::framework_config;
use crate::scaffold::{default_std, header_ext, lib_files, main_file, test_file};
use crate::util;

pub fn exec(lang: &str, lib: bool, framework: Option<Framework>) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;

    if cwd.join("Mojo.toml").exists() {
        bail!("Mojo.toml already exists in current directory");
    }

    let name = cwd
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "project".to_string());

    validate_project_name(&name)?;

    let pkg_type = if lib { "lib" } else { "bin" };

    fs::create_dir_all(cwd.join("src"))?;
    fs::create_dir_all(cwd.join("include"))?;
    fs::create_dir_all(cwd.join("tests"))?;

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
        fs::write(cwd.join("Mojo.toml"), toml_content)?;

        // Write framework main source
        if !lib {
            let main_path = cwd.join("src").join(format!("main.{}", cfg.src_ext));
            if !main_path.exists() {
                fs::write(&main_path, cfg.main_content)?;
            }
        } else {
            let header_path = cwd
                .join("include")
                .join(format!("{}.{}", name, header_ext(actual_lang)));
            if !header_path.exists() {
                let (_, header_content, _) = lib_files(&name, actual_lang);
                fs::write(&header_path, header_content)?;
            }
            let (ext, _, src_content) = lib_files(&name, actual_lang);
            let src_path = cwd.join("src").join(format!("{}.{}", name, ext));
            if !src_path.exists() {
                fs::write(&src_path, src_content)?;
            }
        }

        // Write extra framework files
        for (path, content) in cfg.extra_files {
            let file_path = cwd.join(path);
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent)?;
            }
            if !file_path.exists() {
                fs::write(&file_path, content)?;
            }
        }

        // Generate sample test file if tests/ is empty
        let test_dir = cwd.join("tests");
        if test_dir
            .read_dir()
            .map(|mut d| d.next().is_none())
            .unwrap_or(true)
        {
            let (test_ext, test_content) = test_file(&name, actual_lang);
            fs::write(
                test_dir.join(format!("test_basic.{}", test_ext)),
                test_content,
            )?;
        }

        if !cwd.join(".gitignore").exists() {
            fs::write(cwd.join(".gitignore"), "/build/\n/deps/\n")?;
        }

        util::status(
            "Initialized",
            &format!("{} {} `{}` with {} support", actual_lang, pkg_type, name, fw),
        );
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
            cwd.join("Mojo.toml"),
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
            let header_path = cwd
                .join("include")
                .join(format!("{}.{}", name, header_ext(lang)));
            if !header_path.exists() {
                let (_, header_content, _) = lib_files(&name, lang);
                fs::write(&header_path, header_content)?;
            }
            let (ext, _, src_content) = lib_files(&name, lang);
            let src_path = cwd.join("src").join(format!("{}.{}", name, ext));
            if !src_path.exists() {
                fs::write(&src_path, src_content)?;
            }
        } else {
            let (ext, main_content) = main_file(lang);
            let main_path = cwd.join("src").join(format!("main.{}", ext));
            if !main_path.exists() {
                fs::write(&main_path, main_content)?;
            }
        }

        let test_dir = cwd.join("tests");
        if test_dir
            .read_dir()
            .map(|mut d| d.next().is_none())
            .unwrap_or(true)
        {
            let (test_ext, test_content) = test_file(&name, lang);
            fs::write(
                test_dir.join(format!("test_basic.{}", test_ext)),
                test_content,
            )?;
        }

        if !cwd.join(".gitignore").exists() {
            fs::write(cwd.join(".gitignore"), "/build/\n/deps/\n")?;
        }

        util::status("Initialized", &format!("{} {} `{}`", lang, pkg_type, name));
    }

    Ok(())
}
