# Contributing to tpt-doc

Thanks for helping. The workspace is pure Rust (edition 2024, MSRV 1.85), dual licensed `MIT OR Apache-2.0`; contributions are accepted under the same terms.

## Before you open a PR

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo deny check
```

## Guidelines

- **Test first for bugs.** Add a regression test that fails, then fix it.
- **No C-FFI, no subprocesses, no GPL-adjacent dependencies** (enforced by `cargo-deny`).
- **Untrusted input must never panic.** Parsers return `DocError`, honour `tpt_doc_core::Limits`, and bound allocations.
- Public items need `///` docs (`missing_docs` is enforced in CI).
- Update the crate's `CHANGELOG.md` (Keep a Changelog) and, for behaviour changes, its mdBook chapter in `docs/src/crates/`.
- Output must stay deterministic: identical input gives byte-identical output.

## Reporting security issues

Do not open a public issue; see [SECURITY.md](SECURITY.md).
