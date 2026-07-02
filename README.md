# tag-cli

A Rust command-line tool for reading and writing audio metadata and embedded cover art via a vendored [TagLib](https://taglib.org/) C++ library.

<a id="table-of-contents"></a>
## Table of contents

- [Features](#features)
- [Quick start](#quick-start)
- [Prerequisites](#prerequisites)
- [Installation](#installation)
- [Installation verification](#installation-verification)
- [Supported audio formats and cover image input formats](#supported-formats)
- [Global options and safety behavior](#global-options)
- [Command reference](#command-reference)
- [Declarative batch editing with apply](#apply)
- [init-manifest templates](#init-manifest-templates)
- [Image processing behavior](#image-processing)
- [Batch editing safety checklist](#batch-safety-checklist)
- [Automation examples](#automation-examples)
- [Exit codes and environment variables](#exit-codes-and-environment-variables)
- [Troubleshooting / FAQ](#troubleshooting-and-faq)
- [License](#license)

<a id="features"></a>
## Features

- Read and write metadata tags and embedded cover art for common audio formats.
- Declarative batch editing for entire albums or music libraries via YAML manifest.
- Automatic cover processing: scaling, format selection, EXIF/metadata stripping, and size limits.
- Output formats support human-readable tables, JSON, and YAML.
- All write operations support `--dry-run` preview.
- Generate scenario-oriented manifest templates via `init-manifest --template`.
- Supports environment variable confirmation for scripting and CI automation (`TAG_CLI_YES=1` or `CI=true`).

> **Safety note:** tag-cli **does not** create backups automatically. Before writing, preview with `--dry-run` and back up important files yourself.

<a id="safety-cheatsheet"></a>
### Safety cheat sheet: three steps for write operations

```text
1. Back up original files  →  cp -r music music.bak
2. Preview changes         →  tag-cli ... --dry-run
3. Confirm and write       →  tag-cli ... -y
```

- `--dry-run` does not modify any files, but `apply` still decodes/re-encodes cover images for validation; missing or corrupt covers will cause preview failures.
- Any write or overwrite command (`set`, `clear`, `cover set/clear`, `apply`, `init-manifest`, `export metadata` when writing files) requires confirmation via `-y`, `TAG_CLI_YES=1`, or `CI=true`.

<a id="quick-start"></a>
## Quick start

```bash
# 1. Verify installation
$ tag-cli --version
tag-cli 0.1.0

# 2. Read full metadata of a file
$ tag-cli info -i song.mp3

# 3. Preview changes (will not write to file)
$ tag-cli set -i song.mp3 --dry-run TITLE="My Song" ARTIST="Me"
```

After confirming the preview output is correct, perform the actual write:

```bash
# ⚠️ This command modifies song.mp3 in place; be sure to run the --dry-run above first and check the diff.
$ tag-cli set -i song.mp3 -y TITLE="My Song" ARTIST="Me"
```

For first-time use, follow the `--dry-run` → `-y` sequence, confirming the diff matches expectations before actually writing.

<a id="prerequisites"></a>
## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) stable toolchain (MSRV 1.85+)
- C++ toolchain
- `cmake`
- `zlib` development headers
- `vendor/taglib` Git submodule initialized

### Platform notes

- **macOS**: Xcode Command Line Tools provide the C++ toolchain and `zlib`; `cmake` is not installed with CLT by default and usually needs to be installed separately:
  ```bash
  brew install cmake
  ```
- **Linux (Debian/Ubuntu)**: Install system dependencies:
  ```bash
  sudo apt-get update && sudo apt-get install -y cmake clang zlib1g-dev
  # or
  sudo apt-get update && sudo apt-get install -y build-essential cmake zlib1g-dev
  ```
- **Windows**: Requires the Visual Studio C++ toolchain, CMake, and zlib; building from source in MSYS2/WSL or PowerShell is recommended.

<a id="installation"></a>
## Installation

### Install from source

```bash
git clone https://github.com/MunMunMiao/tag-cli.git
cd tag-cli
git submodule update --init --recursive
cargo install --path crates/tag-cli
```

The first full build compiles the vendored TagLib C++ library, taking about 3–10 minutes (depending on CPU); the `target/` directory may use several GB of disk space. `cargo install` installs the `tag-cli` binary to `~/.cargo/bin`; make sure that directory is in your `PATH`, for example:

```bash
# bash / zsh
export PATH="$HOME/.cargo/bin:$PATH"
```

To uninstall:

```bash
cargo uninstall tag-cli
```

### Pre-built binaries

Download the archive for your platform from [GitHub Releases](https://github.com/MunMunMiao/tag-cli/releases).

#### Linux / macOS

```bash
VERSION=0.1.0  # Replace with the latest version from GitHub Releases
TARGET=x86_64-unknown-linux-gnu  # or x86_64-apple-darwin, aarch64-apple-darwin
curl -LO "https://github.com/MunMunMiao/tag-cli/releases/download/v${VERSION}/tag-cli-${VERSION}-${TARGET}.tar.gz"
tar xzf "tag-cli-${VERSION}-${TARGET}.tar.gz"
cd "tag-cli-${VERSION}-${TARGET}"
./tag-cli --help
```

To call it globally, add that directory to your `PATH`, for example:

```bash
# bash / zsh
export PATH="$(pwd):$PATH"
# Or add permanently to ~/.bashrc / ~/.zshrc
```

#### Windows (PowerShell)

```powershell
$VERSION="0.1.0"  # Replace with the latest version from GitHub Releases
$TARGET="x86_64-pc-windows-msvc"
Invoke-WebRequest -Uri "https://github.com/MunMunMiao/tag-cli/releases/download/v${VERSION}/tag-cli-${VERSION}-${TARGET}.zip" -OutFile "tag-cli-${VERSION}-${TARGET}.zip"
Expand-Archive -Path "tag-cli-${VERSION}-${TARGET}.zip" -DestinationPath "tag-cli-${VERSION}-${TARGET}"
cd "tag-cli-${VERSION}-${TARGET}"
.\tag-cli.exe --help
```

To call it globally, add the extracted directory to your `PATH`:

```powershell
# Current session
$env:PATH = "$PWD;$env:PATH"
# Add permanently to user PATH (PowerShell 7+)
[Environment]::SetEnvironmentVariable("PATH", "$PWD;$env:PATH", "User")
```

<a id="installation-verification"></a>
## Installation verification

After successful installation, the following commands should produce normal output:

```bash
$ tag-cli --version
tag-cli 0.1.0

$ tag-cli list-keys
TITLE
ARTIST
ALBUM
...
```

If `list-keys` prints supported key names, TagLib FFI is correctly linked.

tag-cli does not currently read additional configuration files; all options are provided via command-line arguments or environment variables.

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

> Actual supported formats depend on the vendored TagLib build configuration; unsupported files will be skipped in `export metadata`.

### Supported cover image input formats

| Format | Common extensions |
|--------|-------------------|
| JPEG | `*.jpg`, `*.jpeg` |
| PNG | `*.png` |
| GIF | `*.gif` |
| BMP | `*.bmp` |
| WebP | `*.webp` |
| TIFF | `*.tiff`, `*.tif` |

Embedded covers are ultimately written as **JPEG** or **PNG** (automatically selected based on transparency by default; use `--cover-format` to force a specific format).

<a id="global-options"></a>
## Global options and safety behavior

### Write confirmation priority

All commands that modify files or overwrite output (`set`, `clear`, `cover set`, `cover clear`, `apply`, `init-manifest`, `export metadata` when writing files) require confirmation. Confirmation sources take effect in the following priority order:

| Priority | Source | Description |
|----------|--------|-------------|
| 1 | `-y` / `--yes` | Explicit command-line confirmation; highest priority |
| 2 | `TAG_CLI_YES=1` or `TAG_CLI_YES=true` | User-level environment variable confirmation |
| 3 | `CI` non-empty and not equal to `false` | CI / automation environment confirmation |

If none of the three sources are satisfied, the write command exits with an error and does not modify any files.

### Common shared options

| Flag | Purpose | Applicable commands |
|------|---------|---------------------|
| `-v`, `--verbose` | Output DEBUG-level logs | All commands |
| `-i <path>` | Input audio file path or glob pattern | `info`, `get`, `set`, `clear`, `cover`, `export metadata` |
| `-o <path>` | Output file, directory, or sidecar directory | `set`, `cover get`, `cover set`, `init-manifest`, `export metadata` |
| `-f <format>` | Output format, or compatibility alias for the manifest path in `apply` | `info`, `get`, `list-keys`, `export metadata`, `apply` (as alias for `-m`) |
| `-y`, `--yes` | Skip confirmation prompt | Write commands |
| `--dry-run` | Preview changes without writing files | `set`, `clear`, `cover set`, `cover clear`, `apply`; not supported by `init-manifest` or `export metadata` |

> The exact meaning of `-f` depends on the command: in `info`/`get`/`list-keys`/`export metadata` it means `--format` (`table`/`json`/`yaml`); in `apply` it is kept as a backward-compatible alias for `-m`/`--manifest`. `export metadata` still requires confirmation (`-y`/`TAG_CLI_YES`/`CI`) when writing aggregated reports or sidecars.

<a id="command-reference"></a>
## Command reference

The table below summarizes all commands; directly runnable examples follow each command.

| Command | Purpose | Source file read-only | Confirmation required |
|---------|---------|----------------------|-----------------------|
| `list-keys` | List tag keys supported by tag-cli | Yes | No |
| `info` | Show full metadata, audio properties, and embedded pictures for a file | Yes | No |
| `get` | Read specified tag values; outputs all tags when no keys are given | Yes | No |
| `set` | Set one or more tag values | No | Yes |
| `clear` | Clear specified tags or all tags and covers | No | Yes |
| `cover get` | Extract embedded cover to an image file | Yes | No |
| `cover set` | Set embedded cover from an image file | No | Yes |
| `cover clear` | Remove embedded cover | No | Yes |
| `apply` | Apply a YAML manifest to one or more files | No | Yes |
| `init-manifest` | Generate a manifest template | No | Yes |
| `export metadata` | Export metadata and audio properties for matching files | Source file is read-only | Required when writing reports or sidecars |
| `completions` | Generate shell completion scripts | Yes | No |
| `man` | Generate man page | Yes | No |

### `list-keys`

Lists all tag keys supported by tag-cli.

```bash
$ tag-cli list-keys
$ tag-cli list-keys --format json
$ tag-cli list-keys --format yaml
```

### `info`

Displays full metadata, audio properties, and embedded picture information for a single audio file; useful for getting the complete picture of a file.

```bash
$ tag-cli info -i song.mp3
$ tag-cli info -i song.flac -f json
$ tag-cli info -i song.m4a -f yaml
```

JSON output structure example:

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

Reads and outputs the values of one or more specified tags; suitable for scripting. Outputs all tags when no keys are specified.

```bash
$ tag-cli get -i song.mp3 TITLE ARTIST ALBUM
$ tag-cli get -i song.flac -f yaml
$ tag-cli get -i song.mp3
```

The JSON output is an object with tag names as keys and lists of strings as values:

```json
{
  "TITLE": ["My Song"],
  "ARTIST": ["Me"],
  "ALBUM": ["My Album"]
}
```

To extract a single value with `jq`:

```bash
$ tag-cli get -i song.mp3 TITLE -f json | jq -r '.TITLE[0]'
My Song
```

### `set`

Sets one or more tags. In-place modification requires confirmation; use `-o` to write to a new file.

> ⚠️ **`--replace` clears all tags not listed**. Be sure to preview with `--dry-run` before using this flag to confirm you won't accidentally delete needed tags.

```bash
$ tag-cli set -i song.mp3 -y TITLE="My Song" ARTIST="Me" ALBUM="My Album"
$ tag-cli set -i song.mp3 --dry-run TITLE="My Song"

# The following command keeps TITLE/ARTIST and deletes all other tags in the file
$ tag-cli set -i song.mp3 -y --replace TITLE="My Song" ARTIST="Me"

$ tag-cli set -i song.mp3 -o out.mp3 -y TITLE="My Song"
```

### `clear`

Clears specified tags; `--all` clears all supported tags and embedded covers.

```bash
$ tag-cli clear -i song.mp3 -y TITLE COMMENT
$ tag-cli clear -i song.mp3 --dry-run TITLE COMMENT
$ tag-cli clear -i song.mp3 -y --all
```

### `cover get`

Extracts embedded covers. Defaults to `Front Cover`; use `--picture-type` to specify another type.

```bash
$ tag-cli cover get -i song.mp3 -o cover.jpg
$ tag-cli cover get -i song.mp3 -o back.jpg --picture-type "Back Cover"
```

`--picture-type` is a free-form string that must match the picture type name stored in the audio file. Common values (from the ID3v2 APIC spec) include: `Front Cover`, `Back Cover`, `Leaflet Page`, `Media` (such as a CD label scan), `Lead Artist` / `Lead Performer`, `Artist`, `Conductor`, `Band` / `Orchestra`, `Composer`, `Lyricist`, `Recording Location`, `During Recording`, `During Performance`, `Movie / Video Screen Capture`, `Illustration`, `Publisher Logo`, etc.

### `cover set`

Sets the embedded cover from an image file.

```bash
$ tag-cli cover set -i song.mp3 -y cover.jpg
$ tag-cli cover set -i song.mp3 --dry-run cover.jpg
$ tag-cli cover set -i song.mp3 -o out.mp3 -y --cover-format jpeg cover.png
```

### `cover clear`

Removes the embedded cover.

```bash
$ tag-cli cover clear -i song.mp3 -y
$ tag-cli cover clear -i song.mp3 --dry-run
```

### `apply`

Applies a YAML manifest to batch-edit files. See [Declarative batch editing with apply](#apply) for detailed syntax.

```bash
$ tag-cli apply -m manifest.yaml -y
$ tag-cli apply -m manifest.yaml --dry-run
$ tag-cli apply -m manifest.yaml -y --fail-fast
```

### `init-manifest`

Generates a minimal manifest template; `--template` selects a scenario template.

```bash
$ tag-cli init-manifest -y
$ tag-cli init-manifest -y -o manifest.yaml
$ tag-cli init-manifest -y --template classical -o manifest.yaml
```

### `export metadata`

Exports metadata and audio properties of audio files matching a glob pattern. Unsupported files are skipped; output can go to stdout, a single aggregated file, or per-file sidecars.

```bash
$ tag-cli export metadata -i '*.mp3'
$ tag-cli export metadata -i '*.mp3' -o report.json
$ tag-cli export metadata -i '*.mp3' -o sidecars/ --per-file
$ tag-cli export metadata -i '*.mp3' -o report.yaml --by-album
```

**Output notes:**

- Output directories (such as `sidecars/`) are created automatically if they do not exist.
- Writing aggregated reports or sidecar files overwrites existing output and requires confirmation (`-y`, `TAG_CLI_YES=1`, or `CI=true`). Outputting to stdout only does not require confirmation.
- `--per-file` generates sidecar files named `{file_stem}.metadata.{json|yaml}`. If source directories contain files with the same name, they will conflict in a single output directory: without `-y`, existing files block subsequent writes; with `-y`, later files silently overwrite earlier ones.

**JSON/YAML output structure example:**

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
- `failures`: List of files that failed to read, including `error_category` (such as `corrupt_file`, `read_error`) and `error_message`.

### `completions`

Generates shell completion scripts and outputs them to stdout.

```bash
$ tag-cli completions bash > /etc/bash_completion.d/tag-cli
$ tag-cli completions zsh > /usr/local/share/zsh/site-functions/_tag-cli
$ tag-cli completions fish > ~/.config/fish/completions/tag-cli.fish
```

### `man`

Generates the tag-cli man page and outputs it to stdout.

```bash
$ tag-cli man > tag-cli.1
$ sudo cp tag-cli.1 /usr/local/share/man/man1/
$ mandb  # Debian/Ubuntu
```

<a id="apply"></a>
## Declarative batch editing with apply

`apply` declares target file tags and covers via a YAML manifest, suitable for entire albums, library organization, or CI pipelines.

### Manifest syntax

Top-level fields:

| Field | Type | Description |
|-------|------|-------------|
| `defaults` | `map<string, string>` | Default tags applied to every file |
| `image_processing` | object | Global cover processing parameters (`format`, `max_size`, `max_file_size`, `quality`) |
| `files` | list | Per-file configuration: includes `path`, `tags`, `cover`, `picture_type` |
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

- Relative paths in the manifest are resolved relative to the manifest file's directory; absolute paths are preserved as-is.
- `paths` supports literal files, literal directories, and glob patterns containing `*`, `?`, and `[...]`.
- Literal files or directories take precedence over glob interpretation: if a real directory named `[2023] Album` exists, it is traversed as a directory rather than parsed as a character-class glob.
- `recursive: true` recursively traverses directories in `paths`.
- Nonexistent literal paths or globs with no matches silently produce zero matches; verify paths first with `info` or a small `--dry-run` to ensure they hit the intended files.

### Report semantics

After the batch run, `apply` outputs three lines of statistics:

```text
Success: N
Skipped: N   # Only appears during --dry-run
Failures: N
```

Each line is followed by an indented list of per-file statuses (`ok`, `skip (dry-run)`, or `err`).

- `Success`: Number of files written successfully.
- `Skipped`: Number of files skipped during `--dry-run` preview.
- `Failures`: Number of files that failed to read, process, or save.

As long as `Failures > 0`, `apply` returns a non-zero exit code so scripts can detect it.

### Error handling and idempotency

- `--fail-fast` stops processing subsequent files on the first failure; by default, processing continues and a summary report is returned. `--fail-fast` does not roll back already processed files; files before the failure point have already been modified.
- `apply` writes each track as a full replacement (`replace: true`), with tag values fixed by the manifest, so applying the same manifest multiple times produces consistent results and can be considered idempotent. **However, cover images are re-encoded every time**, so file bytes may differ slightly between runs even when metadata has not changed.
- If a file appears in both `files` and `paths`, it will be processed multiple times; avoid duplicate entries.

<a id="init-manifest-templates"></a>
## init-manifest templates

`init-manifest --template <name>` generates scenario-oriented manifest templates, pre-populated with relevant fields.

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

By default, cover images undergo the following processing before embedding:

- **Format selection**: Non-transparent images default to **JPEG**; images with an alpha channel default to **PNG**. Use `--cover-format jpeg|png` to override.
- **Size limits**: Images are scaled down proportionally so that neither width nor height exceeds the format-specific cap:
  - MPEG / WAV / AIFF / WMA / APE / MPC / WavPack / TrueAudio / DSF / MOD / Shorten / Matroska / unknown formats: **1200x1200**
  - MP4 / FLAC / Ogg Vorbis / Ogg Opus / Ogg FLAC / Speex: **2048x2048**
- **File size limits**: Output is kept within the format-specific cap when possible:
  - MPEG / WAV / AIFF / WMA / APE / MPC / WavPack / TrueAudio / DSF / MOD / Shorten / Matroska / unknown formats: **1 MB**
  - MP4 / FLAC / Ogg Vorbis / Ogg Opus / Ogg FLAC / Speex: **2 MB**
- **JPEG quality**: Initial quality is **90**; if the size limit is still exceeded, quality is automatically reduced, but never below **30**. If the output still exceeds `--cover-max-file-size` at quality 30, the file is embedded at its actual size (which may exceed the limit).
- **PNG size reduction**: PNG file size is reduced by progressively scaling down dimensions (minimum 100 px); **quality is not reduced**; `--cover-quality` has no effect on PNG.
- **EXIF/metadata stripping**: JPEG and PNG inputs can have metadata stripped losslessly when no scaling or format conversion occurs; WebP and other inputs are re-encoded, which also removes metadata.
- **Bypass processing**: `--no-process-cover` embeds the image file as-is without scaling, format conversion, or metadata stripping; it also skips validation that the file is a supported image format, so misuse may write non-image data into tags.

The following options override the default limits:

| Option | Description |
|--------|-------------|
| `--cover-max-size <N>` | Maximum edge length in pixels |
| `--cover-max-file-size <KB>` | Maximum file size in KB |
| `--cover-quality <1-100>` | JPEG quality; ignored for PNG encoding |

> Warning: `--no-process-cover` may produce covers that exceed player or target-format recommendations, causing compatibility issues; use it only when you specifically need to preserve the original file bytes.

### Cover processing under `--dry-run`

`apply --dry-run` still fully decodes and re-encodes cover images specified in the manifest to validate paths, formats, and size limits. Therefore:

- Missing or corrupt cover images cause `--dry-run` to fail, even though no audio files are modified.
- Previewing large manifests with many high-resolution covers may be slow.

<a id="batch-safety-checklist"></a>
## Batch editing safety checklist

Before using batch or destructive commands such as `apply`, `set`, `clear`, and `cover set/clear`, follow this checklist:

1. **Back up original files**: tag-cli does not back up automatically and cannot undo writes; back up important libraries before running `-y` for real, not just before `--dry-run`.
2. **Test on a small set**: Run `--dry-run` on 1–3 files first and verify the diff.
3. **Dry-run first, `-y` second**: Always run `--dry-run` first to review changes; only add `-y` for the actual write after confirming the diff. `--dry-run` does not test the write path, so filesystem permissions, read-only files, and similar errors only surface during `-y`.
4. **Understand confirmation source priority**: Command-line `-y` > `TAG_CLI_YES` > `CI`. To avoid accidental overwrites in CI, use `--dry-run` as an explicit validation step in scripts.
5. **Check manifest paths**: Relative paths are resolved relative to the manifest directory; nonexistent paths or empty globs silently produce zero matches. Confirm paths exist before running on another machine or in CI.
6. **Validate cover processing**: For high-resolution covers, verify that `--cover-format`, `--cover-max-size`, and `--cover-max-file-size` meet target platform requirements. PNG size reduction is achieved by lowering resolution; `--cover-quality` affects only JPEG.
7. **Watch for sidecar name collisions**: `export metadata --per-file` names files `{file_stem}.metadata.{ext}`; files with the same name will overwrite each other.
8. **Failures are not rolled back**: `apply --fail-fast` stops at the first failure, but files processed before that point have already been modified; there is no automatic rollback.

<a id="automation-examples"></a>
## Automation examples

### Batch-set album artist with `find`

Preview all files first, then write after confirming correctness:

```bash
# 1. Preview (no write)
find ./album -name '*.mp3' -print0 | while IFS= read -r -d '' f; do
  tag-cli set -i "$f" --dry-run ALBUMARTIST="Various Artists" ARTIST="$(basename "$f" .mp3)"
done

# 2. Write for real after confirming
find ./album -name '*.mp3' -print0 | while IFS= read -r -d '' f; do
  tag-cli set -i "$f" -y ALBUMARTIST="Various Artists" ARTIST="$(basename "$f" .mp3)"
done
```

> Special characters in file names (such as `"`, `$`, newlines) may be injected into shell arguments via `basename`. A safer approach is to let `find` execute the command directly: `find ./album -name '*.mp3' -exec tag-cli set -i {} -y ALBUMARTIST="Various Artists" \;`.

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

### Parse `export metadata` JSON output

```bash
# Output title and artist for every song
tag-cli export metadata -i '*.mp3' -f json | jq '.records[] | {file: .file_path, title: .tags.title, artist: .tags.artist}'

# Count files without a front cover
tag-cli export metadata -i '*.mp3' -f json | jq '[.records[] | select(.pictures.front_cover_present == false)] | length'
```

`apply` prints a text report (`Success / Skipped / Failures`) by default, not JSON; scripts can check the exit code and stderr text to detect failures.

### CI workflow best practices

Use `--dry-run` as a separate validation job/step that fails the pipeline; use `TAG_CLI_YES=1` explicitly for the actual write step:

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
  run: tag-cli export metadata -i 'audio/**/*.mp3' -o metadata-report.json
  env:
    CI: true
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
| Some files failed (default: continue processing remaining files) | `1` |
| `--fail-fast` stops at the first failure | `1` |
| Read, parse, or cover processing fails during `--dry-run` | `1` (dry-run failures also return non-zero) |
| Output file exists but was not confirmed | `1` |
| No files matched | `0` (`export metadata` prints a warning and returns success) |

> The current implementation does not distinguish finer-grained exit codes (for example, `2` for argument errors or `3` for missing confirmation); any failure returns `1`. Scripts should combine stderr output with `--dry-run` for judgment. If you need to distinguish error types, inspect stderr text.

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

> Setting `CI=true` in CI treats all destructive commands in that job as confirmed. Use it only for read-only steps (such as `info` or `export metadata` to stdout) or for write steps that have already passed `--dry-run` validation; for actual write steps, prefer explicit `TAG_CLI_YES=1` or `-y` to clearly express intent.


<a id="troubleshooting-and-faq"></a>
## Troubleshooting / FAQ

### Build-time error: TagLib not found or `vendor/taglib` is empty

Please initialize submodules first:

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

tag-cli **does not** create backups automatically. Please back up original files before performing batch writes.

### Difference between corrupt files and unsupported formats

`export metadata` distinguishes them heuristically by file extension: known audio extensions that cannot be read are marked as `corrupt_file`; unknown extensions are treated as `skipped`.

### Input file does not exist or cannot be read

For commands such as `info`, `get`, `set`, `clear`, and `cover`, when the file does not exist, the path is wrong, or the format is not recognized by TagLib, they usually output `TagLib error: file is not a valid/recognized audio file for TagLib` and return exit code `1`. `export metadata` records such files in the `failures` array and only returns a non-zero exit code when `Failures > 0`.

<a id="license"></a>
## License

[MIT](LICENSE)

Development, build, and contribution guidelines can be found in [CONTRIBUTING.md](CONTRIBUTING.md).
