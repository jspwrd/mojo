use anyhow::{anyhow, bail, Context};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Instant;

use crate::compiler::{build_compile_flags, build_link_flags, Compiler, Language};
use crate::config::validate_sanitizers;
use crate::deps::{resolve_dependencies, ResolvedDep};
use crate::incremental::FreshnessChecker;
use crate::project::Project;
use crate::util;

pub struct BuildResult {
    /// Primary output path (executable, or static lib for lib projects)
    pub output: PathBuf,
}

#[allow(dead_code)]
pub struct TestResult {
    pub passed: usize,
    pub failed: usize,
    pub failures: Vec<String>,
}

pub fn build(
    project: &Project,
    profile_name: &str,
    jobs: Option<usize>,
    sanitizers: &[String],
    target: Option<&str>,
) -> anyhow::Result<BuildResult> {
    let start = Instant::now();
    let profile = project.config.profile(profile_name);

    // Run pre_build script
    if let Some(ref script) = project.config.scripts.pre_build {
        run_script("pre_build", script, &project.root)?;
    }

    let num_jobs = jobs
        .or(project.config.build.jobs)
        .unwrap_or_else(num_cpus::get)
        .max(1);

    let merged_sanitizers = merge_sanitizers(&project.config.build.sanitizers, sanitizers);
    validate_sanitizers(&merged_sanitizers)?;

    let (compiler, ar_override) = resolve_compiler(project, target)?;

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
    let extra_cflags = merge_target_cflags(&project.config, target);
    let flags = build_compile_flags(
        &profile,
        project.config.package.std.as_deref(),
        &include_paths,
        needs_pic,
        &extra_cflags,
        &merged_sanitizers,
    );

    // Build dependencies first
    let mut dep_archives: Vec<PathBuf> = Vec::new();
    for dep in &deps {
        if dep.sources.is_empty() {
            continue; // header-only dep
        }
        let archive = build_dependency(
            project,
            dep,
            profile_name,
            &compiler,
            &flags,
            &checker,
            num_jobs,
            target,
            ar_override.as_deref(),
        )?;
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
    let obj_dir = project.obj_dir(profile_name, target);
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
    let build_dir = project.build_dir(profile_name, target);

    let extra_ldflags = merge_target_ldflags(&project.config, target);
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
                &extra_ldflags,
                &project.config.build.libs,
                &merged_sanitizers,
            ),
            has_cpp,
            compiled_count,
            ar_override.as_deref(),
        )?
    } else {
        all_objects.extend(dep_archives);
        let output = build_dir.join(&project.config.package.name);
        let link_flags = build_link_flags(
            &profile,
            &extra_ldflags,
            &project.config.build.libs,
            &merged_sanitizers,
        );

        if compiled_count > 0 || !output.exists() {
            util::status("Linking", &project.config.package.name);
            compiler.link(&all_objects, &output, &link_flags, has_cpp)?;
        }
        output
    };

    // Run post_build script
    if let Some(ref script) = project.config.scripts.post_build {
        run_script("post_build", script, &project.root)?;
    }

    let elapsed = start.elapsed();
    util::status(
        "Finished",
        &format!("{} target in {:.2}s", profile_name, elapsed.as_secs_f64()),
    );

    Ok(BuildResult { output })
}

