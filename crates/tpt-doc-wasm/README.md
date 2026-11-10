# tpt-doc-wasm

A WebAssembly build of tpt-doc, with a browser playground. It exposes a single
request-and-response entry point, so a browser or Node.js program can parse,
validate and convert documents without a server.

- **Status:** unpublished (`publish = false`). MSRV 1.88. Licence `MIT OR Apache-2.0`.

## API

The library exports two functions:

| Function | Purpose |
|---|---|
| `handle(request: &str) -> Option<String>` | Run one JSON request and return the JSON response |
| `version() -> String` | The crate version |

A request names an operation and carries its input. A malformed request returns
an error inside the response, not a trap.

## Build the playground

```sh
bash crates/tpt-doc-wasm/build-playground.sh
```

The script builds for `wasm32-unknown-unknown`, runs `wasm-bindgen` into
`playground/pkg/`, and leaves the page ready to serve. The `pkg/` folder is
ignored by git.

Serve the `playground/` folder with any static file server, then open it in a
browser. The page imports `./pkg/tpt_doc_wasm.js`, so it must be served over
HTTP rather than opened as a file.

## Smoke test

```sh
node crates/tpt-doc-wasm/playground/smoke.cjs
```

The smoke test loads the Node build and checks a validation request and a
conversion request. It needs Node 22 or later.

## Requirements

- Rust with the `wasm32-unknown-unknown` target.
- `wasm-bindgen-cli` matching the crate's `wasm-bindgen` version (0.2.126).

## Scope and known gaps

- The playground has been checked with the Node smoke test. It has not yet been
  run in a browser.
- The WebAssembly build has no file system access. Input is passed as text in
  the request.

## Licence

Licensed under either of Apache-2.0 or MIT, at your option.
