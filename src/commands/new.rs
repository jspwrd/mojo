use anyhow::bail;
use std::fs;
use std::path::Path;

use crate::util;

pub fn exec(name: &str, lang: &str, lib: bool) -> anyhow::Result<()> {
    let dir = Path::new(name);
    if dir.exists() {
        bail!("directory '{}' already exists", name);
    }

    let std = default_std(lang);
    let pkg_type = if lib { "lib" } else { "bin" };

    fs::create_dir_all(dir.join("src"))?;
    fs::create_dir_all(dir.join("include"))?;

    let type_line = if lib {
        format!("type = \"lib\"\n")
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
        fs::write(dir.join("include").join(format!("{}.{}", name, header_ext(lang))), header_content)?;
        fs::write(dir.join("src").join(format!("{}.{}", name, ext)), src_content)?;
    } else {
        let (ext, main_content) = main_file(lang);
        fs::write(dir.join("src").join(format!("main.{}", ext)), main_content)?;
    }

    fs::write(dir.join(".gitignore"), "/build/\n/deps/\n")?;

    util::status("Created", &format!("{} {} `{}`", lang, pkg_type, name));
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
