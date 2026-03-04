# Mojo

A modern build tool for C and C++ projects, inspired by Cargo.

## Features

- **Project scaffolding** — `mojo new` and `mojo init` generate ready-to-build project structures
- **Incremental compilation** — only recompiles changed sources based on modification times
- **Parallel builds** — compiles across all available CPU cores by default
- **Dependency management** — path and git dependencies with lock file support
- **Multiple compilers** — auto-detects GCC and Clang, configurable per-project or per-target
- **Build profiles** — debug, release, and custom profiles with optimization, debug symbols, and LTO
- **Cross-compilation** — target-triple-specific compiler and flag overrides
- **Sanitizers** — address, undefined, thread, memory, and leak sanitizer support
- **Library support** — static, shared, or both
- **Code formatting** — integrates with `clang-format`

## Quick Start

```sh
# Create a new C++ project
mojo new hello

# Build and run
cd hello
mojo run
```

This generates a project with the following layout:

```
hello/
├── Mojo.toml       # Project configuration
├── src/
│   └── main.cpp    # Entry point
└── tests/
    └── test_hello.cpp
```

## Installation

### From source

```sh
git clone https://github.com/your-org/mojo.git
cd mojo
cargo install --path .
```

### With Docker

```sh
docker build -t mojo .
docker run --rm -v "$PWD":/workspace mojo build
```

## Commands

| Command | Description |
|---|---|
| `mojo new <name>` | Create a new project |
| `mojo init` | Initialize a project in the current directory |
| `mojo build` | Compile the project |
| `mojo run` | Build and run the executable |
| `mojo test` | Build and run tests |
| `mojo check` | Check sources for errors without linking |
| `mojo fmt` | Format code with `clang-format` |
| `mojo clean` | Remove build artifacts |
| `mojo add <dep>` | Add a dependency |
| `mojo tree` | Display the dependency tree |
| `mojo update` | Re-fetch dependencies and regenerate `Mojo.lock` |
| `mojo install` | Install the binary to `~/.local/bin` |

### Global flags

- `-v, --verbose` — show full compiler commands
- `-q, --quiet` — suppress all status output
- `--release` — build with release optimizations

## Configuration

Projects are configured via `Mojo.toml`:

```toml
[package]
name = "myapp"
version = "1.0.0"
lang = "c++"          # "c" or "c++"
std = "c++17"         # Language standard
type = "bin"          # "bin" or "lib"

[build]
compiler = "auto"     # "auto", "gcc", or "clang"
cflags = ["-Wall"]
ldflags = []
libs = ["pthread"]
jobs = 8

[profile.release]
opt_level = "3"       # 0, 1, 2, 3, s, z
debug = false
lto = true

[dependencies]
mylib = { path = "../mylib" }
otherlib = { git = "https://github.com/user/lib.git", tag = "v1.0" }

[scripts]
pre_build = "echo building..."
post_build = "echo done!"

[target.aarch64-unknown-linux-gnu]
cc = "aarch64-linux-gnu-gcc"
cxx = "aarch64-linux-gnu-g++"
ar = "aarch64-linux-gnu-ar"
```

## Dependencies

Mojo supports two kinds of dependencies:

**Path dependencies** — local libraries referenced by filesystem path:
```toml
[dependencies]
mylib = { path = "../mylib" }
```

**Git dependencies** — cloned from a remote repository:
```toml
[dependencies]
mylib = { git = "https://github.com/user/mylib.git", tag = "v1.0" }
mylib = { git = "https://github.com/user/mylib.git", branch = "main" }
mylib = { git = "https://github.com/user/mylib.git", rev = "abc1234" }
```

Dependencies are resolved in topological order with cycle detection. A `Mojo.lock` file is generated to pin exact versions.

## Development

```sh
# Run all tests
cargo test

# Run only unit tests
cargo test --lib

# Run only integration tests
cargo test --test integration

# Check formatting
cargo fmt -- --check

# Lint
cargo clippy -- -D warnings
```

## License

See [LICENSE](LICENSE) for details.
