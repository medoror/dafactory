#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

usage() {
    echo "Usage: $0 <major|minor|patch|x.y.z>"
    exit 1
}

[ $# -eq 1 ] || usage

echo "==> Running tests..."
cd "$REPO_ROOT" && cargo test --quiet

echo "==> Bumping version..."
"$REPO_ROOT/bin/bump-version.sh" "$1"

echo "==> Building release binary..."
"$REPO_ROOT/bin/build.sh"

echo "==> Done. Run bin/install.sh to deploy."
