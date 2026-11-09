//! `tpt-mcp`: serve the tpt-doc tools over stdio (newline-delimited JSON-RPC).
//!
//! Register it with an MCP client as a stdio server whose command is `tpt-mcp`.

use std::io::{self, BufRead, Write};

fn main() -> io::Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout().lock();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        if let Some(response) = tpt_doc_mcp::handle_line(&line) {
            writeln!(stdout, "{response}")?;
            stdout.flush()?;
        }
    }
    Ok(())
}
