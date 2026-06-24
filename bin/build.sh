#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

echo "Building factory (release)..."
cd "$REPO_ROOT"
cargo clean
cargo build --release

echo "Binary: $REPO_ROOT/target/release/factory"
