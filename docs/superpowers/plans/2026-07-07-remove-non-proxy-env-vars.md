# Remove Non-Proxy Environment Variables Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove support for all non-proxy environment variables from `tag-cli`, leaving `-y` / `--yes` as the only non-interactive write confirmation mechanism and keeping only standard proxy variables for `tag-cli update`.

**Architecture:** Confirmation becomes a pure CLI-argument decision: `Cli::is_confirmed(explicit_yes)` returns only `explicit_yes`. The update command stops reading test/debug URL override environment variables and uses the built-in GitHub release URLs. Proxy support remains in `commands/update.rs` through `HTTP_PROXY`, `HTTPS_PROXY`, `ALL_PROXY`, `NO_PROXY`, and lowercase variants.

**Tech Stack:** Rust 2024 workspace, clap, assert_cmd, cargo llvm-cov, GitHub Actions.

## Global Constraints

- Do not remove or change standard proxy support: `HTTP_PROXY` / `http_proxy`, `HTTPS_PROXY` / `https_proxy`, `ALL_PROXY` / `all_proxy`, `NO_PROXY` / `no_proxy` remain supported for `tag-cli update`.
- Remove runtime support for `TAG_CLI_YES` and `CI` confirmation.
- Remove runtime/test/debug support for `TAG_CLI_UPDATE_API_URL` and `TAG_CLI_UPDATE_DOWNLOAD_BASE`.
- Keep `-y` / `--yes` as the only non-interactive confirmation for destructive writes and output overwrites.
- Do not add new dependencies.
- Keep coverage script behavior unchanged unless tests require updating.
- Verify with `rustup run stable cargo fmt --check`, `rustup run stable cargo clippy --workspace -- -D warnings`, `rustup run stable cargo test --workspace`, and `./scripts/coverage.sh`.

---

## File Structure

- `crates/tag-cli/src/cli.rs`
  - Owns CLI definitions, help text, and confirmation policy.
  - Will remove `std::env::var("TAG_CLI_YES")` and `std::env::var("CI")` checks.
  - Will update `--yes` help text and command long help.
  - Will delete env-confirmation unit tests and env helper.

- `crates/tag-cli/src/commands/update.rs`
  - Owns update URL selection and proxy handling.
  - Will remove `TAG_CLI_UPDATE_API_URL` and `TAG_CLI_UPDATE_DOWNLOAD_BASE` reads.
  - Will keep proxy selection and its unit tests.

- `crates/tag-cli/tests/cli_test.rs`
  - Owns CLI integration tests.
  - Will remove `TAG_CLI_YES` / `CI` confirmation tests and update any write tests to pass `-y` explicitly.

- `crates/tag-cli/tests/error_propagation.rs`
  - Owns error propagation integration tests.
  - Will remove CI env manipulation helper and keep overwrite failure tests by not passing `-y`.

- `crates/tag-cli/tests/update_test.rs`
  - Owns update integration tests.
  - Will remove tests that depend on URL override env vars, or convert remaining proxy behavior checks to lower-level unit tests in `update.rs` if needed.

- `README.md`
  - Will remove all documentation claiming env-var confirmation or update URL overrides.
  - Will keep proxy environment variable documentation.
  - CI examples will use `-y` instead of `TAG_CLI_YES` or implicit `CI`.

---

### Task 1: Simplify confirmation policy in `cli.rs`

**Files:**
- Modify: `crates/tag-cli/src/cli.rs`

**Interfaces:**
- Consumes: existing `Cli::is_confirmed(explicit_yes: bool) -> bool` callers.
- Produces: same function signature, new behavior: returns `explicit_yes` only.

- [ ] **Step 1: Write the failing tests**

In `crates/tag-cli/src/cli.rs`, replace the env-confirmation tests with tests proving env vars are ignored:

