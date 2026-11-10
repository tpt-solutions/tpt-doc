# Publishing tracker

This file tracks the crates.io release of tpt-doc 0.1.0: the publish order,
what has been published, and the result of each dry run and publish. Publishing
is irreversible: a published version can be yanked, but its name and number stay
taken.

Last updated: 2026-11-10, workspace version 0.1.0, MSRV 1.88.

## Status

All 17 publishable crates are on crates.io at 0.1.0, published 2026-11-10. Several
needed a retry after crates.io's new-crate rate limit (see the results log).

## Publish order

Each crate must be on crates.io before any crate that depends on it can be
published. Publish top to bottom, and wait for each one to appear in the index
before the next.

| # | Crate | Internal dependencies | Status |
|---|---|---|---|
| 1 | `tpt-doc-core` | none | Published 2026-11-10 |
| 2 | `tpt-doc-fhir` | core | Published 2026-11-10 |
| 3 | `tpt-doc-hl7v2` | core | Published 2026-11-10 |
| 4 | `tpt-doc-edi` | core | Published 2026-11-10 |
| 5 | `tpt-doc-ooxml` | core | Published 2026-11-10 |
| 6 | `tpt-doc-pdf` | core | Published 2026-11-10 (after a rate-limit wait) |
| 7 | `tpt-doc-convert` | core, fhir, hl7v2 | Published 2026-11-10 (after a rate-limit wait) |
| 8 | `tpt-doc-spreadsheet` | core, ooxml | Published 2026-11-10 (after a rate-limit wait) |
| 9 | `tpt-doc-word` | core, ooxml | Published 2026-11-10 (after a rate-limit wait) |
| 10 | `tpt-doc-layout` | core, pdf | Published 2026-11-10 (after a rate-limit wait) |
| 11 | `tpt-doc-sign` | core, pdf | Published 2026-11-10 (after a rate-limit wait) |
| 12 | `tpt-doc-ubl` | core, pdf | Published 2026-11-10 (after a rate-limit wait) |
| 13 | `tpt-doc-diff` | core, edi, hl7v2, spreadsheet, word | Published 2026-11-10 (after a rate-limit wait) |
| 14 | `tpt-doc-audit` | core, pdf, sign | Published 2026-11-10 (after a rate-limit wait) |
| 15 | `tpt-doc` | convert, core, diff, edi, fhir, hl7v2, layout, pdf, sign, spreadsheet, ubl, word | Published 2026-11-10 (after a rate-limit wait) |
| 16 | `tpt-doc-cli` | tpt-doc, convert, diff | Published 2026-11-10 (after a rate-limit wait) |
| 17 | `tpt-doc-mcp` | cli, ubl | Published 2026-11-10 (after a rate-limit wait) |

The order above is a valid topological sort of normal dependencies, derived from
`cargo metadata`. Dev-dependencies are checked separately; see "Open items".

## Not published

| Crate | Reason |
|---|---|
| `tpt-doc-bench` | `publish = false` |
| `tpt-doc-templates` | `publish = false` |
| `tpt-doc-wasm` | `publish = false` |

## Per-crate status

Every publishable crate has a README, a CHANGELOG, `keywords` (at most five,
each at most 20 characters) and `categories` (checked against the live crates.io
category list). Documentation links point at docs.rs. The CHANGELOGs were rolled
into `[0.1.0] - 2026-11-10` in commit `abce1af`.

| Crate | README | CHANGELOG | Keywords | Categories | Manifest changes uncommitted |
|---|---|---|---|---|---|
| `tpt-doc-core` | rewritten | rolled to 0.1.0 | yes | yes | no |
| `tpt-doc-fhir` | rewritten | rolled to 0.1.0 | yes | yes | no |
| `tpt-doc-hl7v2` | existing (not reviewed in this pass) | rolled to 0.1.0 | yes | yes | no |
| `tpt-doc-edi` | rewritten | rolled to 0.1.0 | yes | yes | no |
| `tpt-doc-ooxml` | new | rolled to 0.1.0 | yes | yes | no |
| `tpt-doc-pdf` | rewritten | rolled to 0.1.0 | yes | yes | no |
| `tpt-doc-spreadsheet` | rewritten | rolled to 0.1.0 | yes | yes | no |
| `tpt-doc-word` | existing (not reviewed in this pass) | rolled to 0.1.0 | yes | yes | no |
| `tpt-doc-layout` | existing (not reviewed in this pass) | rolled to 0.1.0 | yes | yes | no |
| `tpt-doc-sign` | existing (not reviewed in this pass) | rolled to 0.1.0 | yes | yes | no |
| `tpt-doc-ubl` | existing (not reviewed in this pass) | rolled to 0.1.0 | yes | yes | no |
| `tpt-doc-convert` | rewritten | rolled to 0.1.0 | yes | yes | no |
| `tpt-doc-diff` | rewritten | rolled to 0.1.0 | yes | yes | no |
| `tpt-doc-audit` | rewritten | rolled to 0.1.0 | yes | yes | no |
| `tpt-doc` | rewritten | rolled to 0.1.0 | yes | yes | no |
| `tpt-doc-cli` | rewritten | rolled to 0.1.0 | yes | yes | no |
| `tpt-doc-mcp` | rewritten | rolled to 0.1.0 | yes | yes | no |