pub fn check(
    project: &Project,
    profile_name: &str,
    jobs: Option<usize>,
    sanitizers: &[String],
    target: Option<&str>,
) -> anyhow::Result<()> {
    let start = Instant::now();
    let profile = project.config.profile(profile_name);

    let num_jobs = jobs
        .or(project.config.build.jobs)
        .unwrap_or_else(num_cpus::get)
        .max(1);

    let merged_sanitizers = merge_sanitizers(&project.config.build.sanitizers, sanitizers);
    validate_sanitizers(&merged_sanitizers)?;

    let (compiler, _ar_override) = resolve_compiler(project, target)?;

    // Resolve dependencies (for include paths)
    let deps = resolve_dependencies(project)?;

    let mut include_paths: Vec<PathBuf> = Vec::new();
    let proj_include = project.include_dir();
    if proj_include.exists() {
        include_paths.push(proj_include);
    }
    for dep in &deps {
        include_paths.push(dep.include_path.clone());
    }

    let needs_pic = project.config.is_lib()
        && matches!(project.config.package.lib_type.as_str(), "shared" | "both");
    let extra_cflags = merge_target_cflags(&project.config, target);
    let flags = build_compile_flags(
        &profile,
        project.config.package.std.as_deref(),
        &include_paths,
        needs_pic,
        &extra_cflags,
        &merged_sanitizers,
    );

    let sources = collect_sources(&project.src_dir())?;
    if sources.is_empty() {
        bail!("no source files found in {}", project.src_dir().display());
    }

    util::status(
        "Checking",
        &format!(
            "{} v{} ({})",
            project.config.package.name, project.config.package.version, profile_name
        ),
    );

    let mut check_jobs: Vec<CheckJob> = Vec::new();
    for source in &sources {
        let ext = source.extension().and_then(|e| e.to_str()).unwrap_or("");
        let lang = Language::from_extension(ext)
            .ok_or_else(|| anyhow!("unsupported source file extension: {}", source.display()))?;
        check_jobs.push(CheckJob {
            source: source.clone(),
            lang,
        });
    }

    check_parallel(&check_jobs, &compiler, &flags, num_jobs)?;

    let elapsed = start.elapsed();
    util::status(
        "Finished",
        &format!("{} check in {:.2}s", profile_name, elapsed.as_secs_f64()),
    );

    Ok(())
}

