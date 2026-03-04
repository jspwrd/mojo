# Repository Guidelines

- Repo: https://github.com/jspwrd/mojo
- File references must be repo-root relative (e.g. `src/commands/build.rs:45`); never absolute paths.

## Project Overview

Mojo is a build tool and package manager for C and C++ projects, written in Rust. It provides a Cargo-like experience for C/C++ development: project scaffolding, dependency management, incremental builds, cross-compilation, formatting, testing, and installation.

## Project Structure & Module Organization

- Entry point: `src/main.rs` (CLI dispatch via clap).
- CLI definitions: `src/cli.rs` (all subcommands and argument structs).
- Commands: `src/commands/` (one file per subcommand: `new`, `init`, `build`, `run`, `check`, `test`, `fmt`, `clean`, `add`, `tree`, `install`, `update`).
- Core modules:
  - `src/config.rs` — `MojoConfig` deserialization from `Mojo.toml`, validation logic.
  - `src/project.rs` — project discovery (walks up to find `Mojo.toml`), directory layout.
  - `src/build.rs` — core build logic: compile, link, parallel jobs, library output.
  - `src/compiler.rs` — compiler detection and abstraction (gcc/clang, C/C++).
  - `src/deps.rs` — dependency resolution (path + git), topological sort, cycle detection.
  - `src/incremental.rs` — freshness checking via mtime comparison.
  - `src/lock.rs` — `Mojo.lock` generation and reading.
  - `src/scaffold.rs` — project template generation for `new`/`init`.
  - `src/util.rs` — colored output helpers, verbosity control.
- Tests: `tests/integration.rs` (integration tests using `assert_cmd` + `tempfile`).
- Unit tests: colocated as `#[cfg(test)] mod tests` blocks within source files (e.g. `src/config.rs`).

## Build, Test, and Development Commands

- Language: Rust (edition 2024).
- Build: `cargo build`
- Build release: `cargo build --release`
- Run: `cargo run -- <mojo-args>`
- Check (type/borrow check without codegen): `cargo check`
- Lint: `cargo clippy --all-features -- -D warnings`
- Format check: `cargo fmt --all -- --check`
- Format fix: `cargo fmt --all`
- Tests: `cargo test`
- Install from source: `cargo install --path .`

## Coding Style & Naming Conventions

- Language: Rust. Follow standard Rust idioms and conventions.
- Use `anyhow::Result` for fallible functions; use `anyhow::bail!` / `anyhow::Context` for error context.
- Prefer `clap` derive macros for CLI argument parsing.
- Use `serde::Deserialize` for config parsing; keep `Mojo.toml` structure in `src/config.rs`.
- No `unsafe` code unless absolutely necessary and well-justified.
- No `#[allow(...)]` suppressions; fix the root cause.
- Add brief code comments for tricky or non-obvious logic.
- Keep files concise; aim for under ~500 LOC. Extract modules when files grow.
- Naming: use **Mojo** for product/docs headings; `mojo` for CLI command and paths.

## Testing Guidelines

- Integration tests in `tests/integration.rs` using `assert_cmd` and `predicates`.
- Unit tests as `#[cfg(test)]` modules within source files.
- Test naming: descriptive snake_case (e.g. `build_debug`, `new_invalid_name`).
- Run tests before pushing when you touch logic.
- Integration tests use `tempfile::TempDir` for isolation — always clean up.

## Project Configuration (Mojo.toml)

The manifest file `Mojo.toml` supports these sections:
- `[package]` — name, version, lang (`c`/`c++`), std, type (`bin`/`lib`), lib-type (`static`/`shared`/`both`).
- `[build]` — compiler (`auto`/`gcc`/`clang`), cflags, ldflags, libs, jobs, sanitizers.
- `[profile.<name>]` — opt_level (`0`-`3`, `s`, `z`), debug, lto.
- `[dependencies]` — path-based (`path = "../lib"`) or git-based (`git`, `tag`, `branch`, `rev`).
- `[scripts]` — `pre_build`, `post_build`.
- `[target.<triple>]` — cross-compilation: cc, cxx, ar, cflags, ldflags.

## Commit & Pull Request Guidelines

- Follow concise, action-oriented commit messages (e.g. `build: add incremental header tracking`).
- Group related changes; avoid bundling unrelated refactors.

## Security & Configuration Tips

- Never commit real secrets or live configuration values.
- Use obviously fake placeholders in docs, tests, and examples.
- Validate all user-provided paths (project names, dependency paths) to prevent path traversal.

## Agent-Specific Notes

- When answering questions, verify in code; do not guess.
- Do not modify `Cargo.lock` manually; let `cargo` manage it.
- When adding dependencies, justify the addition and prefer minimal, well-maintained crates.
- The project uses Rust edition 2024 — use current Rust idioms.

### Multi-Agent Safety

- Do **not** create/apply/drop `git stash` entries unless explicitly requested.
- When told to "push", you may `git pull --rebase` (never discard others' work).
- When told to "commit", scope to your changes only.
- Do **not** switch branches unless explicitly requested.
- When you see unrecognized files, keep going; focus on your own changes.

### Lint/Format Churn

- If diffs are formatting-only, auto-resolve without asking.
- Only ask when changes are semantic (logic/data/behavior).

### Bug Investigations

- Read source code of relevant crates and all related local code before concluding.
- Aim for high-confidence root cause.
