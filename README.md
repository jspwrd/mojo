# Mojo

A build tool and package manager for C and C++ projects, bringing a Cargo-like experience to C/C++ development.

## Features

- Project scaffolding (`mojo new`, `mojo init`)
- Incremental builds with automatic dependency tracking
- Dependency management (path and git dependencies)
- Cross-compilation support
- Code formatting via `clang-format`
- Testing integration
- Build profiles (debug, release, custom)

## Installation

### Quick install (Linux and macOS)

```sh
curl -fsSL https://raw.githubusercontent.com/jspwrd/mojo/main/install.sh | sh
```

To install a specific version:

```sh
MOJO_VERSION=v1.0.0 curl -fsSL https://raw.githubusercontent.com/jspwrd/mojo/main/install.sh | sh
```

### Build from source

Requires [Rust](https://rustup.rs/) 1.85+ (edition 2024).

```sh
cargo install --path .
```

## Quick Start

```sh
# Create a new C project
mojo new myapp

# Build and run
cd myapp
mojo build
mojo run
```

## License

See [LICENSE](LICENSE) for details.
