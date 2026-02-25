use anyhow::bail;
use std::fs;

use crate::util;

pub fn exec(lang: &str, lib: bool) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;

    if cwd.join("Mojo.toml").exists() {
        bail!("Mojo.toml already exists in current directory");
    }

    let name = cwd
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "project".to_string());

    let std = default_std(lang);
    let pkg_type = if lib { "lib" } else { "bin" };

    fs::create_dir_all(cwd.join("src"))?;
    fs::create_dir_all(cwd.join("include"))?;

    let type_line = if lib {
        format!("type = \"lib\"\n")
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
        let header_path = cwd.join("include").join(format!("{}.{}", name, header_ext(lang)));
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

    if !cwd.join(".gitignore").exists() {
        fs::write(cwd.join(".gitignore"), "/build/\n/deps/\n")?;
    }

    util::status("Initialized", &format!("{} {} `{}`", lang, pkg_type, name));
    Ok(())
}

fn default_std(lang: &str) -> &str {
    match lang {
        "c" => "c11",
        _ => "c++17",
    }
}

fn header_ext(lang: &str) -> &str {
    match lang {
        "c" => "h",
        _ => "hpp",
    }
}

fn main_file(lang: &str) -> (&str, &str) {
    match lang {
        "c" => (
            "c",
            "#include <stdio.h>\n\nint main(void) {\n    printf(\"Hello, world!\\n\");\n    return 0;\n}\n",
        ),
        _ => (
            "cpp",
            "#include <iostream>\n\nint main() {\n    std::cout << \"Hello, world!\" << std::endl;\n    return 0;\n}\n",
        ),
    }
}

fn lib_files(name: &str, lang: &str) -> (&'static str, String, String) {
    match lang {
        "c" => (
            "c",
            format!(
                "#ifndef {guard}_H\n#define {guard}_H\n\nint {name}_add(int a, int b);\n\n#endif\n",
                guard = name.to_uppercase(),
                name = name,
            ),
            format!(
                "#include \"{name}.h\"\n\nint {name}_add(int a, int b) {{\n    return a + b;\n}}\n",
                name = name,
            ),
        ),
        _ => (
            "cpp",
            format!(
                "#pragma once\n\nnamespace {name} {{\n\nint add(int a, int b);\n\n}} // namespace {name}\n",
                name = name,
            ),
            format!(
                "#include \"{name}.hpp\"\n\nnamespace {name} {{\n\nint add(int a, int b) {{\n    return a + b;\n}}\n\n}} // namespace {name}\n",
                name = name,
            ),
        ),
    }
}
