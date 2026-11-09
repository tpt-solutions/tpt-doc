//! WebAssembly entry points for the tpt-doc playground.
//!
//! The browser sends one JSON-RPC message per call, exactly as an MCP client
//! would over stdio, and gets back the response. Everything the server does
//! (validate, convert, extract text, generate an invoice) therefore runs in the
//! page, with no server round trip and no document leaving the browser.

#![forbid(unsafe_code)]

use wasm_bindgen::prelude::*;

/// Handle one JSON-RPC request line. Returns the response, or `undefined` in
/// JavaScript for a notification, which gets no response.
#[wasm_bindgen]
pub fn handle(request: &str) -> Option<String> {
    tpt_doc_mcp::handle_line(request)
}

/// The version of the tpt-doc build.
#[wasm_bindgen]
#[must_use]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}
