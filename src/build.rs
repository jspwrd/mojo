use anyhow::{anyhow, bail, Context};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Instant;

use crate::compiler::{build_compile_flags, build_link_flags, Compiler, Language};
use crate::deps::{resolve_dependencies, ResolvedDep};
use crate::incremental::FreshnessChecker;
use crate::project::Project;
use crate::util;

pub struct BuildResult {
    /// Primary output path (executable, or static lib for lib projects)
    pub output: PathBuf,
}

pub fn build(
    project: &Project,
    release: bool,
    jobs: Option<usize>,
) -> anyhow::Result<BuildResult> {
    let start = Instant::now();
    let profile_name = if release { "release" } else { "debug" };
    let profile = project.config.profile(profile_name);

    let num_jobs = jobs
        .or(project.config.build.jobs)
        .unwrap_or_else(num_cpus::get)
        .max(1);

    let compiler = Compiler::detect(
        &project.config.build.compiler,
        &project.config.package.lang,
    )?;

    // Resolve dependencies
    let deps = resolve_dependencies(project)?;

    // Collect include paths
    let mut include_paths: Vec<PathBuf> = Vec::new();
    let proj_include = project.include_dir();
    if proj_include.exists() {
        include_paths.push(proj_include);
    }
    for dep in &deps {
        include_paths.push(dep.include_path.clone());
    }

    // Freshness checker
    let checker = FreshnessChecker::new(&include_paths);

    // Compile flags
    let needs_pic = project.config.is_lib()
        && matches!(project.config.package.lib_type.as_str(), "shared" | "both");
    let flags = build_compile_flags(
        &profile,
        project.config.package.std.as_deref(),
        &include_paths,
        needs_pic,
        &project.config.build.cflags,
    );

    // Build dependencies first
    let mut dep_archives: Vec<PathBuf> = Vec::new();
    for dep in &deps {
        if dep.sources.is_empty() {
            continue; // header-only dep
        }
        let archive =
            build_dependency(project, dep, profile_name, &compiler, &flags, &checker, num_jobs)?;
        dep_archives.push(archive);
    }

    // Collect project sources
    let sources = collect_sources(&project.src_dir())?;
    if sources.is_empty() {
        bail!("no source files found in {}", project.src_dir().display());
    }

    util::status(
        "Compiling",
        &format!(
            "{} v{} ({})",
            project.config.package.name, project.config.package.version, profile_name
        ),
    );

    // Determine which sources need compilation
    let obj_dir = project.obj_dir(profile_name);
    let mut has_cpp = project.config.package.lang == "c++";
    let mut all_objects: Vec<PathBuf> = Vec::new();
    let mut stale: Vec<CompileJob> = Vec::new();

    for source in &sources {
        let ext = source.extension().and_then(|e| e.to_str()).unwrap_or("");
        let lang = Language::from_extension(ext)
            .ok_or_else(|| anyhow!("unsupported source file extension: {}", source.display()))?;
        if lang == Language::Cpp {
            has_cpp = true;
        }

        let obj_path = source_to_object(source, &project.src_dir(), &obj_dir);

        if !checker.is_fresh(source, &obj_path) {
            stale.push(CompileJob {
                source: source.clone(),
                object: obj_path.clone(),
                lang,
            });
        }

        all_objects.push(obj_path);
    }

    let compiled_count = stale.len();

    // Compile in parallel
    if !stale.is_empty() {
        compile_parallel(&stale, &compiler, &flags, num_jobs)?;
    }

    let is_lib = project.config.is_lib();
    let build_dir = project.build_dir(profile_name);

    let output = if is_lib {
        let lib_type = &project.config.package.lib_type;
        build_lib(
            &project.config.package.name,
            lib_type,
            &all_objects,
            &dep_archives,
            &build_dir,
            &compiler,
            &build_link_flags(
                &profile,
                &project.config.build.ldflags,
                &project.config.build.libs,
            ),
            has_cpp,
            compiled_count,
        )?
    } else {
        all_objects.extend(dep_archives);
        let output = build_dir.join(&project.config.package.name);
        let link_flags = build_link_flags(
            &profile,
            &project.config.build.ldflags,
            &project.config.build.libs,
        );

        if compiled_count > 0 || !output.exists() {
            util::status("Linking", &project.config.package.name);
            compiler.link(&all_objects, &output, &link_flags, has_cpp)?;
        }
        output
    };

    let elapsed = start.elapsed();
    util::status(
        "Finished",
        &format!("{} target in {:.2}s", profile_name, elapsed.as_secs_f64()),
    );

    Ok(BuildResult { output })
}

struct CompileJob {
    source: PathBuf,
    object: PathBuf,
    lang: Language,
}

