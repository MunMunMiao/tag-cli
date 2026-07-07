#!/bin/bash
set -euo pipefail

# Measure line coverage for the project's core logic.
#
# Excluded from the report:
# - tag-cli/src/cli.rs            : clap derive-generated parsing surface
# - tag-cli/src/lib.rs            : top-level CLI dispatch and `?` propagation glue
# - tag-cli/src/commands/export_metadata.rs : filesystem/serialization fallbacks
# - tag-cli/src/commands/update.rs : network release-check paths
# - rustlib                        : standard library sources
#
# These files contain macro-generated code or OS/network error paths that
# cannot be exercised deterministically in tests. The remaining code is
# required to maintain 100% line coverage.

rustup run stable cargo llvm-cov --workspace \
  --ignore-filename-regex 'crates/tag-cli/src/lib\.rs|src/cli\.rs|src/commands/export_metadata\.rs|src/commands/update\.rs|rustlib' \
  --fail-under-lines 100 \
  "$@"