```rust
#[test]
fn is_confirmed_true_with_explicit_yes() {
    assert!(Cli::is_confirmed(true));
}

#[test]
fn is_confirmed_false_without_explicit_yes_even_when_env_is_set() {
    unsafe {
        std::env::set_var("TAG_CLI_YES", "1");
        std::env::set_var("CI", "true");
    }

    assert!(!Cli::is_confirmed(false));

    unsafe {
        std::env::remove_var("TAG_CLI_YES");
        std::env::remove_var("CI");
    }
}
```

Remove these old tests and their helper because they encode deleted behavior:

```rust
is_confirmed_true_with_tag_cli_yes_true
is_confirmed_true_with_tag_cli_yes_one
is_confirmed_true_with_tag_cli_yes_uppercase
is_confirmed_false_with_neutral_env
is_confirmed_true_with_ci_true
with_env_restores_existing_variables
with_env
ENV_LOCK
WITH_ENV_DEPTH
```

- [ ] **Step 2: Run the focused test and verify it fails**

Run:

```bash
rustup run stable cargo test -p tag-cli --lib cli::tests::is_confirmed_false_without_explicit_yes_even_when_env_is_set
```

Expected before implementation: FAIL because current `Cli::is_confirmed(false)` returns true when `TAG_CLI_YES=1` or `CI=true`.

- [ ] **Step 3: Implement minimal confirmation change**

In `crates/tag-cli/src/cli.rs`, replace `is_confirmed` with:

```rust
impl Cli {
    /// Return whether destructive writes were explicitly confirmed.
    pub fn is_confirmed(explicit_yes: bool) -> bool {
        explicit_yes
    }
}
```

Update the top-level long help text by replacing:

```text
Commands that modify files in place require -y/--yes, TAG_CLI_YES=1/true, or CI=true.
```

with:

```text
Commands that modify files in place require -y/--yes.
```

Update every `--yes` help string by replacing:

```rust
help = "Skip confirmation for destructive writes; also respects TAG_CLI_YES=1/true or CI=true"
```

with:

```rust
help = "Skip confirmation for destructive writes"
```

Update the export metadata overwrite help string by replacing:

```rust
help = "Skip confirmation for output overwrites; also respects TAG_CLI_YES=1/true or CI=true"
```

with:

```rust
help = "Skip confirmation for output overwrites"
```

- [ ] **Step 4: Run focused tests**

Run:

```bash
rustup run stable cargo test -p tag-cli --lib cli::tests::is_confirmed
```

Expected: all matching `is_confirmed` unit tests pass.

- [ ] **Step 5: Commit task**

```bash
git add crates/tag-cli/src/cli.rs
git commit -m "refactor(cli): require explicit yes for writes"
```

---

### Task 2: Update confirmation integration tests

**Files:**
- Modify: `crates/tag-cli/tests/cli_test.rs`
- Modify: `crates/tag-cli/tests/error_propagation.rs`

**Interfaces:**
- Consumes: new `Cli::is_confirmed(false) == false` behavior.
- Produces: integration tests that use `-y` for writes and no longer rely on env-var confirmation.

- [ ] **Step 1: Update failing integration expectations**

In `crates/tag-cli/tests/cli_test.rs`, remove tests that prove deleted behavior:

```rust
test_env_tag_cli_yes_bypasses_confirmation
test_ci_env_bypasses_confirmation
```

Keep or add a test proving `CI=true` does not bypass confirmation:

```rust
#[test]
fn test_ci_env_does_not_bypass_confirmation() {
    let temp = tempfile::tempdir().unwrap();
    let input = copy_fixture_to_temp(temp.path(), "sample_flac.flac");

    Command::cargo_bin("tag-cli")
        .unwrap()
        .args(["set", "-i", input.to_str().unwrap(), "TITLE=New Title"])
        .env("CI", "true")
        .assert()
        .failure()
        .stderr(predicate::str::contains("requires confirmation"));
}
```

If the exact helper names differ, use the existing fixture-copy helper in `cli_test.rs` and preserve the same assertion style already used by nearby confirmation tests.

