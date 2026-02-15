#!/usr/bin/env bash
# Install ratlog: prefer pre-built binary from GitHub Releases (no Rust needed).
# Fallback: build from source with cargo (requires Rust and git).
# Usage: ./install.sh  (from project root)
#    or: curl -fsSL https://raw.githubusercontent.com/ahmetbarut/ratlog/main/install.sh | bash
set -e

REPO="${RATLOG_REPO:-https://github.com/ahmetbarut/ratlog}"
REPO_API="${RATLOG_REPO_API:-https://api.github.com/repos/ahmetbarut/ratlog}"
BRANCH="${RATLOG_BRANCH:-main}"
INSTALL_DIR="${RATLOG_INSTALL_DIR:-$HOME/.local/bin}"

# --- Detect platform for pre-built binary ---
detect_platform() {
    local os arch
    case "$(uname -s)" in
        Darwin)  os="darwin" ;;
        Linux)   os="linux" ;;
        *)       os="" ;;
    esac
    case "$(uname -m)" in
        x86_64|amd64) arch="x86_64" ;;
        aarch64|arm64) arch="aarch64" ;;
        *) arch="" ;;
    esac
    if [[ -n "$os" && -n "$arch" ]]; then
        echo "${os}-${arch}"
    else
        echo ""
    fi
}

# --- Install pre-built binary from GitHub Releases ---
install_binary() {
    local platform="$1"
    echo "Checking latest release..."
    if ! command -v curl &>/dev/null; then
        echo "curl not found; skipping binary install."
        return 1
    fi
    local json
    json="$(curl -fsSL "${REPO_API}/releases/latest" 2>/dev/null)" || return 1
    # Find browser_download_url for an asset whose name contains the platform (e.g. ratlog-darwin-aarch64)
    local download_url
    download_url="$(echo "$json" | grep -o '"browser_download_url": *"[^"]*'"$platform"'[^"]*"' | head -1 | sed 's/.*"\(https:\/\/[^"]*\)".*/\1/')"
    [[ -z "$download_url" ]] && return 1
    local asset_name
    asset_name="$(echo "$download_url" | sed 's/.*\///')"
    echo "Downloading $asset_name..."
    mkdir -p "$INSTALL_DIR"
    local tmp_file="$INSTALL_DIR/ratlog.tmp.$$"
    if ! curl -fsSL "$download_url" -o "$tmp_file" 2>/dev/null; then
        rm -f "$tmp_file"
        return 1
    fi
    if [[ "$asset_name" == *.tar.gz ]]; then
        (cd "$INSTALL_DIR" && tar -xzf "$tmp_file" && rm -f "$tmp_file")
        if [[ ! -f "$INSTALL_DIR/ratlog" ]]; then
            for f in "$INSTALL_DIR"/ratlog*; do
                [[ -x "$f" && -f "$f" ]] && mv "$f" "$INSTALL_DIR/ratlog" 2>/dev/null && break
            done
        fi
    else
        mv "$tmp_file" "$INSTALL_DIR/ratlog"
    fi
    chmod +x "$INSTALL_DIR/ratlog"
    echo ""
    echo "Done. ratlog is installed at: $INSTALL_DIR/ratlog"
    if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
        echo "Add to PATH if needed: export PATH=\"\$PATH:$INSTALL_DIR\""
    fi
    echo ""
    echo "Run: ratlog [logfile]"
    return 0
}

# --- Build from source (requires Rust) ---
install_from_source() {
    local dir="$1"
    if ! command -v cargo &>/dev/null; then
        echo "Error: cargo not found. Install Rust from https://rustup.rs and run this script again."
        exit 1
    fi
    echo "Building ratlog (release)..."
    (cd "$dir" && cargo build --release)
    echo "Installing ratlog..."
    mkdir -p "$INSTALL_DIR"
    cp "$dir/target/release/ratlog" "$INSTALL_DIR/ratlog"
    chmod +x "$INSTALL_DIR/ratlog"
    echo ""
    echo "Done. ratlog is installed at: $INSTALL_DIR/ratlog"
    if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
        echo "Add to PATH if needed: export PATH=\"\$PATH:$INSTALL_DIR\""
    fi
    echo ""
    echo "Run: ratlog [logfile]"
}

# --- Main ---
if [[ -f "${BASH_SOURCE[0]:-.}" ]]; then
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
else
    SCRIPT_DIR=""
fi

# If we're in the project root, build from source and exit
if [[ -n "$SCRIPT_DIR" ]] && [[ -f "$SCRIPT_DIR/Cargo.toml" ]] && grep -q 'name = "ratlog"' "$SCRIPT_DIR/Cargo.toml" 2>/dev/null; then
    cd "$SCRIPT_DIR"
    install_from_source "$SCRIPT_DIR"
    exit 0
fi
if [[ -f Cargo.toml ]] && grep -q 'name = "ratlog"' Cargo.toml 2>/dev/null; then
    install_from_source "$(pwd)"
    exit 0
fi

# Not in project: try pre-built binary first
PLATFORM="$(detect_platform)"
if [[ -n "$PLATFORM" ]]; then
    if install_binary "$PLATFORM"; then
        exit 0
    fi
    echo "Pre-built binary not available for $PLATFORM; falling back to build from source."
fi

# Fallback: clone and build from source
REPO_CLONE="${RATLOG_REPO}.git"
echo "Cloning from $REPO_CLONE ($BRANCH)..."
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT
if command -v git &>/dev/null; then
    git clone --depth 1 --branch "$BRANCH" "$REPO_CLONE" "$TMP_DIR/ratlog"
    install_from_source "$TMP_DIR/ratlog"
else
    echo "Error: git not found. Install git and Rust, or use a GitHub Release binary."
    exit 1
fi
