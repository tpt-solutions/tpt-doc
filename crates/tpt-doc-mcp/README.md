# tpt-doc-mcp

A Model Context Protocol server that exposes tpt-doc to AI assistants. It runs
over stdio using JSON-RPC 2.0, and it implements protocol version `2024-11-05`.

- **Status:** 0.1.0, not yet published. MSRV 1.88. Licence `MIT OR Apache-2.0`.

## Tools

| Tool | What it does |
|---|---|
| `validate_document` | Validate a document; the format is detected from its contents |
| `convert_hl7_to_fhir` | Convert an HL7 v2 message to a FHIR bundle |
| `extract_text` | Extract the text of a DOCX file (DOCX only) |
| `generate_invoice` | Build a UBL invoice from structured input |

## Run

```sh
cargo run -p tpt-doc-mcp --bin tpt-mcp
```

Register the binary in an MCP client's configuration, using the path to the
built `tpt-mcp` executable as the command.

## Limits

The server reads one JSON-RPC message per line from stdin, and it writes one
response per line to stdout. A malformed request returns a JSON-RPC error and
does not stop the server.

## Scope and known gaps

- `extract_text` handles DOCX only. PDF and spreadsheet text extraction are not
  exposed as tools yet.
- The server has no authentication. Run it only as a local subprocess of a
  trusted client.

## Testing

```sh
cargo test -p tpt-doc-mcp
```

## Licence

Licensed under either of Apache-2.0 or MIT, at your option.