fn compile_parallel(
    jobs: &[CompileJob],
    compiler: &Compiler,
    flags: &[String],
    num_threads: usize,
) -> anyhow::Result<()> {
    if num_threads <= 1 || jobs.len() <= 1 {
        // Sequential fallback
        for job in jobs {
            compiler
                .compile(&job.source, &job.object, job.lang, flags)
                .with_context(|| format!("failed to compile {}", job.source.display()))?;
        }
        return Ok(());
    }

    let idx = AtomicUsize::new(0);
    let errors: Mutex<Vec<String>> = Mutex::new(Vec::new());

    std::thread::scope(|s| {
        for _ in 0..num_threads.min(jobs.len()) {
            s.spawn(|| {
                loop {
                    let i = idx.fetch_add(1, Ordering::Relaxed);
                    if i >= jobs.len() {
                        break;
                    }
                    let job = &jobs[i];
                    if let Err(e) = compiler.compile(&job.source, &job.object, job.lang, flags) {
                        let mut errs = errors.lock().unwrap();
                        errs.push(format!("{}: {:#}", job.source.display(), e));
                    }
                }
            });
        }
    });

    let errs = errors.into_inner().unwrap();
    if !errs.is_empty() {
        bail!("compilation failed:\n{}", errs.join("\n"));
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn build_lib(
    name: &str,
    lib_type: &str,
    objects: &[PathBuf],
    dep_archives: &[PathBuf],
    build_dir: &Path,
    compiler: &Compiler,
    link_flags: &[String],
    has_cpp: bool,
    compiled_count: usize,
) -> anyhow::Result<PathBuf> {
    let static_path = build_dir.join(format!("lib{}.a", name));
    let shared_path = build_dir.join(shared_lib_name(name));

    let needs_static = lib_type == "static" || lib_type == "both";
    let needs_shared = lib_type == "shared" || lib_type == "both";

    if needs_static && (compiled_count > 0 || !static_path.exists()) {
        util::status("Archiving", &format!("lib{}.a", name));
        let mut all = objects.to_vec();
        all.extend_from_slice(dep_archives);
        Compiler::archive(&all, &static_path)?;
    }

    if needs_shared && (compiled_count > 0 || !shared_path.exists()) {
        util::status("Linking", &format!("{} (shared)", name));
        let mut all = objects.to_vec();
        all.extend_from_slice(dep_archives);
        let mut flags = vec!["-shared".to_string()];
        flags.extend_from_slice(link_flags);
        compiler.link(&all, &shared_path, &flags, has_cpp)?;
    }

    if needs_static {
        Ok(static_path)
    } else {
        Ok(shared_path)
    }
}

fn shared_lib_name(name: &str) -> String {
    if cfg!(target_os = "macos") {
        format!("lib{}.dylib", name)
    } else {
        format!("lib{}.so", name)
    }
}

fn build_dependency(
    project: &Project,
    dep: &ResolvedDep,
    profile_name: &str,
    compiler: &Compiler,
    flags: &[String],
    checker: &FreshnessChecker,
    num_jobs: usize,
) -> anyhow::Result<PathBuf> {
    let dep_build_dir = project.deps_build_dir(profile_name).join(&dep.name);
    let dep_obj_dir = dep_build_dir.join("obj");
    let archive_path = dep_build_dir.join(format!("lib{}.a", dep.name));

    let src_base = if dep.root.join("src").exists() {
        dep.root.join("src")
    } else {
        dep.root.clone()
    };

    let mut all_objects: Vec<PathBuf> = Vec::new();
    let mut stale: Vec<CompileJob> = Vec::new();

    for source in &dep.sources {
        let ext = source.extension().and_then(|e| e.to_str()).unwrap_or("");
        let lang = Language::from_extension(ext)
            .ok_or_else(|| anyhow!("unsupported source file extension: {}", source.display()))?;
        let obj_path = source_to_object(source, &src_base, &dep_obj_dir);

        if !checker.is_fresh(source, &obj_path) {
            let file_name = source
                .file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_else(|| source.display().to_string());
            util::status("Compiling", &format!("{} ({})", dep.name, file_name));
            stale.push(CompileJob {
                source: source.clone(),
                object: obj_path.clone(),
                lang,
            });
        }

        all_objects.push(obj_path);
    }

    if !stale.is_empty() {
        compile_parallel(&stale, compiler, flags, num_jobs)?;
    }

    if !stale.is_empty() || !archive_path.exists() {
        util::status("Archiving", &format!("lib{}.a", dep.name));
        Compiler::archive(&all_objects, &archive_path)?;
    }

    Ok(archive_path)
}

fn collect_sources(dir: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let mut sources = Vec::new();
    if !dir.exists() {
        return Ok(sources);
    }

    for entry in walkdir::WalkDir::new(dir) {
        let entry = entry?;
        if entry.file_type().is_file()
            && let Some(ext) = entry.path().extension().and_then(|e| e.to_str())
            && Language::from_extension(ext).is_some()
        {
            sources.push(entry.into_path());
        }
    }

    sources.sort();
    Ok(sources)
}

/// Convert a source file path to an object file path.
/// e.g., src/net/socket.cpp -> obj/net__socket.o
fn source_to_object(source: &Path, src_base: &Path, obj_dir: &Path) -> PathBuf {
    let relative = source.strip_prefix(src_base).unwrap_or(source);
    let stem: String = relative
        .components()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join("__");

    let stem = stem
        .trim_end_matches(".c")
        .trim_end_matches(".cpp")
        .trim_end_matches(".cxx")
        .trim_end_matches(".cc");

    obj_dir.join(format!("{}.o", stem))
}