pub fn test(
    project: &Project,
    profile_name: &str,
    jobs: Option<usize>,
    sanitizers: &[String],
    filter: Option<&str>,
    target: Option<&str>,
) -> anyhow::Result<TestResult> {
    let start = Instant::now();
    let profile = project.config.profile(profile_name);

    let test_dir = project.test_dir();
    if !test_dir.exists() {
        bail!("no tests directory found at {}", test_dir.display());
    }

    let mut test_sources = collect_sources(&test_dir)?;
    if test_sources.is_empty() {
        bail!("no test files found in {}", test_dir.display());
    }

    // Apply filter
    if let Some(filter) = filter {
        test_sources.retain(|s| {
            s.file_stem()
                .map(|n| n.to_string_lossy().contains(filter))
                .unwrap_or(false)
        });
        if test_sources.is_empty() {
            bail!("no test files match filter '{}'", filter);
        }
    }

    let num_jobs = jobs
        .or(project.config.build.jobs)
        .unwrap_or_else(num_cpus::get)
        .max(1);

    let merged_sanitizers = merge_sanitizers(&project.config.build.sanitizers, sanitizers);
    validate_sanitizers(&merged_sanitizers)?;

    let (compiler, ar_override) = resolve_compiler(project, target)?;

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

    let checker = FreshnessChecker::new(&include_paths);

    let extra_cflags = merge_target_cflags(&project.config, target);
    let flags = build_compile_flags(
        &profile,
        project.config.package.std.as_deref(),
        &include_paths,
        false, // tests are always executables, no PIC needed
        &extra_cflags,
        &merged_sanitizers,
    );

    let build_dir = project.build_dir(profile_name, target);
    let obj_dir = project.obj_dir(profile_name, target);
    let test_obj_dir = build_dir.join("test_obj");
    let test_bin_dir = build_dir.join("tests");

    // Build dependency archives
    let mut dep_archives: Vec<PathBuf> = Vec::new();
    for dep in &deps {
        if dep.sources.is_empty() {
            continue;
        }
        let archive = build_dependency(
            project,
            dep,
            profile_name,
            &compiler,
            &flags,
            &checker,
            num_jobs,
            target,
            ar_override.as_deref(),
        )?;
        dep_archives.push(archive);
    }

    // Build project objects to link tests against
    let has_cpp = project.config.package.lang == "c++";
    let link_objects: Vec<PathBuf> = if project.config.is_lib() {
        // Build library archive, link tests against it
        let sources = collect_sources(&project.src_dir())?;
        let mut all_objects = Vec::new();
        let mut stale = Vec::new();

        for source in &sources {
            let ext = source.extension().and_then(|e| e.to_str()).unwrap_or("");
            let lang = Language::from_extension(ext)
                .ok_or_else(|| anyhow!("unsupported extension: {}", source.display()))?;
            let obj = source_to_object(source, &project.src_dir(), &obj_dir);
            if !checker.is_fresh(source, &obj) {
                stale.push(CompileJob {
                    source: source.clone(),
                    object: obj.clone(),
                    lang,
                });
            }
            all_objects.push(obj);
        }

        if !stale.is_empty() {
            util::status("Compiling", &format!("{} (lib)", project.config.package.name));
            compile_parallel(&stale, &compiler, &flags, num_jobs)?;
        }

        let archive = build_dir.join(format!("lib{}.a", project.config.package.name));
        if !stale.is_empty() || !archive.exists() {
            Compiler::archive_with(&all_objects, &archive, ar_override.as_deref())?;
        }
        vec![archive]
    } else {
        // Bin project: compile all src/ except the file with main()
        let sources = collect_sources(&project.src_dir())?;
        let mut objects = Vec::new();
        let mut stale = Vec::new();

        for source in &sources {
            if source_has_main(source) {
                continue;
            }
            let ext = source.extension().and_then(|e| e.to_str()).unwrap_or("");
            let lang = Language::from_extension(ext)
                .ok_or_else(|| anyhow!("unsupported extension: {}", source.display()))?;
            let obj = source_to_object(source, &project.src_dir(), &obj_dir);
            if !checker.is_fresh(source, &obj) {
                stale.push(CompileJob {
                    source: source.clone(),
                    object: obj.clone(),
                    lang,
                });
            }
            objects.push(obj);
        }

        if !stale.is_empty() {
            util::status(
                "Compiling",
                &format!("{} (sources)", project.config.package.name),
            );
            compile_parallel(&stale, &compiler, &flags, num_jobs)?;
        }

        objects
    };

    // Compile and run each test
    util::status(
        "Testing",
        &format!(
            "{} v{} ({})",
            project.config.package.name, project.config.package.version, profile_name
        ),
    );

    let extra_ldflags = merge_target_ldflags(&project.config, target);
    let link_flags = build_link_flags(
        &profile,
        &extra_ldflags,
        &project.config.build.libs,
        &merged_sanitizers,
    );

    let mut passed = 0usize;
    let mut failed = 0usize;
    let mut failures = Vec::new();

    for test_source in &test_sources {
        let test_name = test_source
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .to_string();

        // Compile test
        let ext = test_source
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        let lang = Language::from_extension(ext)
            .ok_or_else(|| anyhow!("unsupported extension: {}", test_source.display()))?;
        let test_obj = test_obj_dir.join(format!("{}.o", test_name));
        compiler.compile(test_source, &test_obj, lang, &flags)?;

        // Link test
        let test_bin = test_bin_dir.join(&test_name);
        let mut test_link_objects = vec![test_obj];
        test_link_objects.extend(link_objects.clone());
        test_link_objects.extend(dep_archives.clone());
        compiler.link(&test_link_objects, &test_bin, &link_flags, has_cpp)?;

        // Run test
        let status = std::process::Command::new(&test_bin).status();

        match status {
            Ok(s) if s.success() => {
                util::pass(&test_name);
                passed += 1;
            }
            _ => {
                util::fail(&test_name);
                failed += 1;
                failures.push(test_name);
            }
        }
    }

    let elapsed = start.elapsed();
    println!();
    util::status(
        "Finished",
        &format!(
            "testing in {:.2}s: {} passed, {} failed",
            elapsed.as_secs_f64(),
            passed,
            failed
        ),
    );

    Ok(TestResult {
        passed,
        failed,
        failures,
    })
}

// --- Helpers ---

fn merge_sanitizers(config_sans: &[String], cli_sans: &[String]) -> Vec<String> {
    let mut all: Vec<String> = config_sans.to_vec();
    for s in cli_sans {
        if !all.contains(s) {
            all.push(s.clone());
        }
    }
    all
}

