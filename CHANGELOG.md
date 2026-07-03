# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
