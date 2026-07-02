#!/bin/bash
set -euo pipefail

VERSION="$1"
TARGET="$2"

EXE=""
if [[ "$TARGET" == *-windows-* ]]; then
  EXE=".exe"
fi

BIN="target/${TARGET}/release/tag-cli${EXE}"
PKG_NAME="tag-cli-${VERSION}-${TARGET}"
PKG_DIR="dist/${PKG_NAME}"

mkdir -p "${PKG_DIR}/completions" "${PKG_DIR}/man"

cp "${BIN}" "${PKG_DIR}/"
cp README.md CHANGELOG.md "${PKG_DIR}/"

LICENSE_SRC=""
if [[ -f LICENSE ]]; then
  LICENSE_SRC="LICENSE"
elif [[ -f LICENSE-MIT ]]; then
  LICENSE_SRC="LICENSE-MIT"
fi

if [[ -n "$LICENSE_SRC" ]]; then
  cp "$LICENSE_SRC" "${PKG_DIR}/LICENSE"
else
  echo "Warning: no LICENSE or LICENSE-MIT found; package will not include a license file" >&2
fi

if [[ -d dist/completions ]] && [[ -n "$(ls -A dist/completions)" ]]; then
  cp dist/completions/* "${PKG_DIR}/completions/"
else
  "${BIN}" completions bash > "${PKG_DIR}/completions/tag-cli.bash"
  "${BIN}" completions fish > "${PKG_DIR}/completions/tag-cli.fish"
  "${BIN}" completions zsh > "${PKG_DIR}/completions/_tag-cli"
  "${BIN}" completions powershell > "${PKG_DIR}/completions/_tag-cli.ps1"
fi

if [[ -f dist/man/tag-cli.1 ]]; then
  cp dist/man/tag-cli.1 "${PKG_DIR}/man/tag-cli.1"
else
  "${BIN}" man > "${PKG_DIR}/man/tag-cli.1"
fi

cd dist

sha256_file() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1"
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1"
  else
    openssl dgst -sha256 -r "$1"
  fi
}

if [[ "$TARGET" == *-windows-* ]]; then
  ARCHIVE="${PKG_NAME}.zip"
  if command -v 7z >/dev/null 2>&1; then
    7z a "$ARCHIVE" "$PKG_NAME"
  elif command -v zip >/dev/null 2>&1; then
    zip -r "$ARCHIVE" "$PKG_NAME"
  else
    powershell -Command "Compress-Archive -Path '${PKG_NAME}' -DestinationPath '${ARCHIVE}' -Force"
  fi
else
  ARCHIVE="${PKG_NAME}.tar.gz"
  tar czf "$ARCHIVE" "$PKG_NAME"
fi

sha256_file "$ARCHIVE" > "${ARCHIVE}.sha256"

echo "Created ${ARCHIVE} and ${ARCHIVE}.sha256"
