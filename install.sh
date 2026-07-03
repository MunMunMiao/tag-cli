#!/bin/bash
set -euo pipefail

REPO="MunMunMiao/tag-cli"
DEFAULT_SYS_DIR="/usr/local/bin"
DEFAULT_USER_DIR="${HOME}/.local/bin"

usage() {
  cat <<EOF
Usage: $0 [--install-dir <dir>] [--version <tag>] [--uninstall]

Install tag-cli from GitHub releases.

Options:
  --install-dir <dir>  Install directory (default: ${DEFAULT_SYS_DIR} if writable,
                       otherwise ${DEFAULT_USER_DIR})
  --version <tag>      Release tag to install, e.g. v0.1.0 (default: latest)
  --uninstall          Remove an existing tag-cli installation
  -h, --help           Show this help
EOF
}

parse_args() {
  INSTALL_DIR=""
  VERSION="latest"
  UNINSTALL=0

  while [[ $# -gt 0 ]]; do
    case "$1" in
      --install-dir)
        if [[ $# -lt 2 ]]; then
          echo "Option --install-dir requires an argument." >&2
          usage >&2
          exit 1
        fi
        INSTALL_DIR="$2"
        shift 2
        ;;
      --version)
        if [[ $# -lt 2 ]]; then
          echo "Option --version requires an argument." >&2
          usage >&2
          exit 1
        fi
        VERSION="$2"
        shift 2
        ;;
      --uninstall)
        UNINSTALL=1
        shift
        ;;
      -h|--help)
        usage
        exit 0
        ;;
      *)
        echo "Unknown option: $1" >&2
        usage >&2
        exit 1
        ;;
    esac
  done
}

detect_platform() {
  local os arch

  case "$(uname -s)" in
    Linux)     os="linux" ;;
    Darwin)    os="macos" ;;
    CYGWIN*|MINGW*|MSYS*) os="windows" ;;
    *)
      echo "Unsupported operating system: $(uname -s)" >&2
      echo "Please install manually from https://github.com/${REPO}/releases" >&2
      exit 1
      ;;
  esac

  case "$(uname -m)" in
    x86_64|amd64) arch="x86_64" ;;
    aarch64|arm64) arch="aarch64" ;;
    *)
      echo "Unsupported architecture: $(uname -m)" >&2
      echo "Please install manually from https://github.com/${REPO}/releases" >&2
      exit 1
      ;;
  esac

  if [[ "$os" == "windows" ]]; then
    echo "Windows/Cygwin/MSYS installation is not supported by this script." >&2
    echo "Please download the .exe manually from https://github.com/${REPO}/releases" >&2
    exit 1
  fi

  if [[ "$os" == "linux" && "$arch" != "x86_64" ]]; then
    echo "Unsupported platform: ${arch}-${os}" >&2
    echo "Please install manually from https://github.com/${REPO}/releases" >&2
    exit 1
  fi

  printf '%s %s\n' "$os" "$arch"
}

require_command() {
  local cmd="$1"
  if ! command -v "$cmd" >/dev/null 2>&1; then
    echo "Required command not found: $cmd" >&2
    exit 1
  fi
}

resolve_install_dir() {
  if [[ -z "$INSTALL_DIR" ]]; then
    if [[ -d "$DEFAULT_SYS_DIR" && -w "$DEFAULT_SYS_DIR" ]]; then
      INSTALL_DIR="$DEFAULT_SYS_DIR"
    else
      INSTALL_DIR="$DEFAULT_USER_DIR"
    fi
  fi

  if [[ ! -d "$INSTALL_DIR" ]]; then
    mkdir -p "$INSTALL_DIR" || {
      echo "Failed to create install directory: $INSTALL_DIR" >&2
      exit 1
    }
  fi

  if [[ ! -w "$INSTALL_DIR" ]]; then
    echo "Install directory is not writable: $INSTALL_DIR" >&2
    exit 1
  fi
}

resolve_version() {
  if [[ "$VERSION" == "latest" ]]; then
    echo "Determining latest release..."
    VERSION="$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
      | grep -o '"tag_name": "[^"]*' \
      | cut -d'"' -f4)"
    if [[ -z "$VERSION" ]]; then
      echo "Failed to determine latest release tag." >&2
      exit 1
    fi
    BASE_URL="https://github.com/${REPO}/releases/latest/download"
  else
    BASE_URL="https://github.com/${REPO}/releases/download/${VERSION}"
  fi

  VERSION_PLAIN="${VERSION#v}"
}

