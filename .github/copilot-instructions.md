# Mojo Codebase Patterns

**Always reuse existing code - no redundancy!**

## Tech Stack

- **Language**: Rust (edition 2024)
- **CLI Framework**: clap (derive macros)
- **Config Parsing**: serde + toml
- **Error Handling**: anyhow
- **Lint/Format**: clippy, rustfmt (`cargo fmt`, `cargo clippy`)
- **Tests**: cargo test (assert_cmd + predicates for integration)

## Anti-Redundancy Rules

- If a function already exists, use it — do NOT create a duplicate.
- Before creating any utility or helper, search for existing implementations first.
- Reuse `src/util.rs` for output helpers (colored printing, verbosity).

## Source of Truth Locations

### Configuration (`src/config.rs`)

- **MojoConfig**: all manifest parsing and validation lives here.
- **NEVER** duplicate config validation logic elsewhere.

### Project Discovery (`src/project.rs`)

- **Project::discover()**: walks up to find `Mojo.toml`.
- Directory layout helpers: `src_dir()`, `include_dir()`, `test_dir()`, `build_dir()`, etc.

### Build Logic (`src/build.rs`)

- Compilation, linking, parallel jobs, library output.
- Object file naming convention: nested paths → flat names (`src/net/socket.cpp` → `net__socket.o`).

### CLI (`src/cli.rs`)

- All subcommand definitions and argument structs.
- Commands dispatch from `src/main.rs`.

### Commands (`src/commands/`)

- One file per subcommand. Each exports an `exec` function.

## Code Quality

- Rust, strict typing, no `unsafe` without justification
- Keep files under ~500 LOC — extract modules when larger
- Unit tests: `#[cfg(test)]` blocks within source files
- Integration tests: `tests/integration.rs`
- Run `cargo clippy` and `cargo fmt --check` before commits
- Run `cargo test` before pushing
