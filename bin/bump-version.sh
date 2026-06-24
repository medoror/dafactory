#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CARGO_TOML="$REPO_ROOT/Cargo.toml"

usage() {
    echo "Usage: $0 <major|minor|patch|x.y.z>"
    exit 1
}

[ $# -eq 1 ] || usage

# Read current version from the [package] section
CURRENT=$(sed -n '/^\[package\]/,/^\[/{s/^version = "\(.*\)"/\1/p}' "$CARGO_TOML")
[ -n "$CURRENT" ] || { echo "Could not read version from Cargo.toml"; exit 1; }

IFS='.' read -r MAJOR MINOR PATCH <<< "$CURRENT"

case "$1" in
    major) MAJOR=$((MAJOR + 1)); MINOR=0; PATCH=0 ;;
    minor) MINOR=$((MINOR + 1)); PATCH=0 ;;
    patch) PATCH=$((PATCH + 1)) ;;
    [0-9]*.[0-9]*.[0-9]*) IFS='.' read -r MAJOR MINOR PATCH <<< "$1" ;;
    *) usage ;;
esac

NEW="$MAJOR.$MINOR.$PATCH"
echo "Bumping $CURRENT → $NEW"

# Update only the version line in [package] (first occurrence)
awk -v old="$CURRENT" -v new="$NEW" '
    !done && /^version = "/ { sub(old, new); done=1 }
    { print }
' "$CARGO_TOML" > "$CARGO_TOML.tmp" && mv "$CARGO_TOML.tmp" "$CARGO_TOML"

# Sync Cargo.lock
cd "$REPO_ROOT" && cargo generate-lockfile --quiet

git -C "$REPO_ROOT" add Cargo.toml Cargo.lock
git -C "$REPO_ROOT" commit -m "Bump version to $NEW"

echo "Version is now $NEW"
