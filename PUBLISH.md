# Publishing tracker

This file tracks what is ready to publish to crates.io, in what order, and the
result of each dry run. **Nothing here has been published.** Publishing is
irreversible: a published version can be yanked, but its name and number stay
taken. Publish only when the release is approved.

Last checked: 2026-11-10, workspace version 0.1.0, MSRV 1.88.

## Publish order

Each crate must be on crates.io before any crate that depends on it can be
packaged. Publish top to bottom, and wait for each one to appear in the index
before the next.

| # | Crate | Internal dependencies | Dry run (`cargo package --no-verify`) |
|---|---|---|---|
| 1 | `tpt-doc-core` | none | **Pass** (13 files, 58.2 KiB; `cargo publish --dry-run` also passes, including the verify build) |
| 2 | `tpt-doc-fhir` | core | Blocked: core not on crates.io |
| 3 | `tpt-doc-hl7v2` | core | Blocked: core not on crates.io |
| 4 | `tpt-doc-edi` | core | Blocked: core not on crates.io |
| 5 | `tpt-doc-ooxml` | core | Blocked: core not on crates.io |
| 6 | `tpt-doc-pdf` | core | Blocked: core not on crates.io |
| 7 | `tpt-doc-convert` | core, fhir, hl7v2 | Blocked: fhir, hl7v2 not on crates.io |
| 8 | `tpt-doc-spreadsheet` | core, ooxml | Blocked: ooxml not on crates.io |
| 9 | `tpt-doc-word` | core, ooxml | Blocked: ooxml not on crates.io |
| 10 | `tpt-doc-layout` | core, pdf | Blocked: pdf not on crates.io |
| 11 | `tpt-doc-sign` | core, pdf | Blocked: pdf not on crates.io |
| 12 | `tpt-doc-ubl` | core, pdf | Blocked: pdf not on crates.io |
| 13 | `tpt-doc-diff` | core, edi, hl7v2, spreadsheet, word | Blocked: edi, hl7v2, spreadsheet, word not on crates.io |
| 14 | `tpt-doc-audit` | core, pdf, sign | Blocked: pdf, sign not on crates.io |
| 15 | `tpt-doc` | convert, core, diff, edi, fhir, hl7v2, layout, pdf, sign, spreadsheet, ubl, word | Blocked: all format crates not on crates.io |
| 16 | `tpt-doc-cli` | tpt-doc, convert, diff | Blocked: tpt-doc, convert, diff not on crates.io |
| 17 | `tpt-doc-mcp` | cli, ubl | Blocked: cli, ubl not on crates.io |

The order above is a valid topological sort of normal dependencies, derived from
`cargo metadata`. Dependencies in `[dev-dependencies]` are not included in that
check; see "Open items" below.

## Not published

| Crate | Reason |
|---|---|
| `tpt-doc-bench` | `publish = false` |
| `tpt-doc-templates` | `publish = false` |
| `tpt-doc-wasm` | `publish = false` |

## Per-crate status

Every publishable crate has a README, a CHANGELOG, `keywords` (at most five,
each at most 20 characters) and `categories` (checked against the live crates.io
category list). Documentation links point at docs.rs.

| Crate | README | CHANGELOG | Keywords | Categories | Manifest changes uncommitted |
|---|---|---|---|---|---|
| `tpt-doc-core` | rewritten | existing | yes | yes | no |
| `tpt-doc-fhir` | rewritten | existing | yes | yes | no |
| `tpt-doc-hl7v2` | existing (not reviewed in this pass) | existing | yes | yes | no |
| `tpt-doc-edi` | rewritten | existing | yes | yes | no |
| `tpt-doc-ooxml` | new | new | yes | yes | yes |
| `tpt-doc-pdf` | rewritten | existing | yes | yes | no |
| `tpt-doc-spreadsheet` | rewritten | existing | yes | yes | no |
| `tpt-doc-word` | existing (not reviewed in this pass) | existing | yes | yes | no |
| `tpt-doc-layout` | existing (not reviewed in this pass) | existing | yes | yes | no |
| `tpt-doc-sign` | existing (not reviewed in this pass) | existing | yes | yes | no |
| `tpt-doc-ubl` | existing (not reviewed in this pass) | existing | yes | yes | no |
| `tpt-doc-convert` | rewritten | new | yes | yes | no |
| `tpt-doc-diff` | rewritten | new | yes | yes | no |
| `tpt-doc-audit` | rewritten | new | yes | yes | yes |
| `tpt-doc` | rewritten | new | yes | yes | no |
| `tpt-doc-cli` | rewritten | new | yes | yes | no |
| `tpt-doc-mcp` | rewritten | new | yes | yes | no |

The unpublished crates (`tpt-doc-bench`, `tpt-doc-templates`, `tpt-doc-wasm`) also
have READMEs and CHANGELOGs, so the repository is complete in one place.

## Dry-run commands

These commands do not upload anything. Run them in the order above.

```sh
# Package check for one crate (no upload, no verify build)
cargo package --no-verify --allow-dirty -p tpt-doc-core

# Full dry run, including the verify build (no upload)
cargo publish --dry-run --allow-dirty -p tpt-doc-core
```

The `--allow-dirty` flag is only for dry runs on a tree with uncommitted changes.
Drop it for the real release, which should come from a clean commit.

## Publish commands (do not run until approved)

```sh
cargo publish -p tpt-doc-core
# wait until https://crates.io/crates/tpt-doc-core shows 0.1.0, then the next crate
```

Crates published without the crates.io index updated first will fail to resolve
their dependencies. Allow the index to update between each step.

## Results log

| Date | Command | Crate | Result |
|---|---|---|---|
| 2026-11-10 | `cargo package --no-verify --allow-dirty -p tpt-doc-core` | tpt-doc-core | Pass |
| 2026-11-10 | `cargo publish --dry-run --allow-dirty -p tpt-doc-core` | tpt-doc-core | Pass (aborted before upload, as intended) |
| 2026-11-10 | `cargo package --no-verify --allow-dirty -p <each of the 16 dependents>` | all dependents | Fail: `no matching package named tpt-doc-core found` (expected until core is published). Re-run for one dependent online, same result. |

## Open items before the first real publish

- **Commit the manifest and changelog changes.** The Cargo.toml metadata edits
  for `tpt-doc-ooxml`, `tpt-doc-audit`, `tpt-doc-templates`, `tpt-doc-wasm` and
  `tpt-doc-bench`, and the CHANGELOG.md edits, are not committed. Dry runs were
  run against the working tree with `--allow-dirty`.
- **Check dev-dependencies.** Dev-dependencies that name a workspace crate with
  a version must also exist on crates.io. This has not been checked yet.
- **Review the five READMEs not rewritten in this pass:** `tpt-doc-hl7v2`,
  `tpt-doc-word`, `tpt-doc-layout`, `tpt-doc-sign` and `tpt-doc-ubl`. They were
  written earlier and may not match the current API. Their quick-start code is
  unverified.
- **Dependents cannot be fully dry-run** until their dependencies are published,
  so only `tpt-doc-core` has a complete dry run. The other 16 need a staged
  check: publish core, then check the next tier, and so on.
- **Confirm the owner and the crates.io name availability** for each of the 17
  names before the first publish.
