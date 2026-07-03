# tag-cli

[![GitHub Release](https://img.shields.io/github/v/release/MunMunMiao/tag-cli.svg)](https://github.com/MunMunMiao/tag-cli/releases)
[![CI](https://github.com/MunMunMiao/tag-cli/actions/workflows/ci.yml/badge.svg)](https://github.com/MunMunMiao/tag-cli/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![MSRV](https://img.shields.io/badge/MSRV-1.85+-blue.svg)](https://blog.rust-lang.org/2025/02/20/Rust-1.85.0.html)

**tag-cli** is a Rust CLI for reading and writing audio metadata and embedded cover art. It wraps a vendored [TagLib](https://taglib.org/) C++ library, so the same binary handles MP3, FLAC, M4A, Ogg, and many other formats.

Edit tags and covers for one file or an entire library. Preview every change with `--dry-run`. Automate with YAML manifests, shell scripts, or CI pipelines.

```bash
# Install on Linux or macOS
curl -fsSL https://raw.githubusercontent.com/MunMunMiao/tag-cli/main/install.sh | bash

# Read a file
tag-cli info -i song.mp3

# Preview a write (no changes yet)
tag-cli set -i song.mp3 --dry-run TITLE="My Song" ARTIST="Me"

# Apply the write after confirming the preview
tag-cli set -i song.mp3 -y TITLE="My Song" ARTIST="Me"
```

Prefer a manual install? Download prebuilt binaries from [GitHub Releases](https://github.com/MunMunMiao/tag-cli/releases).

> [!NOTE]
> This README uses `tag-cli` to mean the installed binary and the source repository.

<a id="why-tag-cli"></a>
## Why tag-cli?

- **One tool, many formats.** MP3, FLAC, M4A, Ogg Vorbis, Opus, WAV, and more.
- **Safe by default.** Write commands require confirmation for in-place edits and when overwriting existing output files; most write commands support `--dry-run` preview.
- **Batch ready.** Apply a YAML manifest to an album or library in one command.
- **CI friendly.** Machine-readable JSON/YAML output and environment-variable confirmation.
- **Self updating.** `tag-cli update` downloads verified releases and replaces the running binary.

<a id="who-is-this-for"></a>
## Who is this for?

- Music collectors who want reproducible metadata edits across a library.
- Producers and labels who need consistent album metadata and cover art.
- Developers who want to lint or transform audio metadata in CI pipelines.

<a id="safety-first"></a>
## Safety first: three steps for every write

> [!WARNING]
> tag-cli does not create backups automatically. Before writing, back up important files yourself.

```text
1. Back up originals   →  cp -r music music.bak
2. Preview changes     →  tag-cli ... --dry-run
3. Confirm and write   →  tag-cli ... -y
```

- `--dry-run` does not modify audio files, but `apply` still decodes and re-encodes cover images for validation. Missing or corrupt covers cause preview failures.
- In-place writes (`set`, `clear`, `cover set`, `cover clear`, `apply`) require confirmation. Writing to a new file with `-o` does not.
- `export metadata` requires confirmation only when overwriting an existing aggregated report or sidecar file. Output to stdout does not.
- `init-manifest` always requires confirmation because it creates or overwrites the output file.

<a id="quick-start"></a>
## Quick start

```bash
# Verify the installation
$ tag-cli --version
tag-cli 0.1.0

# List supported tag keys
$ tag-cli list-keys

# Read full metadata for one file
$ tag-cli info -i song.mp3

# Preview an in-place tag edit (does not write)
$ tag-cli set -i song.mp3 --dry-run TITLE="My Song" ARTIST="Me"

# Apply the edit after confirming the preview above
$ tag-cli set -i song.mp3 -y TITLE="My Song" ARTIST="Me"
```

> [!CAUTION]
> `tag-cli set -i song.mp3 -y ...` modifies `song.mp3` in place. Always run `--dry-run` first and review the diff before adding `-y`.

---

<a id="table-of-contents"></a>
## Table of contents

### Getting started
- [Why tag-cli?](#why-tag-cli)
- [Who is this for?](#who-is-this-for)
- [Features](#features)
- [Safety first](#safety-first)
- [Quick start](#quick-start)
- [Prerequisites](#prerequisites)
- [Installation](#installation)
- [Installation verification](#installation-verification)
- [Update](#update)

### Using tag-cli
- [Supported formats](#supported-formats)
- [Global options and safety behavior](#global-options)
- [Command reference](#command-reference)
- [Declarative batch editing with apply](#apply)
- [init-manifest templates](#init-manifest-templates)
- [Image processing behavior](#image-processing)
- [Batch editing safety checklist](#batch-safety-checklist)
- [Automation examples](#automation-examples)

### Reference
- [Exit codes and environment variables](#exit-codes-and-environment-variables)
- [Troubleshooting / FAQ](#troubleshooting-and-faq)
- [Contributing](#contributing)
- [License](#license)

---

<a id="features"></a>
## Features

- Read and write metadata tags and embedded cover art for common audio formats.
- Declarative batch editing for albums or libraries via YAML manifest.
- Automatic cover processing: scaling, format selection, EXIF/metadata stripping, and size limits.
- Output formats: human-readable tables, JSON, and YAML.
- `--dry-run` preview for all write operations that support it.
- Scenario-oriented manifest templates via `init-manifest --template`.
- Environment-variable confirmation for scripting and CI (`TAG_CLI_YES=1` or `CI=true`).

<a id="prerequisites"></a>
## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) stable toolchain (MSRV 1.85+)
- C++ toolchain
- `cmake`
- `zlib` development headers
- `vendor/taglib` Git submodule initialized

### Platform notes

- **macOS**: Xcode Command Line Tools provide the C++ toolchain and `zlib`. `cmake` is not included by default:
  ```bash
  brew install cmake
  ```
- **Linux (Debian/Ubuntu)**: Install system dependencies:
  ```bash
  sudo apt-get update && sudo apt-get install -y cmake clang zlib1g-dev
  # or
  sudo apt-get update && sudo apt-get install -y build-essential cmake zlib1g-dev
  ```
- **Windows**: Use a manual download from [GitHub Releases](https://github.com/MunMunMiao/tag-cli/releases), or build from source in MSYS2/WSL. The one-line PowerShell installer is not supported.

<a id="installation"></a>
## Installation

### One-line installer

Available for Linux and macOS. The script downloads the binary and a `SHA256SUMS` file, verifies the checksum, and installs to `/usr/local/bin` when writable; otherwise it falls back to `$HOME/.local/bin`. If the binary is installed to `$HOME/.local/bin`, ensure that directory is in your `PATH`.

> [!CAUTION]
> Always review scripts before piping them to `bash`. The installer is served from the repository `main` branch. For extra assurance, pin a release version or download and inspect `install.sh` first.

```bash
curl -fsSL https://raw.githubusercontent.com/MunMunMiao/tag-cli/main/install.sh | bash
```

Install a specific version:

```bash
curl -fsSL https://raw.githubusercontent.com/MunMunMiao/tag-cli/main/install.sh | bash -s -- --version v0.1.0
```

Install to a custom directory:

```bash
curl -fsSL https://raw.githubusercontent.com/MunMunMiao/tag-cli/main/install.sh | bash -s -- --install-dir ~/.bin
```

### Pre-built binaries

Download raw binaries for Linux or macOS from [GitHub Releases](https://github.com/MunMunMiao/tag-cli/releases), then verify the published SHA256 checksum before moving the binary into your `PATH`.

```bash
VERSION=0.1.0
TARGET=x86_64-linux  # or x86_64-macos, aarch64-macos

# Download the binary and checksum file
curl -LO "https://github.com/MunMunMiao/tag-cli/releases/download/v${VERSION}/tag-cli-${VERSION}-${TARGET}"
curl -LO "https://github.com/MunMunMiao/tag-cli/releases/download/v${VERSION}/SHA256SUMS"

# Verify the checksum
shasum -a 256 -c SHA256SUMS

# Install
chmod +x "tag-cli-${VERSION}-${TARGET}"
sudo mv "tag-cli-${VERSION}-${TARGET}" /usr/local/bin/tag-cli
tag-cli --version
```

### Install from source

```bash
git clone https://github.com/MunMunMiao/tag-cli.git
cd tag-cli
git submodule update --init --recursive
cargo install --path crates/tag-cli
```

The first build compiles the vendored TagLib C++ library. Expect 3–10 minutes depending on CPU, and several GB of disk use under `target/`. `cargo install` places the `tag-cli` binary in `~/.cargo/bin`; add it to your `PATH`:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

Add the same line to `~/.bashrc`, `~/.zshrc`, or equivalent to make it persist across sessions.

Uninstall with:

```bash
cargo uninstall tag-cli
```

### Uninstall

Run the installer with `--uninstall`:

```bash
curl -fsSL https://raw.githubusercontent.com/MunMunMiao/tag-cli/main/install.sh | bash -s -- --uninstall
```

Or remove the binary manually:

```bash
rm -f /usr/local/bin/tag-cli
rm -f "$HOME/.local/bin/tag-cli"
rmdir "$HOME/.local/bin" 2>/dev/null || true
```

<a id="installation-verification"></a>
## Installation verification

```bash
$ tag-cli --version
tag-cli 0.1.0

$ tag-cli list-keys
TITLE
ARTIST
ALBUM
...
```

If `list-keys` prints supported key names, TagLib's foreign-function interface (FFI) is correctly linked.

tag-cli does not read additional configuration files. All options are passed as command-line arguments or environment variables.

<a id="update"></a>
## Update

```bash
tag-cli update
```

`tag-cli update` checks GitHub Releases, downloads the matching binary for your platform, verifies the SHA256 checksum, and replaces the running executable. No confirmation prompt is shown.

The update command honors standard proxy environment variables:

- `HTTP_PROXY` / `http_proxy`
- `HTTPS_PROXY` / `https_proxy`
- `ALL_PROXY` / `all_proxy`
- `NO_PROXY` / `no_proxy`

Proxy selection follows the usual scheme-specific priority: `HTTPS_PROXY` for HTTPS URLs, then `ALL_PROXY`, then `HTTP_PROXY`. `NO_PROXY` supports `*` for all hosts, exact hosts, and domain suffixes such as `.example.com`.

> [!NOTE]
> `TAG_CLI_UPDATE_API_URL` and `TAG_CLI_UPDATE_DOWNLOAD_BASE` can override the GitHub API and download base URLs, but only in debug/test builds or when the `test-overrides` feature is enabled. They are compiled out of release binaries.

<a id="supported-formats"></a>
## Supported audio formats and cover image input formats

### Supported audio formats

| Format | Common extensions |
|--------|-------------------|
| MPEG | `*.mp3`, `*.mp2` |
| MP4 / M4A / M4R / M4B / M4P / 3G2 / AAC | `*.m4a`, `*.m4r`, `*.m4b`, `*.m4p`, `*.mp4`, `*.m4v`, `*.3g2`, `*.aac` |
| FLAC | `*.flac` |
| Ogg Vorbis | `*.ogg` |
| Ogg Opus | `*.opus` |
| Ogg FLAC | `*.oga` |
| Speex | `*.spx` |
| WAV | `*.wav` |
| AIFF / AIFC | `*.aif`, `*.aiff`, `*.afc`, `*.aifc` |
| WMA / ASF | `*.wma`, `*.asf` |
| APE | `*.ape` |
| MPC | `*.mpc` |
| WavPack | `*.wv` |
| TrueAudio | `*.tta` |
| DSF / DSDIFF | `*.dsf`, `*.dff`, `*.dsdiff` |
| MOD / Tracker | `*.mod`, `*.module`, `*.nst`, `*.wow`, `*.s3m`, `*.it`, `*.xm` |
| Shorten | `*.shn` |
| Matroska / WebM | `*.mkv`, `*.mka`, `*.webm` |

> [!NOTE]
> Actual supported formats depend on the vendored TagLib build configuration. Unsupported files are skipped in `export metadata`.

### Supported cover image input formats

| Format | Common extensions |
|--------|-------------------|
| JPEG | `*.jpg`, `*.jpeg` |
| PNG | `*.png` |
| GIF | `*.gif` |
| BMP | `*.bmp` |
| WebP | `*.webp` |
| TIFF | `*.tiff`, `*.tif` |

Embedded covers are ultimately written as **JPEG** or **PNG**. The format is selected automatically based on transparency by default; use `--cover-format` to force a specific format.

<a id="global-options"></a>
## Global options and safety behavior

### Write confirmation priority

Destructive commands (`set`, `clear`, `cover set`, `cover clear`, `apply`, `init-manifest`, `export metadata` when writing files) require confirmation. Confirmation sources take priority in this order:

| Priority | Source | Description |
|----------|--------|-------------|
| 1 | `-y` / `--yes` | Explicit command-line confirmation; highest priority |
| 2 | `TAG_CLI_YES=1` or `TAG_CLI_YES=true` | User-level environment variable confirmation |
| 3 | `CI` non-empty and not equal to `false` | CI / automation environment confirmation |

If none of these sources are satisfied, the write command exits with an error and modifies no files.

### Confirmation by command

| Command | Confirmation required? | Notes |
|---------|------------------------|-------|
| `set`, `clear`, `cover set`, `cover clear` | Only for in-place edits | Using `-o` to write a new file skips confirmation |
| `apply` | Yes | Replaces tags on every matched file |
| `init-manifest` | Always | Creates or overwrites the output file |
| `export metadata` | Only when overwriting an existing file | Stdout and new files do not require confirmation |
| `info`, `get`, `cover get`, `list-keys`, `completions`, `man` | No | Read-only |

> [!IMPORTANT]
> `-o` must be a different path than `-i`. If they are the same, the command fails with `output path cannot be the same as input path`.

### Common shared options

| Flag | Purpose | Applicable commands |
|------|---------|---------------------|
| `-v`, `--verbose` | Output DEBUG-level logs | All commands |
| `-i <path>` | Input audio file path or glob pattern | `info`, `get`, `set`, `clear`, `cover`, `export metadata` |
| `-o <path>` | Output path: a file for most commands, a directory for sidecar exports, or an image file for cover get | `set`, `clear`, `cover get`, `cover set`, `cover clear`, `init-manifest`, `export metadata` |
| `-f <format>` | Output format, or compatibility alias for the manifest path in `apply` | `info`, `get`, `list-keys`, `export metadata`, `apply` (alias for `-m`) |
| `-y`, `--yes` | Skip confirmation prompt | Write commands |
| `--dry-run` | Preview changes without writing files | `set`, `clear`, `cover set`, `cover clear`, `apply`; not supported by `init-manifest` or `export metadata` |
| `--replace`, `-R` | Replace mode for `set`: clear every tag except those listed | `set` |

> [!NOTE]
> The exact meaning of `-f` depends on the command. In `info`/`get`/`list-keys`/`export metadata` it selects the output format (`table`, `json`, or `yaml`). In `apply` it is a backward-compatible alias for `-m`/`--manifest`.

<a id="command-reference"></a>
## Command reference

### Summary

| Command | Purpose | Read-only | Confirmation required |
|---------|---------|-----------|-----------------------|
| `list-keys` | List tag keys supported by tag-cli | Yes | No |
| `info` | Show full metadata, audio properties, and embedded pictures | Yes | No |
| `get` | Read specified tag values; outputs all tags when no keys are given | Yes | No |
| `set` | Set one or more tag values; `--replace` clears tags not listed | No | Yes, for in-place edits |
| `clear` | Clear specified tags or all tags and covers | No | Yes, for in-place edits |
| `cover get` | Extract embedded cover to an image file | Yes | No |
| `cover set` | Set embedded cover from an image file | No | Yes, for in-place edits |
| `cover clear` | Remove embedded cover | No | Yes, for in-place edits |
| `apply` | Apply a YAML manifest to one or more files | No | Yes |
| `init-manifest` | Generate a manifest template | No | Always |
| `export metadata` | Export metadata and audio properties for matching files | No (does not modify source audio files) | Only when overwriting output |
| `completions` | Generate shell completion scripts | Yes | No |
| `man` | Generate man page | Yes | No |

### `list-keys`

```bash
$ tag-cli list-keys
$ tag-cli list-keys --format json
$ tag-cli list-keys --format yaml
```

### `info`

Displays full metadata, audio properties, and embedded picture information for a single audio file.

```bash
$ tag-cli info -i song.mp3
$ tag-cli info -i song.flac -f json
$ tag-cli info -i song.m4a -f yaml
```

JSON output structure:

```json
{
  "file": "song.mp3",
  "audio": {
    "length_seconds": 120,
    "bitrate_kbps": 320,
    "sample_rate_hz": 44100,
    "channels": 2
  },
  "tags": {
    "TITLE": ["My Song"],
    "ARTIST": ["Me"]
  },
  "pictures": [
    {
      "mime_type": "image/jpeg",
      "picture_type": "Front Cover",
      "size_bytes": 102400
    }
  ]
}
```

### `get`

Reads one or more tag values. Outputs all tags when no keys are supplied.

```bash
$ tag-cli get -i song.mp3 TITLE ARTIST ALBUM
$ tag-cli get -i song.flac -f yaml
$ tag-cli get -i song.mp3
```

The JSON output uses tag names as keys and lists of strings as values:

```json
{
  "TITLE": ["My Song"],
  "ARTIST": ["Me"],
  "ALBUM": ["My Album"]
}
```

Extract a single value with `jq`:

```bash
$ tag-cli get -i song.mp3 TITLE -f json | jq -r '.TITLE[0]'
My Song
```

### `set`

Sets one or more tags. In-place modification requires confirmation. Use `-o` to write to a new file.

> [!CAUTION]
> `--replace` clears every tag except the ones you list and preserves embedded cover art. Preview with `--dry-run` before using it so you do not accidentally delete needed tags.

```bash
# In-place write (requires confirmation)
$ tag-cli set -i song.mp3 --dry-run TITLE="My Song"
$ tag-cli set -i song.mp3 -y TITLE="My Song" ARTIST="Me" ALBUM="My Album"

# Replace mode: keep only the listed tags
$ tag-cli set -i song.mp3 --dry-run --replace TITLE="My Song" ARTIST="Me"
$ tag-cli set -i song.mp3 -y --replace TITLE="My Song" ARTIST="Me"

# Write to a new file (no confirmation needed)
$ tag-cli set -i song.mp3 -o out.mp3 TITLE="My Song"
```

### `clear`

Clears specified tags. `--all` clears all supported tags and embedded covers.

```bash
$ tag-cli clear -i song.mp3 --dry-run TITLE COMMENT
$ tag-cli clear -i song.mp3 -y TITLE COMMENT
$ tag-cli clear -i song.mp3 --dry-run --all
$ tag-cli clear -i song.mp3 -y --all
```

### `cover get`

Extracts embedded covers. Defaults to `Front Cover`; use `--picture-type` for another type.

```bash
$ tag-cli cover get -i song.mp3 -o cover.jpg
$ tag-cli cover get -i song.mp3 -o back.jpg --picture-type "Back Cover"
```

> [!CAUTION]
> `cover get -o` overwrites the output image file without confirmation. Make sure the target path is correct.

`--picture-type` is a free-form string that must match the picture type stored in the file. Common values from the ID3v2 APIC spec include: `Front Cover`, `Back Cover`, `Leaflet Page`, `Media`, `Lead Artist`, `Lead Performer`, `Artist`, `Conductor`, `Band`, `Orchestra`, `Composer`, `Lyricist`, `Recording Location`, `During Recording`, `During Performance`, `Movie / Video Screen Capture`, `Illustration`, `Publisher Logo`.

### `cover set`

Sets the embedded cover from an image file.

```bash
$ tag-cli cover set -i song.mp3 --dry-run cover.jpg
$ tag-cli cover set -i song.mp3 -y cover.jpg
$ tag-cli cover set -i song.mp3 -o out.mp3 --cover-format jpeg cover.png
```

### `cover clear`

Removes the embedded cover.

```bash
$ tag-cli cover clear -i song.mp3 --dry-run
$ tag-cli cover clear -i song.mp3 -y
```

### `apply`

Applies a YAML manifest to batch-edit files. See [Declarative batch editing with apply](#apply) for the manifest syntax.

> [!WARNING]
> `apply` uses replace semantics: tags not listed in the manifest or `defaults` are cleared. Cover images are re-encoded every run, so file bytes may differ slightly between runs.

```bash
$ tag-cli apply -m manifest.yaml --dry-run
$ tag-cli apply -m manifest.yaml -y
$ tag-cli apply -m manifest.yaml -y --fail-fast
```

### `init-manifest`

Generates a minimal manifest template. `--template` selects a scenario template.

```bash
$ tag-cli init-manifest -y
$ tag-cli init-manifest -y -o manifest.yaml
$ tag-cli init-manifest -y --template classical -o manifest.yaml
```

> [!NOTE]
> `init-manifest` has no `--dry-run` mode and always requires confirmation because it creates or overwrites the output file.

### `export metadata`

Exports metadata and audio properties of files matching a glob pattern. Unsupported files are skipped. Output can go to stdout, a single aggregated file, or per-file sidecars.

```bash
$ tag-cli export metadata -i '*.mp3'
$ tag-cli export metadata -i '*.mp3' -o report.json
$ tag-cli export metadata -i '*.mp3' -o sidecars/ --per-file
$ tag-cli export metadata -i '*.mp3' -o report.yaml --by-album
```

> [!NOTE]
> Output directories (such as `sidecars/`) are created automatically if they do not exist.

> [!CAUTION]
> Writing aggregated reports or sidecar files overwrites existing output and requires confirmation (`-y`, `TAG_CLI_YES=1`, or `CI=true`). Outputting to stdout does not require confirmation.

`--per-file` generates sidecars named `{file_stem}.metadata.{json|yaml}`. If source directories contain files with the same name, they conflict in a single output directory: without `-y`, existing files block later writes; with `-y`, later files silently overwrite earlier ones.

JSON/YAML output structure:

```json
{
  "export_timestamp": "2026-07-02T12:00:00Z",
  "generator": "tag-cli export metadata",
  "summary": {
    "total": 2,
    "succeeded": 2,
    "skipped": 0,
    "failed": 0
  },
  "records": [
    {
      "file_path": "./song.mp3",
      "file_name": "song.mp3",
      "relative_path": "./song.mp3",
      "file_format": "mp3",
      "tags": {
        "title": "Song",
        "artist": "Artist"
      },
      "properties": {
        "TITLE": ["Song"],
        "ARTIST": ["Artist"]
      },
      "audio": {
        "length_seconds": 120,
        "bitrate_kbps": 320,
        "sample_rate_hz": 44100,
        "channels": 2
      },
      "pictures": {
        "count": 1,
        "front_cover_present": true,
        "summaries": [
          {
            "mime_type": "image/jpeg",
            "picture_type": "Front Cover",
            "size_bytes": 102400
          }
        ]
      },
      "read_status": "ok",
      "error_message": null
    }
  ],
  "failures": []
}
```

Field descriptions:

- `file_path`: File path (relative to the working directory by default; absolute when `--absolute-paths` is used).
- `file_name` / `relative_path` / `file_format`: File name, relative path, and extension.
- `tags`: Normalized common tag keys (lowercase / underscore).
- `properties`: Raw tag key-value pairs returned by TagLib (keys are uppercase, values are lists).
- `audio`: Audio properties; may be `null`.
- `pictures`: Embedded picture statistics and summaries.
- `failures`: Files that failed to read, with `error_category` (such as `corrupt_file`, `read_error`) and `error_message`.

### `completions`

Generates shell completion scripts and writes them to stdout.

```bash
$ tag-cli completions bash > /etc/bash_completion.d/tag-cli
$ tag-cli completions zsh > /usr/local/share/zsh/site-functions/_tag-cli
$ tag-cli completions fish > ~/.config/fish/completions/tag-cli.fish
```

### `man`

Generates the tag-cli man page and writes it to stdout.

```bash
$ tag-cli man > tag-cli.1
$ sudo cp tag-cli.1 /usr/local/share/man/man1/
$ mandb  # Debian/Ubuntu
```

<a id="apply"></a>
## Declarative batch editing with apply

> [!WARNING]
> `apply` replaces tags on every matched file. Any tag not listed in the manifest or `defaults` is cleared. Cover images are re-encoded every run.

`apply` declares target file tags and covers via a YAML manifest. Use it for albums, library organization, or CI pipelines.

### Manifest syntax

Top-level fields:

| Field | Type | Description |
|-------|------|-------------|
| `defaults` | Map of strings to strings (e.g., `ARTIST: Example Artist`) | Default tags applied to every file |
| `image_processing` | object | Global cover processing parameters (`format`, `max_size`, `max_file_size`, `quality`) |
| `files` | list | Per-file configuration: `path`, `tags`, `cover`, `picture_type` |
| `paths` | list | Glob patterns, literal files, or directories; resolved relative to the manifest directory |
| `recursive` | boolean | When `true`, recursively expands directories in `paths` |

`files` entry notes:

- `path`: Audio file path (required).
- `tags`: File-level tags that override `defaults`.
- `cover`: Cover image path; omit to leave the cover unchanged.
- `picture_type`: Picture type, such as `Front Cover`, `Back Cover`; defaults to `Front Cover` when omitted.

### Album-style manifest example

```yaml
# album.yaml
defaults:
  ARTIST: "Example Artist"
  ALBUM: "Example Album"
  DATE: "2026"
  GENRE: "Rock"

image_processing:
  format: jpeg
  max_size: 1200
  max_file_size: 1024
  quality: 90

files:
  - path: "01-intro.mp3"
    tags:
      TITLE: "Intro"
      TRACKNUMBER: "1"
    cover: "artwork.jpg"

  - path: "02-main.flac"
    tags:
      TITLE: "Main Track"
      TRACKNUMBER: "2"
    cover: "artwork.jpg"

  - path: "03-outro.mp3"
    tags:
      TITLE: "Outro"
      TRACKNUMBER: "3"
    cover: "artwork.jpg"

# You can also use paths to match directories or globs
paths:
  - "bonus/*.mp3"
recursive: false
```

### Running examples

```bash
# Preview (no write)
$ tag-cli apply -m album.yaml --dry-run

# Apply for real
$ tag-cli apply -m album.yaml -y

# Stop on the first failure
$ tag-cli apply -m album.yaml -y --fail-fast
```

### Path resolution rules

- Relative paths in the manifest are resolved relative to the manifest file's directory. Absolute paths are preserved as-is.
- `paths` supports literal files, literal directories, and glob patterns containing `*`, `?`, `[...]`, and `**` (recursive).
- Literal files or directories take precedence over glob interpretation: if a real directory named `[2023] Album` exists, it is traversed as a directory rather than parsed as a character-class glob.
- `recursive: true` recursively traverses directories in `paths`.
- Nonexistent literal paths or globs with no matches silently produce zero matches. Verify paths first with `info` or a small `--dry-run`.

### Report semantics

After the batch run, `apply` outputs three lines of statistics:

```text
Success: N
Skipped: N   # Non-zero only during --dry-run
Failures: N
```

Each line is followed by an indented list of per-file statuses (`ok`, `skip (dry-run)`, or `err`).

- `Success`: Files written successfully.
- `Skipped`: Files skipped during `--dry-run` preview.
- `Failures`: Files that failed to read, process, or save.

If `Failures > 0`, `apply` returns a non-zero exit code.

### Error handling and idempotency

- `--fail-fast` stops on the first failure. It does not roll back files already processed.
- `apply` writes each track as a full replacement, so applying the same manifest multiple times produces consistent metadata results and can be considered idempotent. However, cover images are re-encoded every time, so file bytes may differ slightly between runs.
- If a file appears in both `files` and `paths`, it is processed multiple times. Avoid duplicate entries.

<a id="init-manifest-templates"></a>
## init-manifest templates

`init-manifest --template <name>` generates scenario-oriented manifest templates pre-populated with relevant fields.

| Template name | Description |
|---------------|-------------|
| `classical` | Classical music template, including composer / conductor / movement fields |
| `podcast` | Podcast template, including podcast-related fields |
| `radio` | Radio show template |
| `education` | Educational content template |
| `vinyl` | Vinyl release template |
| `release` | General music release template |

```bash
$ tag-cli init-manifest -y --template release -o manifest.yaml
```

<a id="image-processing"></a>
## Image processing behavior

By default, cover images are processed before embedding:

- **Format selection**: Non-transparent images default to **JPEG**. Images with an alpha channel default to **PNG**. Use `--cover-format jpeg|png` to override.
- **Size limits**: Images are scaled down proportionally so that neither width nor height exceeds the format-specific cap:
  - MPEG / WAV / AIFF / WMA / APE / MPC / WavPack / TrueAudio / DSF / MOD / Shorten / Matroska / unknown formats: **1200x1200**
  - MP4 / FLAC / Ogg Vorbis / Ogg Opus / Ogg FLAC / Speex: **2048x2048**
- **File size limits**: Output is kept within the format-specific cap when possible:
  - MPEG / WAV / AIFF / WMA / APE / MPC / WavPack / TrueAudio / DSF / MOD / Shorten / Matroska / unknown formats: **1 MB**
  - MP4 / FLAC / Ogg Vorbis / Ogg Opus / Ogg FLAC / Speex: **2 MB**
- **JPEG quality**: Initial quality is **90**. If the size limit is still exceeded, quality is reduced automatically, but never below **30**. If the output still exceeds `--cover-max-file-size` at quality 30, the file is embedded at its actual size.
- **PNG size reduction**: PNG file size is reduced by progressively scaling down dimensions (minimum 100 px). Quality is not reduced; `--cover-quality` has no effect on PNG.
- **EXIF/metadata stripping**: JPEG and PNG inputs can have metadata stripped losslessly when no scaling or format conversion occurs. WebP and other inputs are re-encoded, which also removes metadata.
- **Bypass processing**: `--no-process-cover` embeds the image file as-is without scaling, format conversion, or metadata stripping. It also skips validation that the file is a supported image format, so misuse may write non-image data into tags.

Override default limits with:

| Option | Description |
|--------|-------------|
| `--cover-max-size <N>` | Maximum edge length in pixels |
| `--cover-max-file-size <KB>` | Maximum file size in KB |
| `--cover-quality <1-100>` | JPEG quality; ignored for PNG encoding |

> [!WARNING]
> `--no-process-cover` may produce covers that exceed player or target-format recommendations, causing compatibility issues. Use it only when you specifically need to preserve the original file bytes.

### Cover processing under `--dry-run`

`apply --dry-run` still fully decodes and re-encodes cover images specified in the manifest. This validates paths, formats, and size limits. Missing or corrupt cover images cause `--dry-run` to fail, even though no audio files are modified. Previewing large manifests with many high-resolution covers may be slow.

<a id="batch-safety-checklist"></a>
## Batch editing safety checklist

Before using batch or destructive commands such as `apply`, `set`, `clear`, and `cover set/clear`, follow this checklist:

1. **Back up original files.** tag-cli does not back up automatically and cannot undo writes.
2. **Test on a small set.** Run `--dry-run` on 1–3 files first and verify the diff.
3. **Dry-run first, `-y` second.** Always run `--dry-run` first; only add `-y` after confirming the diff. `--dry-run` does not test the write path, so filesystem permissions and read-only files only surface during `-y`.
4. **Understand confirmation source priority.** Command-line `-y` > `TAG_CLI_YES` > `CI`. In CI, use `--dry-run` as an explicit validation step, then use `TAG_CLI_YES=1` or `-y` for writes.
5. **Check manifest paths.** Relative paths are resolved relative to the manifest directory. Nonexistent paths or empty globs silently produce zero matches.
6. **Validate cover processing.** Verify that `--cover-format`, `--cover-max-size`, and `--cover-max-file-size` meet target platform requirements. PNG size reduction lowers resolution; `--cover-quality` affects only JPEG.
7. **Watch for sidecar name collisions.** `export metadata --per-file` names files `{file_stem}.metadata.{ext}`. Files with the same name overwrite each other.
8. **Failures are not rolled back.** `apply --fail-fast` stops at the first failure, but files processed before that point are already modified.

<a id="automation-examples"></a>
## Automation examples

### Safer batch edits with `find -exec`

Use `find -exec` to avoid shell-quoting issues with special characters in file names.

```bash
# Preview all files first
$ find ./album -name '*.mp3' -exec tag-cli set -i {} --dry-run ALBUMARTIST="Various Artists" \;

# Write after confirming the preview
$ find ./album -name '*.mp3' -exec tag-cli set -i {} -y ALBUMARTIST="Various Artists" \;
```

For more complex edits, prefer a manifest plus `apply`.

### Batch-apply manifest and stop on failure

```bash
#!/usr/bin/env bash
set -euo pipefail

MANIFEST="library.yaml"

# Independent validation step: abort if dry-run fails
if ! tag-cli apply -m "$MANIFEST" --dry-run; then
  echo "Dry-run failed, aborting" >&2
  exit 1
fi

# Write for real: explicitly use TAG_CLI_YES=1 instead of relying on implicit CI behavior
tag-cli apply -m "$MANIFEST" -y --fail-fast
```

### Replace all tags safely

> [!CAUTION]
> `--replace` clears every tag except those you explicitly list and preserves embedded cover art. Preview carefully.

```bash
$ tag-cli set -i song.mp3 --dry-run --replace TITLE="New Title" ARTIST="New Artist"
$ tag-cli set -i song.mp3 -y --replace TITLE="New Title" ARTIST="New Artist"
```

### Parse `export metadata` JSON output

```bash
# Output title and artist for every song
$ tag-cli export metadata -i '*.mp3' -f json | jq '.records[] | {file: .file_path, title: .tags.title, artist: .tags.artist}'

# Count files without a front cover
$ tag-cli export metadata -i '*.mp3' -f json | jq '[.records[] | select(.pictures.front_cover_present == false)] | length'
```

`apply` prints a text report (`Success / Skipped / Failures`) by default. Scripts can check the exit code and stderr text to detect failures.

### CI workflow best practices

Use `--dry-run` as a separate validation job that fails the pipeline. Use `TAG_CLI_YES=1` explicitly for the write step.

```yaml
# .github/workflows/tag-cli.yml example snippet
jobs:
  dry-run:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Validate manifest
        run: tag-cli apply -m album.yaml --dry-run

  apply:
    needs: dry-run
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Apply metadata
        run: tag-cli apply -m album.yaml -y
        env:
          TAG_CLI_YES: "1"
```

### Export metadata report in CI

```yaml
# .github/workflows/metadata.yml example snippet
- name: Export metadata report
  run: tag-cli export metadata -i 'audio/**/*.mp3' -o metadata-report.json -y
  env:
    TAG_CLI_YES: "1"
```

<a id="exit-codes-and-environment-variables"></a>
## Exit codes and environment variables

### Exit codes

| Code | Meaning |
|------|---------|
| `0` | Command completed successfully |
| `1` | Command failed (argument error, missing confirmation, I/O error, TagLib error, failures in `apply`/`export metadata`, etc.) |

Specific behavior for `apply` and `export metadata`:

| Scenario | Exit code |
|----------|-----------|
| All succeeded | `0` |
| Some files failed (default: continue processing) | `1` |
| `--fail-fast` stops at the first failure | `1` |
| Read, parse, or cover processing fails during `--dry-run` | `1` |
| Output file exists but was not confirmed | `1` |
| No files matched | `0` (`export metadata` prints a warning and returns success) |

> [!NOTE]
> The current implementation does not distinguish finer-grained exit codes (for example, `2` for argument errors or `3` for missing confirmation). Any failure returns `1`. Combine stderr output with `--dry-run` for judgment.

### Environment variables

| Variable | Value | Purpose |
|----------|-------|---------|
| `TAG_CLI_YES` | `1` or `true` | Treat destructive write operations as confirmed |
| `CI` | Any non-empty value except `false` | Treat as confirmed in CI/automation environments |

### Priority examples

```bash
# Explicit -y always takes precedence
$ tag-cli set -i song.mp3 -y TITLE="A"

# Environment variable confirmation
$ TAG_CLI_YES=1 tag-cli set -i song.mp3 TITLE="A"

# CI environment confirmation (no need to pass -y)
$ CI=true tag-cli apply -m manifest.yaml
```

> [!CAUTION]
> Setting `CI=true` in CI treats all destructive commands in that job as confirmed. Use it only for read-only steps (such as `info` or `export metadata` to stdout) or for write steps that have already passed `--dry-run` validation. For actual write steps, prefer explicit `TAG_CLI_YES=1` or `-y` to clearly express intent.

<a id="troubleshooting-and-faq"></a>
## Troubleshooting / FAQ

### Build-time error: TagLib not found or `vendor/taglib` is empty

Initialize submodules first:

```bash
git submodule update --init --recursive
```

### `cmake` or `zlib` errors

- Debian/Ubuntu: `sudo apt-get install -y cmake zlib1g-dev` (building from source also requires `clang` or `build-essential`).
- macOS: Xcode Command Line Tools provide `zlib` and the C++ toolchain, but `cmake` usually needs to be installed separately: `brew install cmake`. If C++ headers are reported missing, run `xcode-select --install`.

### Write command reports confirmation required

Write commands require `-y`, `TAG_CLI_YES=1`, or `CI=true`. See [Global options and safety behavior](#global-options).

### Why does a 1-second MP3 show a longer duration?

TagLib reports the duration stored in the MP3 frame headers. This value may include encoder delay and padding added by some encoders (such as LAME), so very short MP3s may appear slightly longer than the actual audio content. The exact value depends on the encoder and TagLib version.

### Automatic backups

tag-cli **does not** create backups automatically. Back up original files before batch writes.

### Difference between corrupt files and unsupported formats

`export metadata` distinguishes them heuristically by file extension. Known audio extensions that cannot be read are marked as `corrupt_file`. Unknown extensions are treated as `skipped`.

### Input file does not exist or cannot be read

For commands such as `info`, `get`, `set`, `clear`, and `cover`, when the file does not exist, the path is wrong, or the format is not recognized by TagLib, they usually output `TagLib error: file is not a valid/recognized audio file for TagLib` and return exit code `1`. `export metadata` records such files in the `failures` array and returns a non-zero exit code only when `Failures > 0`.

<a id="contributing"></a>
## Contributing

Development, build, and contribution guidelines are in [CONTRIBUTING.md](CONTRIBUTING.md). That document is currently in Chinese; non-Chinese speakers are welcome to open an issue or discussion in English.

- Report bugs or request features: [Issues](https://github.com/MunMunMiao/tag-cli/issues)
- Propose changes: [Pull requests](https://github.com/MunMunMiao/tag-cli/pulls)
- Ask questions or share ideas: [Discussions](https://github.com/MunMunMiao/tag-cli/discussions)

<a id="license"></a>
## License

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

tag-cli is licensed under the [MIT License](LICENSE).
