use clap::CommandFactory;
use clap_complete::Shell;
use std::io;
use std::io::Write;

use crate::cli::Cli;

/// Generate shell completion script for `tag-cli` and write it to `writer`.
pub fn generate_completions<W: Write>(shell: Shell, writer: &mut W) -> io::Result<()> {
    let mut cmd = Cli::command();
    let mut buf = Vec::new();
    clap_complete::generate(shell, &mut cmd, "tag-cli", &mut buf);
    writer.write_all(&buf)
}

/// Generate a man page for `tag-cli` and write it to `writer`.
pub fn generate_man<W: Write>(writer: &mut W) -> io::Result<()> {
    let cmd = Cli::command();
    let man = clap_mangen::Man::new(cmd);
    man.render(writer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completions_bash_contains_command_name() {
        let mut buf = Vec::new();
        generate_completions(Shell::Bash, &mut buf).unwrap();
        let script = String::from_utf8(buf).unwrap();
        assert!(script.contains("tag-cli"));
    }

    #[test]
    fn completions_zsh_is_non_empty() {
        let mut buf = Vec::new();
        generate_completions(Shell::Zsh, &mut buf).unwrap();
        assert!(!buf.is_empty());
    }

    #[test]
    fn completions_fish_is_non_empty() {
        let mut buf = Vec::new();
        generate_completions(Shell::Fish, &mut buf).unwrap();
        assert!(!buf.is_empty());
    }

    #[test]
    fn man_page_contains_command_name() {
        let mut buf = Vec::new();
        generate_man(&mut buf).unwrap();
        let page = String::from_utf8(buf).unwrap();
        assert!(page.contains("tag-cli"));
    }
}
