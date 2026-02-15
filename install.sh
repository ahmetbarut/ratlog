#!/usr/bin/env bash
# Install ratlog: build release binary and install to $CARGO_HOME/bin (~/.cargo/bin)
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

if ! command -v cargo &>/dev/null; then
    echo "Error: cargo not found. Install Rust from https://rustup.rs and run this script again."
    exit 1
fi

if [[ ! -f Cargo.toml ]] || ! grep -q 'name = "ratlog"' Cargo.toml 2>/dev/null; then
    echo "Error: Run this script from the ratlog project root (where Cargo.toml is)."
    exit 1
fi

echo "Building ratlog (release)..."
cargo build --release

echo "Installing ratlog..."
cargo install --path .

BIN_DIR="${CARGO_HOME:-$HOME/.cargo}/bin"
echo ""
echo "Done. ratlog is installed at: $BIN_DIR/ratlog"
if [[ ":$PATH:" != *":$BIN_DIR:"* ]]; then
    echo "Add to PATH if needed: export PATH=\"\$PATH:$BIN_DIR\""
fi
echo ""
echo "Run: ratlog [logfile]"