- [ ] **Step 2: Remove CI env workaround in error propagation tests**

In `crates/tag-cli/tests/error_propagation.rs`, delete this helper entirely:

```rust
fn with_ci_false<F: FnOnce()>(f: F) {
    let old_ci = std::env::var("CI").ok();
    unsafe {
        std::env::set_var("CI", "false");
    }
    f();
    unsafe {
        match old_ci {
            Some(v) => std::env::set_var("CI", v),
            None => std::env::remove_var("CI"),
        }
    }
}
```

Replace the wrapped call:

```rust
with_ci_false(|| {
    let result = tag_cli::run_with(cli, &mut stdout);
    assert!(
        result.is_err(),
        "expected export metadata to fail when output file exists without -y"
    );
});
```

with:

```rust
let result = tag_cli::run_with(cli, &mut stdout);
assert!(
    result.is_err(),
    "expected export metadata to fail when output file exists without -y"
);
```

Remove `.env("CI", "false")` calls from tests that only needed to counteract CI auto-confirmation. Do not remove proxy env cleanup from update tests.

- [ ] **Step 3: Run focused integration tests**

Run:

```bash
rustup run stable cargo test -p tag-cli --test cli_test confirmation
rustup run stable cargo test -p tag-cli --test error_propagation
```

Expected: confirmation-related integration tests pass, and error propagation has 6 passing tests.

- [ ] **Step 4: Commit task**

```bash
git add crates/tag-cli/tests/cli_test.rs crates/tag-cli/tests/error_propagation.rs
git commit -m "test(cli): remove environment confirmation coverage"
```

---

### Task 3: Remove update URL override environment variables

**Files:**
- Modify: `crates/tag-cli/src/commands/update.rs`
- Modify: `crates/tag-cli/tests/update_test.rs`

**Interfaces:**
- Consumes: constants `DEFAULT_API_URL` and `DEFAULT_DOWNLOAD_BASE`.
- Produces: `api_url() -> String` and `download_base() -> String` that always return defaults; or inline constants if no tests need the helpers.

- [ ] **Step 1: Write/update tests for removed override behavior**

In `crates/tag-cli/src/commands/update.rs`, replace tests that assert overrides work:

```rust
api_url_uses_test_override
download_base_uses_test_override
api_url_defaults_when_override_missing
download_base_defaults_when_override_missing
```

with tests that prove overrides are ignored:

```rust
#[test]
fn api_url_ignores_env_override() {
    let result = with_env_vars(&[("TAG_CLI_UPDATE_API_URL", Some("http://override/api"))], || {
        api_url()
    });
    assert_eq!(result, DEFAULT_API_URL);
}

#[test]
fn download_base_ignores_env_override() {
    let result = with_env_vars(
        &[("TAG_CLI_UPDATE_DOWNLOAD_BASE", Some("http://override/dl"))],
        || download_base(),
    );
    assert_eq!(result, DEFAULT_DOWNLOAD_BASE);
}
```

- [ ] **Step 2: Run focused tests and verify they fail**

Run:

```bash
rustup run stable cargo test -p tag-cli --lib commands::update::tests::api_url_ignores_env_override commands::update::tests::download_base_ignores_env_override
```

Expected before implementation: FAIL because current debug/test builds return the override env values.

- [ ] **Step 3: Implement minimal update URL change**

In `crates/tag-cli/src/commands/update.rs`, replace:

```rust
fn api_url() -> String {
    #[cfg(any(debug_assertions, test, feature = "test-overrides"))]
    if let Ok(url) = env::var("TAG_CLI_UPDATE_API_URL") {
        return url;
    }
    DEFAULT_API_URL.into()
}

fn download_base() -> String {
    #[cfg(any(debug_assertions, test, feature = "test-overrides"))]
    if let Ok(base) = env::var("TAG_CLI_UPDATE_DOWNLOAD_BASE") {
        return base;
    }
    DEFAULT_DOWNLOAD_BASE.into()
}
```

