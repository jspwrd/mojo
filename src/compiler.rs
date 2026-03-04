use anyhow::{bail, Context};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config::{Profile, TargetConfig};

pub struct Compiler {
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
            _ => {
                // Try clang first, then gcc
                Self::find_clang().or_else(|_| Self::find_gcc()).with_context(|| {
                    let hint = if cfg!(target_os = "macos") {
                        ". Try: xcode-select --install"
                    } else {
                        ". Try: sudo apt install build-essential"
                    };
                    format!(
                        "could not find a {} compiler{}",
                        if lang == "c" { "C" } else { "C++" },
                        hint
                    )
                })
            }
        }
    }

    pub fn from_target(target_config: &TargetConfig, lang: &str) -> anyhow::Result<Self> {
        match (&target_config.cc, &target_config.cxx) {
            (Some(cc), Some(cxx)) => Ok(Self {
                c_path: which::which(cc).with_context(|| format!("{} not found", cc))?,
                cxx_path: which::which(cxx).with_context(|| format!("{} not found", cxx))?,
            }),
            (Some(cc), None) => {
                let c = which::which(cc).with_context(|| format!("{} not found", cc))?;
                Ok(Self {
                    c_path: c.clone(),
                    cxx_path: c,
                })
            }
            (None, Some(cxx)) => {
                let cxx = which::which(cxx).with_context(|| format!("{} not found", cxx))?;
                Ok(Self {
                    c_path: cxx.clone(),
                    cxx_path: cxx,
                })
            }
            (None, None) => Self::detect("auto", lang),
        }
    }

    fn find_clang() -> anyhow::Result<Self> {
        let c = which::which("clang").context("clang not found")?;
        let cxx = which::which("clang++").context("clang++ not found")?;
        Ok(Self {
            c_path: c,
            cxx_path: cxx,
        })
    }

    fn find_gcc() -> anyhow::Result<Self> {
        let c = which::which("gcc").context("gcc not found")?;
        let cxx = which::which("g++").context("g++ not found")?;
        Ok(Self {
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

    pub fn check(
        &self,
        source: &Path,
        lang: Language,
        flags: &[String],
    ) -> anyhow::Result<()> {
        let compiler = self.compiler_for(lang);

        if crate::util::is_verbose() {
            let cmd_str = format!(
                "{} -fsyntax-only {} {}",
                compiler.display(),
                source.display(),
                flags.join(" ")
            );
            crate::util::verbose("Running", &cmd_str);
        }

        let output = Command::new(compiler)
            .arg("-fsyntax-only")
            .arg(source)
            .args(flags)
            .output()
            .with_context(|| format!("failed to run {}", compiler.display()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let mut msg = format!("check failed for {}", source.display());
            if !stderr.is_empty() {
                msg.push_str(&format!("\n{}", stderr.trim_end()));
            }
            bail!("{}", msg);
        }

        // Forward any warnings from successful checks
        if !output.stderr.is_empty() {
            eprint!("{}", String::from_utf8_lossy(&output.stderr));
        }
        Ok(())
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

        if crate::util::is_verbose() {
            let cmd_str = format!(
                "{} -c {} -o {} {}",
                compiler.display(),
                source.display(),
                object.display(),
                flags.join(" ")
            );
            crate::util::verbose("Running", &cmd_str);
        }

        let output = Command::new(compiler)
            .arg("-c")
            .arg(source)
            .arg("-o")
            .arg(object)
            .args(flags)
            .output()
            .with_context(|| format!("failed to run {}", compiler.display()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let mut msg = format!("compilation failed for {}", source.display());
            if !stderr.is_empty() {
                msg.push_str(&format!("\n{}", stderr.trim_end()));
            }
            bail!("{}", msg);
        }

        // Forward any warnings from successful compilations
        if !output.stderr.is_empty() {
            eprint!("{}", String::from_utf8_lossy(&output.stderr));
        }
        Ok(())
    }

    pub fn link(&self, objects: &[PathBuf], output: &Path, flags: &[String], has_cpp: bool) -> anyhow::Result<()> {
        if let Some(parent) = output.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let linker = self.linker(has_cpp);

        if crate::util::is_verbose() {
            let objs: Vec<_> = objects.iter().map(|o| o.display().to_string()).collect();
            let cmd_str = format!(
                "{} {} -o {} {}",
                linker.display(),
                objs.join(" "),
                output.display(),
                flags.join(" ")
            );
            crate::util::verbose("Running", &cmd_str);
        }

        let cmd_output = Command::new(linker)
            .args(objects)
            .arg("-o")
            .arg(output)
            .args(flags)
            .output()
            .with_context(|| format!("failed to run linker {}", linker.display()))?;

        if !cmd_output.status.success() {
            let stderr = String::from_utf8_lossy(&cmd_output.stderr);
            let mut msg = "linking failed".to_string();
            if !stderr.is_empty() {
                msg.push_str(&format!("\n{}", stderr.trim_end()));
            }
            bail!("{}", msg);
        }

        // Forward any warnings
        if !cmd_output.stderr.is_empty() {
            eprint!("{}", String::from_utf8_lossy(&cmd_output.stderr));
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub fn archive(objects: &[PathBuf], output: &Path) -> anyhow::Result<()> {
        Self::archive_with(objects, output, None)
    }

    pub fn archive_with(objects: &[PathBuf], output: &Path, ar_path: Option<&str>) -> anyhow::Result<()> {
        if let Some(parent) = output.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let ar = match ar_path {
            Some(p) => which::which(p).with_context(|| format!("{} not found", p))?,
            None => which::which("ar").context("ar not found")?,
        };

        let cmd_output = Command::new(ar)
            .arg("rcs")
            .arg(output)
            .args(objects)
            .output()
            .context("failed to run ar")?;

        if !cmd_output.status.success() {
            let stderr = String::from_utf8_lossy(&cmd_output.stderr);
            let mut msg = format!("archiving failed for {}", output.display());
            if !stderr.is_empty() {
                msg.push_str(&format!("\n{}", stderr.trim_end()));
            }
            bail!("{}", msg);
        }
        Ok(())
    }
}

pub fn build_compile_flags(
    profile: &Profile,
    std: Option<&str>,
    include_paths: &[PathBuf],
    pic: bool,
    extra_cflags: &[String],
    sanitizers: &[String],
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

    for san in sanitizers {
        flags.push(format!("-fsanitize={}", san));
    }

    flags.extend_from_slice(extra_cflags);

    flags
}

pub fn build_link_flags(
    profile: &Profile,
    extra_ldflags: &[String],
    libs: &[String],
    sanitizers: &[String],
) -> Vec<String> {
    let mut flags = Vec::new();
    if profile.lto {
        flags.push("-flto".to_string());
    }
    for san in sanitizers {
        flags.push(format!("-fsanitize={}", san));
    }
    flags.extend_from_slice(extra_ldflags);
    for lib in libs {
        flags.push(format!("-l{}", lib));
    }
    flags
}
