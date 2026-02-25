use anyhow::{bail, Context};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config::Profile;

#[derive(Debug, Clone, Copy)]
pub enum CompilerFamily {
    Gcc,
    Clang,
}

pub struct Compiler {
    pub family: CompilerFamily,
    pub c_path: PathBuf,
    pub cxx_path: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Language {
    C,
    Cpp,
}

impl Language {
    pub fn from_extension(ext: &str) -> Option<Language> {
        match ext {
            "c" => Some(Language::C),
            "cpp" | "cxx" | "cc" => Some(Language::Cpp),
            _ => None,
        }
    }
}

impl Compiler {
    pub fn detect(compiler_pref: &str, lang: &str) -> anyhow::Result<Self> {
        match compiler_pref {
            "clang" => Self::find_clang(),
            "gcc" => Self::find_gcc(),
            "auto" | _ => {
                // Try clang first, then gcc
                Self::find_clang().or_else(|_| Self::find_gcc()).with_context(|| {
                    format!(
                        "could not find a {} compiler. Install gcc or clang",
                        if lang == "c" { "C" } else { "C++" }
                    )
                })
            }
        }
    }

    fn find_clang() -> anyhow::Result<Self> {
        let c = which::which("clang").context("clang not found")?;
        let cxx = which::which("clang++").context("clang++ not found")?;
        Ok(Self {
            family: CompilerFamily::Clang,
            c_path: c,
            cxx_path: cxx,
        })
    }

    fn find_gcc() -> anyhow::Result<Self> {
        let c = which::which("gcc").context("gcc not found")?;
        let cxx = which::which("g++").context("g++ not found")?;
        Ok(Self {
            family: CompilerFamily::Gcc,
            c_path: c,
            cxx_path: cxx,
        })
    }

    pub fn compiler_for(&self, lang: Language) -> &Path {
        match lang {
            Language::C => &self.c_path,
            Language::Cpp => &self.cxx_path,
        }
    }

    /// Returns the linker path — always use C++ compiler if any C++ sources exist
    pub fn linker(&self, has_cpp: bool) -> &Path {
        if has_cpp {
            &self.cxx_path
        } else {
            &self.c_path
        }
    }

    pub fn compile(
        &self,
        source: &Path,
        object: &Path,
        lang: Language,
        flags: &[String],
    ) -> anyhow::Result<()> {
        if let Some(parent) = object.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let compiler = self.compiler_for(lang);
        let status = Command::new(compiler)
            .arg("-c")
            .arg(source)
            .arg("-o")
            .arg(object)
            .args(flags)
            .status()
            .with_context(|| format!("failed to run {}", compiler.display()))?;

        if !status.success() {
            bail!("compilation failed for {}", source.display());
        }
        Ok(())
    }

    pub fn link(&self, objects: &[PathBuf], output: &Path, flags: &[String], has_cpp: bool) -> anyhow::Result<()> {
        if let Some(parent) = output.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let linker = self.linker(has_cpp);
        let status = Command::new(linker)
            .args(objects)
            .arg("-o")
            .arg(output)
            .args(flags)
            .status()
            .with_context(|| format!("failed to run linker {}", linker.display()))?;

        if !status.success() {
            bail!("linking failed");
        }
        Ok(())
    }

    pub fn archive(objects: &[PathBuf], output: &Path) -> anyhow::Result<()> {
        if let Some(parent) = output.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let ar = which::which("ar").context("ar not found")?;
        let status = Command::new(ar)
            .arg("rcs")
            .arg(output)
            .args(objects)
            .status()
            .context("failed to run ar")?;

        if !status.success() {
            bail!("archiving failed for {}", output.display());
        }
        Ok(())
    }
}

pub fn build_compile_flags(
    profile: &Profile,
    std: Option<&str>,
    include_paths: &[PathBuf],
    pic: bool,
) -> Vec<String> {
    let mut flags = Vec::new();

    flags.push(format!("-O{}", profile.opt_level));

    if profile.debug {
        flags.push("-g".to_string());
    }

    if let Some(std) = std {
        flags.push(format!("-std={}", std));
    }

    if pic {
        flags.push("-fPIC".to_string());
    }

    for path in include_paths {
        flags.push(format!("-I{}", path.display()));
    }

    flags.push("-Wall".to_string());
    flags.push("-Wextra".to_string());

    if profile.lto {
        flags.push("-flto".to_string());
    }

    flags
}

pub fn build_link_flags(profile: &Profile) -> Vec<String> {
    let mut flags = Vec::new();
    if profile.lto {
        flags.push("-flto".to_string());
    }
    flags
}