fn resolve_compiler(
    project: &Project,
    target: Option<&str>,
) -> anyhow::Result<(Compiler, Option<String>)> {
    if let Some(triple) = target {
        if let Some(tc) = project.config.target.get(triple) {
            let compiler = Compiler::from_target(tc, &project.config.package.lang)?;
            return Ok((compiler, tc.ar.clone()));
        }
    }
    let compiler = Compiler::detect(
        &project.config.build.compiler,
        &project.config.package.lang,
    )?;
    Ok((compiler, None))
}

fn merge_target_cflags(
    config: &crate::config::MojoConfig,
    target: Option<&str>,
) -> Vec<String> {
    let mut cflags = config.build.cflags.clone();
    if let Some(triple) = target {
        if let Some(tc) = config.target.get(triple) {
            cflags.extend(tc.cflags.clone());
        }
    }
    cflags
}

fn merge_target_ldflags(
    config: &crate::config::MojoConfig,
    target: Option<&str>,
) -> Vec<String> {
    let mut ldflags = config.build.ldflags.clone();
    if let Some(triple) = target {
        if let Some(tc) = config.target.get(triple) {
            ldflags.extend(tc.ldflags.clone());
        }
    }
    ldflags
}

fn run_script(label: &str, script: &str, project_root: &Path) -> anyhow::Result<()> {
    util::status("Running", &format!("{} script", label));
    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(script)
        .current_dir(project_root)
        .output()
        .with_context(|| format!("failed to run {} script", label))?;

    if !output.stdout.is_empty() {
        print!("{}", String::from_utf8_lossy(&output.stdout));
    }

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let mut msg = format!("{} script failed", label);
        if !stderr.is_empty() {
            msg.push_str(&format!("\n{}", stderr.trim_end()));
        }
        bail!("{}", msg);
    }

    Ok(())
}

/// Heuristic: scan source file for "int main" or "void main"
fn source_has_main(path: &Path) -> bool {
    let Ok(content) = std::fs::read_to_string(path) else {
        return false;
    };
    content.contains("int main") || content.contains("void main")
}

struct CheckJob {
    source: PathBuf,
    lang: Language,
}

fn check_parallel(
    jobs: &[CheckJob],
    compiler: &Compiler,
    flags: &[String],
    num_threads: usize,
) -> anyhow::Result<()> {
    if num_threads <= 1 || jobs.len() <= 1 {
        for job in jobs {
            compiler
                .check(&job.source, job.lang, flags)
                .with_context(|| format!("failed to check {}", job.source.display()))?;
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
                    if let Err(e) = compiler.check(&job.source, job.lang, flags) {
                        let mut errs = errors.lock().unwrap();
                        errs.push(format!("{:#}", e));
                    }
                }
            });
        }
    });

    let errs = errors.into_inner().unwrap();
    if !errs.is_empty() {
        bail!("check failed:\n{}", errs.join("\n"));
    }

    Ok(())
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
    ar_override: Option<&str>,
) -> anyhow::Result<PathBuf> {
    let static_path = build_dir.join(format!("lib{}.a", name));
    let shared_path = build_dir.join(shared_lib_name(name));

    let needs_static = lib_type == "static" || lib_type == "both";
    let needs_shared = lib_type == "shared" || lib_type == "both";

    if needs_static && (compiled_count > 0 || !static_path.exists()) {
        util::status("Archiving", &format!("lib{}.a", name));
        let mut all = objects.to_vec();
        all.extend_from_slice(dep_archives);
        Compiler::archive_with(&all, &static_path, ar_override)?;
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

#[allow(clippy::too_many_arguments)]
fn build_dependency(
    project: &Project,
    dep: &ResolvedDep,
    profile_name: &str,
    compiler: &Compiler,
    flags: &[String],
    checker: &FreshnessChecker,
    num_jobs: usize,
    target: Option<&str>,
    ar_override: Option<&str>,
) -> anyhow::Result<PathBuf> {
    let dep_build_dir = project
        .deps_build_dir(profile_name, target)
        .join(&dep.name);
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
        Compiler::archive_with(&all_objects, &archive_path, ar_override)?;
    }

    Ok(archive_path)
}

pub fn collect_sources(dir: &Path) -> anyhow::Result<Vec<PathBuf>> {
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
