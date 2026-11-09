# Fuzz targets

One target per parser that reads untrusted input. Each target asserts only that
the parser returns (an error is fine) and does not panic, overflow, or allocate
without bound.

Run on Linux or with a nightly toolchain (libFuzzer is not supported by the
MSVC toolchain on Windows):

```sh
cargo install cargo-fuzz
cd fuzz
cargo +nightly fuzz run hl7v2_parse
```

The targets are in `fuzz_targets/`; add a target for each new untrusted-input
parser.
