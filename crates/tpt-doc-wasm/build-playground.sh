#!/usr/bin/env sh
# Build the wasm module and generate the JavaScript glue for the playground.
#
# Needs: the wasm32-unknown-unknown target and the wasm-bindgen CLI at the same
# version as the wasm-bindgen crate (0.2.126 in Cargo.toml).
set -eu
cd "$(dirname "$0")/../.."
cargo build -p tpt-doc-wasm --target wasm32-unknown-unknown --release
WASM=target/wasm32-unknown-unknown/release/tpt_doc_wasm.wasm
wasm-bindgen --target web --out-dir crates/tpt-doc-wasm/playground/pkg "$WASM"
wasm-bindgen --target nodejs --out-dir crates/tpt-doc-wasm/playground/pkg-node "$WASM"
echo "built; open crates/tpt-doc-wasm/playground/index.html through a local web server"