The unpublished crates (`tpt-doc-bench`, `tpt-doc-templates`, `tpt-doc-wasm`) also
have READMEs and CHANGELOGs, so the repository is complete in one place.

## Dry-run commands

These commands do not upload anything. Use them on a clean commit.

```sh
# Package check for one crate (no upload, no verify build)
cargo package --no-verify -p tpt-doc-core

# Full dry run, including the verify build (no upload)
cargo publish --dry-run -p tpt-doc-core
```

A dependent crate cannot be packaged until its dependencies are on crates.io, so
the dry run is only meaningful for `tpt-doc-core` before the first publish.

## Publish commands

```sh
cargo publish -p <crate>
# wait until https://crates.io/crates/<crate> shows 0.1.0, then the next crate
```

crates.io limits how many new crates can be created in a short period. The first
five new crates went through at once, then each further one needed about ten
minutes. Future publishes of new crates should expect the same.

## Results log

| Date | Command | Crate | Result |
|---|---|---|---|
| 2026-11-10 | `cargo package --no-verify --allow-dirty -p tpt-doc-core` | tpt-doc-core | Pass |
| 2026-11-10 | `cargo publish --dry-run --allow-dirty -p tpt-doc-core` | tpt-doc-core | Pass (aborted before upload, as intended) |
| 2026-11-10 | `cargo package --no-verify --allow-dirty -p <each of the 16 dependents>` | all dependents | Fail: `no matching package named tpt-doc-core found` (expected until core is published). |
| 2026-11-10 | `cargo package --no-verify -p tpt-doc-core` (clean tree, commit 152ecc6) | tpt-doc-core | Pass (13 files, 58.2 KiB) |
| 2026-11-10 | `cargo publish --dry-run -p tpt-doc-core` (clean tree, includes verify build) | tpt-doc-core | Pass (verify compiled; upload aborted by dry run, as intended) |
| 2026-11-10 | `cargo package --no-verify -p <each of the 16 dependents>` (clean tree) | all dependents | Fail with exit 101 for every crate: `no matching package named tpt-doc-core found` (for `tpt-doc-mcp` the first missing dependency reported is `tpt-doc-cli`). Expected until the earlier tiers are published. |
| 2026-11-10 | `cargo publish -p tpt-doc-core` | tpt-doc-core | Published 0.1.0 |
| 2026-11-10 | `cargo publish -p` for `fhir`, `hl7v2`, `edi`, `ooxml` | 4 crates | Published 0.1.0 |
| 2026-11-10 | `cargo publish -p tpt-doc-pdf` | tpt-doc-pdf | Rate-limited (429) at 18:44; published on retry at 18:52 |
| 2026-11-10 | `cargo publish -p` for `convert`, `spreadsheet`, `word`, `layout`, `sign`, `ubl`, `diff`, `audit`, `tpt-doc` | 9 crates | Each rate-limited (429) at least once until its retry time; all published on retry |
| 2026-11-10 | `cargo publish -p` for `tpt-doc-cli`, `tpt-doc-mcp` | 2 crates | Each rate-limited (429) once; published on retry |

Times in the 429 rows are local (UTC+13).

## Open items

- **Dev-dependencies (checked 2026-11-10):** every workspace dev-dependency
  that carries a version is published earlier in the order than the crate that
  uses it. The only dev-dependents on workspace crates that are not in the order
  are the unpublished crates (`tpt-doc-bench`, `tpt-doc-templates`).
- **Review the five READMEs not rewritten in the earlier pass:** `tpt-doc-hl7v2`,
  `tpt-doc-word`, `tpt-doc-layout`, `tpt-doc-sign` and `tpt-doc-ubl`. They were
  written earlier and may not match the current API. Their quick-start code is
  unverified. These are now live on crates.io, so a fix needs 0.1.1.
- **crates.io names:** all 17 names returned 404 (free) on 2026-11-10 before the
  publish. The publishing account is the one whose token is in the local cargo
  credentials.
- **Release tags:** the CHANGELOG links now point at crates.io. No `v0.1.0` git
  tag exists on the remote yet. The tag-triggered release workflow
  (`.github/workflows/release.yml`) would republish on a tag push, so do not push
  a `v*.*.*` tag unless a republish is intended.
