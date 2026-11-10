# Changelog

All notable changes to `tpt-doc-cli` are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added

- `tpt sniff`, `inspect`, `validate` (with `--json`), `render` (with `--data`
  and `-o`), `text`, `convert`, `redact` and `diff` (with `--ignore` and
  `--json`) commands.
- Exit codes: `0` for success, `1` for an invalid document or a difference, and
  `2` for a command that could not run.
- The command logic is in a library target (`tpt_doc_cli::commands`), so it can
  be tested without spawning a process.