with:

```rust
fn api_url() -> String {
    DEFAULT_API_URL.into()
}

fn download_base() -> String {
    DEFAULT_DOWNLOAD_BASE.into()
}
```

Keep `use std::env;` because proxy code still uses `env::var`.

- [ ] **Step 4: Remove update integration tests that require URL env overrides**

In `crates/tag-cli/tests/update_test.rs`, delete tests that can no longer target the mock server through env overrides:

```rust
update_detects_new_version_and_downloads
update_rejects_checksum_mismatch
update_already_up_to_date
update_fails_when_proxy_is_unreachable
update_bypasses_proxy_for_no_proxy_hosts
```

Then delete unused helpers/imports if the whole file becomes empty:

```rust
spawn_mock_server
copy_test_binary
clear_proxy_env
UPDATE_TEST_LOCK
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::thread;
use assert_cmd::Command;
use tempfile::TempDir;
use tiny_http::{Header, ListenAddr, Response, Server};
```

If no tests remain in `update_test.rs`, delete the file:

```bash
git rm crates/tag-cli/tests/update_test.rs
```

Proxy behavior remains covered by unit tests in `crates/tag-cli/src/commands/update.rs` for `select_proxy_for_url`, `is_no_proxy`, and invalid proxy URL handling.

- [ ] **Step 5: Run update tests**

Run:

```bash
rustup run stable cargo test -p tag-cli --lib commands::update
```

Expected: update unit tests pass and still cover proxy selection.

- [ ] **Step 6: Commit task**

```bash
git add crates/tag-cli/src/commands/update.rs crates/tag-cli/tests/update_test.rs
git commit -m "refactor(update): remove URL override environment variables"
```

If `update_test.rs` was deleted, use:

```bash
git add crates/tag-cli/src/commands/update.rs
git rm crates/tag-cli/tests/update_test.rs
git commit -m "refactor(update): remove URL override environment variables"
```

---

### Task 4: Update README documentation

**Files:**
- Modify: `README.md`

**Interfaces:**
- Consumes: new runtime behavior from Tasks 1-3.
- Produces: README that documents only `-y` confirmation and proxy environment variables.

- [ ] **Step 1: Remove env-confirmation copy**

In `README.md`, replace feature bullets and safety copy:

```markdown
- **CI friendly.** Machine-readable JSON/YAML output and environment-variable confirmation.
```

with:

```markdown
- **CI friendly.** Machine-readable JSON/YAML output and explicit `-y` confirmation for writes.
```

Replace:

```markdown
- Environment-variable confirmation for scripting and CI (`TAG_CLI_YES=1` or `CI=true`).
```

with:

```markdown
- Explicit `-y` / `--yes` confirmation for scripting and CI writes.
```

- [ ] **Step 2: Update global confirmation section**

Replace the confirmation priority table with a single-source explanation:

```markdown
### Write confirmation

Destructive commands (`set`, `clear`, `cover set`, `cover clear`, `apply`, `export metadata` when overwriting existing files) require explicit command-line confirmation via `-y` / `--yes`.

If `-y` / `--yes` is not provided, the write command exits with an error and modifies no files.
```

Keep the existing command-by-command table, but ensure it mentions only `-y` / `--yes`.

- [ ] **Step 3: Keep proxy docs and remove update override note**

Keep this section unchanged except deleting the note about removed overrides:

```markdown
The update command honors standard proxy environment variables:

- `HTTP_PROXY` / `http_proxy`
- `HTTPS_PROXY` / `https_proxy`
- `ALL_PROXY` / `all_proxy`
- `NO_PROXY` / `no_proxy`
```

Delete:

```markdown
> [!NOTE]
> `TAG_CLI_UPDATE_API_URL` and `TAG_CLI_UPDATE_DOWNLOAD_BASE` can override the GitHub API and download base URLs, but only in debug/test builds or when the `test-overrides` feature is enabled. They are compiled out of release binaries.
```

