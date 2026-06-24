#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
INSTALL_DIR="${1:-/usr/local/bin}"
BINARY="$REPO_ROOT/target/release/factory"

if [ ! -f "$BINARY" ]; then
    echo "Release binary not found. Running release build first..."
    "$REPO_ROOT/bin/build.sh"
fi

echo "Installing factory to $INSTALL_DIR..."
install -m 755 "$BINARY" "$INSTALL_DIR/factory"
echo "Installed: $(command -v factory)"
factory --version
