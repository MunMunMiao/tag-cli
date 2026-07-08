# tag-cli Help Kubectl-Style Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Improve tag-cli help text and command discoverability using kubectl-style grouping, examples, and concise usage guidance without changing command behavior.

**Architecture:** Keep all command behavior unchanged and update only clap metadata in `crates/tag-cli/src/cli.rs`, help-output tests in `crates/tag-cli/tests/cli_test.rs`, and user-facing docs in `README.md`. Use clap-native help headings where possible and keep examples in command `long_about` strings.

**Tech Stack:** Rust, clap derive, assert_cmd integration tests, README markdown.

## Global Constraints

- Do not change any command name, flag name, alias, parsing behavior, confirmation behavior, or file-writing behavior.
- Do not reintroduce removed commands: `man`, `init-manifest`, or `completions`.
- Keep all examples copy-pasteable and aligned with existing commands: `apply`, `clear`, `cover`, `export metadata`, `get`, `info`, `list-keys`, `set`, `update`.
- Maintain `cargo test --workspace`, `cargo clippy --workspace -- -D warnings`, and project-source line coverage at 100%.

---

### Task 1: Group top-level commands and improve command help text

**Files:**
- Modify: `crates/tag-cli/src/cli.rs`
- Test: `crates/tag-cli/tests/cli_test.rs`

**Interfaces:**
- Consumes: existing `Cli`, `Commands`, `CoverCommands`, `ExportCommands`, and argument structs.
- Produces: same parser behavior with clearer help headings, examples, and flag descriptions.

- [ ] **Step 1: Add top-level subcommand headings**

Use clap command metadata to group top-level commands by workflow:

```rust
#[derive(Subcommand, Debug)]
pub enum Commands {
    #[command(next_help_heading = "Inspect commands")]
    ListKeys(ListKeysArgs),
    // info/get under Inspect commands

    #[command(next_help_heading = "Edit commands")]
    Set(SetArgs),
    // clear/cover under Edit commands

    #[command(next_help_heading = "Batch commands")]
    Apply(ApplyArgs),
    // export under Batch commands

    #[command(next_help_heading = "Utility commands")]
    Update,
}
```

If `next_help_heading` does not compile on variants, use the clap-supported equivalent and verify with `cargo check -p tag-cli`.

- [ ] **Step 2: Simplify top-level `long_about`**

Replace the current manually duplicated command group list with concise overview, safety note, and kubectl-style example block:

```text
Edit audio metadata and embedded cover art.

tag-cli wraps TagLib to read and write tags and cover images for MP3, FLAC, M4A, Ogg, Opus, WAV, and many other formats.

Common workflows:
  # Show everything about a file
  tag-cli info -i song.mp3

  # Read selected tags
  tag-cli get -i song.mp3 TITLE ARTIST

  # Preview a tag edit before writing
  tag-cli set -i song.mp3 --dry-run TITLE="My Title"

  # Write tags in place after confirmation is explicit
  tag-cli set -i song.mp3 -y TITLE="My Title" ARTIST="My Artist"

  # Export metadata as an apply-ready YAML manifest
  tag-cli export metadata -i "**/*.mp3" -o manifest.yaml

Safety:
  Commands that modify files in place require -y/--yes.
  Use --dry-run first when a command supports it.
```

Set `after_help` to:

```text
Use "tag-cli <COMMAND> --help" for more information about a command.
```

- [ ] **Step 3: Convert examples to `# comment + command` style**

For each command and subcommand `long_about`, update `Examples:` to kubectl style:

```text
Examples:
  # Show all metadata in table form
  tag-cli info -i song.mp3

  # Show metadata as JSON
  tag-cli info -i song.mp3 -f json
```

Cover at least:
- `info`
- `get`
- `list-keys`
- `set`
- `clear`
- `cover`, `cover get`, `cover set`, `cover clear`
- `apply`
- `export metadata`
- `update`

- [ ] **Step 4: Standardize flag help and value names**

Update argument attributes to use clear value names and concise descriptions:

```rust
#[arg(short = 'i', long, value_name = "FILE", help = "Audio file path")]
pub input: PathBuf;

#[arg(short = 'o', long, value_name = "FILE", help = "Output file path (default: edit input in place)")]
pub output: Option<PathBuf>;

#[arg(short, long, value_enum, value_name = "FORMAT", help = "Output format")]
pub format: Option<OutputFormat>;

#[arg(short = 'y', long, help = "Skip confirmation for destructive writes")]
pub yes: bool;
```

