#!/usr/bin/env bash
set -euo pipefail

REPO="skyne/scarlet-city"
TAG="assets-v1"
ASSET="scarlet-city-assets.tar.gz"
DEST="client/wwwroot/assets"

if [ -d "$DEST/quaternius" ] && [ -d "$DEST/world" ] && [ "$(find "$DEST" -type f | wc -l)" -gt 100 ]; then
  echo "Assets already present in $DEST"
  exit 0
fi

command -v gh >/dev/null || { echo "gh CLI required: https://cli.github.com/"; exit 1; }

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

echo "Downloading $ASSET from $REPO release $TAG..."
gh release download "$TAG" -R "$REPO" -p "$ASSET" -D "$TMP"

mkdir -p client/wwwroot
tar -xzf "$TMP/$ASSET" -C client/wwwroot
echo "Assets extracted to $DEST"