verify_checksum() {
  local asset="$1"
  local binary_path="$2"
  local sums_path="$3"

  local expected
  expected="$(awk -v name="$asset" '$2 == name {print $1}' "$sums_path")"
  if [[ -z "$expected" ]]; then
    echo "Warning: no checksum found for ${asset} in SHA256SUMS" >&2
    return 0
  fi

  if command -v sha256sum >/dev/null 2>&1; then
    echo "Verifying checksum with sha256sum..."
    local actual
    actual="$(sha256sum "$binary_path" | awk '{print $1}')"
    if [[ "$actual" != "$expected" ]]; then
      echo "Checksum verification failed for ${asset}." >&2
      exit 1
    fi
  elif command -v shasum >/dev/null 2>&1; then
    echo "Verifying checksum with shasum..."
    local actual
    actual="$(shasum -a 256 "$binary_path" | awk '{print $1}')"
    if [[ "$actual" != "$expected" ]]; then
      echo "Checksum verification failed for ${asset}." >&2
      exit 1
    fi
  else
    echo "Warning: neither sha256sum nor shasum found; skipping checksum verification." >&2
  fi
}

warn_path() {
  case ":${PATH}:" in
    *":${INSTALL_DIR}:"*) ;;
    *)
      echo "Warning: ${INSTALL_DIR} is not on your PATH." >&2
      echo "         Add it with: export PATH=\"${INSTALL_DIR}:\$PATH\"" >&2
      ;;
  esac
}

is_system_dir() {
  local dir="$1"
  case "$dir" in
    /usr/local/bin|/usr/bin|/bin|/sbin|/usr/sbin)
      return 0
      ;;
    *)
      return 1
      ;;
  esac
}

uninstall_cli() {
  local binary_path=""
  local on_path=0

  if [[ -n "$INSTALL_DIR" ]]; then
    binary_path="${INSTALL_DIR}/tag-cli"
  elif command -v tag-cli >/dev/null 2>&1; then
    binary_path="$(command -v tag-cli)"
    on_path=1
  fi

  if [[ -z "$binary_path" || ! -f "$binary_path" ]]; then
    echo "Error: tag-cli binary not found." >&2
    exit 1
  fi

  local install_dir
  install_dir="$(dirname "$binary_path")"

  rm -f "$binary_path"

  if is_system_dir "$install_dir"; then
    : # Never remove system directories.
  elif [[ "$install_dir" == "$DEFAULT_USER_DIR" ]]; then
    rmdir "$install_dir" 2>/dev/null || true
  elif [[ -d "$install_dir" && -z "$(ls -A "$install_dir" 2>/dev/null)" ]]; then
    rmdir "$install_dir" 2>/dev/null || true
  fi

  if [[ "$on_path" -eq 1 ]]; then
    echo "tag-cli has been removed from ${binary_path}."
  fi
}

main() {
  parse_args "$@"

  if [[ "$UNINSTALL" -eq 1 ]]; then
    uninstall_cli
    return 0
  fi

  if [[ "$VERSION" != "latest" && ! "$VERSION" =~ ^v?[0-9]+\.[0-9]+\.[0-9]+(-[[:alnum:]._-]+)?$ ]]; then
    echo "Warning: --version should be a release tag like v0.1.0 (got: ${VERSION})." >&2
  fi

  require_command curl

  local os arch
  read -r os arch <<< "$(detect_platform)"

  resolve_version

  local asset bin_name
  bin_name="tag-cli"
  asset="tag-cli-${VERSION_PLAIN}-${arch}-${os}"

  resolve_install_dir

  tmp_dir="$(mktemp -d)"
  trap 'rm -rf "$tmp_dir"' EXIT

  local tmp_binary="${tmp_dir}/${bin_name}"
  local tmp_sums="${tmp_dir}/SHA256SUMS"

  echo "Downloading ${asset}..."
  curl -fsSL "${BASE_URL}/${asset}" -o "$tmp_binary"

  echo "Downloading SHA256SUMS..."
  if curl -fsSL "${BASE_URL}/SHA256SUMS" -o "$tmp_sums"; then
    verify_checksum "$asset" "$tmp_binary" "$tmp_sums"
  else
    echo "Warning: could not download SHA256SUMS; skipping checksum verification." >&2
  fi

  install -m 755 "$tmp_binary" "${INSTALL_DIR}/${bin_name}"

  echo "Installed ${INSTALL_DIR}/${bin_name} (${VERSION})"

  warn_path
}

main "$@"
