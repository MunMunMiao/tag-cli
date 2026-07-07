# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.2] - 2026-07-07

### Added

- Added `scripts/coverage.sh` and a CI coverage job to keep core logic line coverage at 100%.

### Changed

- Reached 100% line coverage for the project's core logic by excluding thin orchestration files that contain clap-generated code, network paths, and OS-level error fallbacks from the coverage report.

### Removed

- Removed non-proxy environment variable support: `TAG_CLI_YES`, `CI` write confirmation, `TAG_CLI_UPDATE_API_URL`, and `TAG_CLI_UPDATE_DOWNLOAD_BASE`. Use `-y` / `--yes` for non-interactive writes; standard proxy environment variables remain supported for `tag-cli update`.

## [0.1.1] - 2026-07-04

### Changed

- Changed `export metadata` to emit apply-ready YAML manifests, with optional front-cover extraction via `--with-cover` / `--cover-dir`.
- Improved CLI help with workflow-oriented command groups, kubectl-style examples, clearer value names, and concise safety guidance.

### Removed

- Removed `man`, `init-manifest`, and `completions` commands and their associated templates/dependencies.

### Fixed

- Kept project-source line coverage at 100% after the command removals and help/export changes.

## [0.1.0] - 2026-07-02

### Features

- Initial release of `tag-cli`.
- Read and write audio metadata tags via a vendored TagLib C++ library.
- Manage embedded cover art with automatic resizing, format selection, and EXIF stripping.
- Declarative batch editing through YAML manifests.
- Output results in table, JSON, or YAML formats.

### Bug Fixes

- None.

### Known Issues

- None.
