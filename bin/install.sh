#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
INSTALL_DIR="${1:-$HOME/.local/bin}"
BINARY="$REPO_ROOT/target/release/factory"

mkdir -p "$INSTALL_DIR"

if [ ! -f "$BINARY" ]; then
    echo "Release binary not found. Running release build first..."
    "$REPO_ROOT/bin/build.sh"
fi

echo "Installing factory to $INSTALL_DIR..."
install -m 755 "$BINARY" "$INSTALL_DIR/factory"
echo "Installed: $INSTALL_DIR/factory"
"$INSTALL_DIR/factory" --version

case ":$PATH:" in
    *":$INSTALL_DIR:"*) ;;
    *)
        echo ""
        echo "warning: $INSTALL_DIR is not in your PATH."
        echo "Add this line to your shell profile (~/.zshrc, ~/.bashrc, etc.):"
        echo ""
        echo "  export PATH=\"$INSTALL_DIR:\$PATH\""
        echo ""
        ;;
esac
