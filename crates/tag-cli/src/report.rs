/// Write a status message to stderr.
///
/// Status messages ("saved to ...", "Success: ...", etc.) are human-facing
/// progress feedback and should not pollute stdout, where structured data such
/// as `info`/`get` output and `--dry-run` diffs are written.
pub fn status(msg: impl AsRef<str>) {
    eprintln!("{}", msg.as_ref());
}
