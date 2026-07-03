#!/bin/bash
set -euo pipefail

VERSION="$1"
TARGET="$2"

case "$TARGET" in
  x86_64-unknown-linux-gnu)
    SUFFIX="x86_64-linux"
    ;;
  x86_64-apple-darwin)
    SUFFIX="x86_64-macos"
    ;;
  aarch64-apple-darwin)
    SUFFIX="aarch64-macos"
    ;;
  x86_64-pc-windows-msvc)
    SUFFIX="x86_64-windows"
    ;;
  *)
    echo "Unsupported target: $TARGET" >&2
    exit 1
    ;;
esac

EXE=""
if [[ "$TARGET" == *-windows-* ]]; then
  EXE=".exe"
fi

BIN="target/${TARGET}/release/tag-cli${EXE}"
DEST="dist/tag-cli-${VERSION}-${SUFFIX}${EXE}"

mkdir -p dist
cp "$BIN" "$DEST"

echo "Created $DEST"
