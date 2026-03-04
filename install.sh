#!/bin/sh
# Mojo installer — detects OS/arch, downloads the matching release tarball
# from GitHub, extracts it, and installs the binary.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/jspwrd/mojo/main/install.sh | sh
#   wget -qO- https://raw.githubusercontent.com/jspwrd/mojo/main/install.sh | sh
#
# Options (environment variables):
#   MOJO_VERSION   — version to install (default: latest)
#   INSTALL_DIR    — where to place the binary (default: ~/.local/bin on Linux, /usr/local/bin on macOS)

set -eu

REPO="jspwrd/mojo"
BINARY="mojo"

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

say() {
    printf '%s\n' "$*"
}

err() {
    say "error: $*" >&2
    exit 1
}

need() {
    if ! command -v "$1" > /dev/null 2>&1; then
        err "need '$1' (command not found)"
    fi
}

# ---------------------------------------------------------------------------
# Detect OS
# ---------------------------------------------------------------------------

detect_os() {
    case "$(uname -s)" in
        Linux*)  echo "linux" ;;
        Darwin*) echo "macos" ;;
        *)       err "unsupported OS: $(uname -s)" ;;
    esac
}

# ---------------------------------------------------------------------------
# Detect architecture
# ---------------------------------------------------------------------------

detect_arch() {
    case "$(uname -m)" in
        x86_64|amd64)   echo "x86_64" ;;
        aarch64|arm64)   echo "aarch64" ;;
        *)               err "unsupported architecture: $(uname -m)" ;;
    esac
}

# ---------------------------------------------------------------------------
# Resolve the download URL
# ---------------------------------------------------------------------------

resolve_version() {
    if [ -n "${MOJO_VERSION:-}" ]; then
        echo "$MOJO_VERSION"
        return
    fi

    need curl
    # GitHub redirects /releases/latest to /releases/tag/<tag>; extract the tag.
    latest_url=$(curl -fsSL -o /dev/null -w '%{url_effective}' \
        "https://github.com/${REPO}/releases/latest" 2>/dev/null) \
        || err "could not determine latest release (are there any releases?)"

    version="${latest_url##*/}"
    if [ -z "$version" ]; then
        err "could not parse version from redirect URL"
    fi
    echo "$version"
}

# Asset name convention: mojo-<tag>-<os>-<arch>.tar.gz
asset_name() {
    _version="$1"
    _os="$2"
    _arch="$3"
    echo "${BINARY}-${_version}-${_os}-${_arch}.tar.gz"
}

download_url() {
    _version="$1"
    _asset="$2"
    echo "https://github.com/${REPO}/releases/download/${_version}/${_asset}"
}

# ---------------------------------------------------------------------------
# Default install directory
# ---------------------------------------------------------------------------

default_install_dir() {
    _os="$1"
    case "$_os" in
        macos) echo "/usr/local/bin" ;;
        linux) echo "${HOME}/.local/bin" ;;
    esac
}

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

main() {
    need uname
    need curl
    need tar
    need install

    os="$(detect_os)"
    arch="$(detect_arch)"
    version="$(resolve_version)"
    asset="$(asset_name "$version" "$os" "$arch")"
    url="$(download_url "$version" "$asset")"
    install_dir="${INSTALL_DIR:-$(default_install_dir "$os")}"

    say "  Detected: ${os} ${arch}"
    say "  Version:  ${version}"
    say "  Asset:    ${asset}"
    say "  Install:  ${install_dir}/${BINARY}"
    say ""

    # Create a temporary directory and ensure cleanup.
    tmpdir="$(mktemp -d)"
    trap 'rm -rf "$tmpdir"' EXIT

    say "Downloading ${url} ..."
    curl -fSL "$url" -o "${tmpdir}/${asset}" \
        || err "download failed — check that release ${version} exists for ${os}-${arch}"

    say "Extracting ..."
    tar xzf "${tmpdir}/${asset}" -C "$tmpdir"

    # Locate the binary inside the extracted archive.
    if [ -f "${tmpdir}/${BINARY}" ]; then
        src="${tmpdir}/${BINARY}"
    elif [ -f "${tmpdir}/${BINARY}-${version}-${os}-${arch}/${BINARY}" ]; then
        src="${tmpdir}/${BINARY}-${version}-${os}-${arch}/${BINARY}"
    else
        # Fallback: find the first executable named $BINARY.
        src="$(find "$tmpdir" -name "$BINARY" -type f | head -n 1)"
        [ -n "$src" ] || err "could not locate '${BINARY}' binary in the archive"
    fi

    # Install the binary.
    mkdir -p "$install_dir"
    install -m 755 "$src" "${install_dir}/${BINARY}" \
        || err "failed to install to ${install_dir}/${BINARY} (try running with sudo or set INSTALL_DIR)"

    say ""
    say "Mojo ${version} installed to ${install_dir}/${BINARY}"

    # Warn if the install directory is not on PATH.
    case ":${PATH}:" in
        *":${install_dir}:"*) ;;
        *)
            say ""
            say "WARNING: '${install_dir}' is not in your PATH."
            say "Add it by running:"
            say ""
            say "  export PATH=\"${install_dir}:\$PATH\""
            say ""
            ;;
    esac
}

main