Apply consistently to manifest, cover image, tags, fields, directory, quality, KB, pixels, and picture type flags.

- [ ] **Step 5: Add or update tests for help output**

Add integration tests in `crates/tag-cli/tests/cli_test.rs`:

```rust
#[test]
fn top_level_help_groups_commands_by_workflow() {
    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Inspect commands:"))
        .stdout(predicate::str::contains("Edit commands:"))
        .stdout(predicate::str::contains("Batch commands:"))
        .stdout(predicate::str::contains("Utility commands:"))
        .stdout(predicate::str::contains("Use \"tag-cli <COMMAND> --help\""));
}

#[test]
fn command_help_uses_comment_examples() {
    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args(["set", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Examples:"))
        .stdout(predicate::str::contains("# Preview a tag edit before writing"))
        .stdout(predicate::str::contains("tag-cli set -i song.mp3 --dry-run"));
}
```

Run:

```bash
cargo test -p tag-cli top_level_help_groups_commands_by_workflow command_help_uses_comment_examples
```

Expected: PASS.

---

### Task 2: Update README help documentation

**Files:**
- Modify: `README.md`

**Interfaces:**
- Consumes: current command reference and removed command state.
- Produces: README that matches the new help style and no longer mentions removed commands.

- [ ] **Step 1: Update command overview**

Ensure README groups commands in the same categories:

```markdown
| Group | Commands |
| --- | --- |
| Inspect | `info`, `get`, `list-keys` |
| Edit | `set`, `clear`, `cover get`, `cover set`, `cover clear` |
| Batch | `apply`, `export metadata` |
| Utility | `update` |
```

- [ ] **Step 2: Update examples to match help style**

Where README shows command examples, prefer short kubectl-style comments before commands:

```bash
# Preview a tag edit before writing
 tag-cli set -i song.mp3 --dry-run TITLE="My Title"
```

- [ ] **Step 3: Confirm removed commands stay removed from docs**

Search README for removed commands:

```bash
grep -nE 'init-manifest|completions|man page|tag-cli man|tag-cli completions' README.md
```

Expected: no active command documentation remains.

---

### Task 3: Verify behavior, help output, and coverage

**Files:**
- No direct code changes unless verification finds an issue.

**Interfaces:**
- Consumes: updated code and tests.
- Produces: verification evidence.

- [ ] **Step 1: Inspect help output manually**

Run:

```bash
cargo run -p tag-cli -- --help
cargo run -p tag-cli -- set --help
cargo run -p tag-cli -- export metadata --help
cargo run -p tag-cli -- cover set --help
```

Expected:
- top-level commands are grouped by workflow;
- examples use `# comment + command` style;
- no `man`, `init-manifest`, or `completions` command appears.

- [ ] **Step 2: Run tests**

```bash
cargo test --workspace
```

Expected: PASS.

- [ ] **Step 3: Run clippy**

```bash
cargo clippy --workspace -- -D warnings
```

Expected: PASS.

- [ ] **Step 4: Run coverage**

```bash
export LLVM_COV="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/lib/rustlib/aarch64-apple-darwin/bin/llvm-cov"
export LLVM_PROFDATA="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/lib/rustlib/aarch64-apple-darwin/bin/llvm-profdata"
cargo llvm-cov --workspace --all-features --tests --json --output-path /tmp/coverage_help_final.json
```

Then parse project source line coverage excluding external paths:

```bash
python3 - <<'PY'
import json
p='/tmp/coverage_help_final.json'
data=json.load(open(p))
total=covered=0
unc=[]
for f in data['data'][0]['files']:
    fn=f['filename']
    if fn.startswith('/opt/homebrew') or fn.startswith('/rustc') or '/target/' in fn:
        continue
    lines={}
    for seg in f['segments']:
        line,_,count,has_code,_,_=seg
        if has_code:
            lines[line]=max(lines.get(line, 0), count)
    for line,count in lines.items():
        total+=1
        if count>0:
            covered+=1
        else:
            unc.append((fn,line))
print(f'project source lines: {covered}/{total} ({covered/total*100:.4f}%)')
for fn,line in unc:
    print(fn, line)
PY
```

Expected: project source lines are 100% covered.