- [ ] **Step 4: Update CI examples**

Replace examples that use environment confirmation:

```yaml
env:
  TAG_CLI_YES: "1"
```

with explicit flags in commands:

```yaml
run: tag-cli apply -m manifest.yaml -y
```

Replace shell examples:

```bash
TAG_CLI_YES=1 tag-cli set -i song.mp3 TITLE="A"
CI=true tag-cli apply -m manifest.yaml
```

with:

```bash
tag-cli set -i song.mp3 TITLE="A" -y
tag-cli apply -m manifest.yaml -y
```

Delete any table rows for `TAG_CLI_YES` and `CI`. Keep proxy environment variable rows only.

- [ ] **Step 5: Check no deleted env vars remain in docs/source**

Run:

```bash
rg -n "TAG_CLI_YES|TAG_CLI_UPDATE_API_URL|TAG_CLI_UPDATE_DOWNLOAD_BASE|CI=true|\bCI\b non-empty|environment-variable confirmation" README.md crates/tag-cli/src crates/tag-cli/tests
```

Expected: no matches for deleted support. Matches for general prose like “CI pipeline” are okay only if they do not describe `CI` env confirmation.

- [ ] **Step 6: Commit task**

```bash
git add README.md
git commit -m "docs: document explicit write confirmation only"
```

---

### Task 5: Full verification and release-note update

**Files:**
- Modify: `CHANGELOG.md`

**Interfaces:**
- Consumes: all previous task changes.
- Produces: verified workspace and release note for the breaking behavior cleanup.

- [ ] **Step 1: Update changelog**

In `CHANGELOG.md`, under `## [0.1.2] - 2026-07-07`, add:

```markdown
### Removed

- Removed non-proxy environment variable support: `TAG_CLI_YES`, `CI` write confirmation, `TAG_CLI_UPDATE_API_URL`, and `TAG_CLI_UPDATE_DOWNLOAD_BASE`. Use `-y` / `--yes` for non-interactive writes; standard proxy environment variables remain supported for `tag-cli update`.
```

If there is already a `### Removed` section under `0.1.2`, append the bullet there instead of creating a duplicate heading.

- [ ] **Step 2: Run formatting**

Run:

```bash
rustup run stable cargo fmt --check
```

Expected: exit 0.

- [ ] **Step 3: Run clippy**

Run:

```bash
rustup run stable cargo clippy --workspace -- -D warnings
```

Expected: exit 0.

- [ ] **Step 4: Run full tests**

Run:

```bash
rustup run stable cargo test --workspace
```

Expected: all tests pass.

- [ ] **Step 5: Run coverage gate**

Run:

```bash
./scripts/coverage.sh
```

Expected: exit 0 and line coverage remains `100.00%` for the gated report.

- [ ] **Step 6: Final search for removed env vars**

Run:

```bash
rg -n "TAG_CLI_YES|TAG_CLI_UPDATE_API_URL|TAG_CLI_UPDATE_DOWNLOAD_BASE|std::env::var\(\"CI\"\)|env::var\(\"CI\"\)" .
```

Expected: no matches, except historical entries in generated artifacts if any are not tracked. Do not commit generated coverage output.

- [ ] **Step 7: Commit final verification docs**

```bash
git add CHANGELOG.md
git commit -m "chore: note removed environment variable support"
```

---

## Self-Review

- Spec coverage: Tasks 1-3 remove all requested non-proxy env vars from code and tests. Task 4 updates README. Task 5 updates changelog and verifies. Proxy env support is explicitly preserved in Task 3.
- Placeholder scan: No TBD/TODO placeholders remain; every task has concrete files, commands, and expected results.
- Type consistency: `Cli::is_confirmed(explicit_yes: bool) -> bool`, `api_url() -> String`, and `download_base() -> String` signatures are consistent across tasks.
