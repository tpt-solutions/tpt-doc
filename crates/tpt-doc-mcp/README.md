# tpt-doc-mcp

A [Model Context Protocol](https://modelcontextprotocol.io) server that exposes
tpt-doc to AI agents over stdio.

```sh
cargo install --path crates/tpt-doc-mcp   # installs the `tpt-mcp` binary
```

Register `tpt-mcp` as a stdio server in your MCP client. It provides:

| Tool | Input | Output |
|---|---|---|
| `validate_document` | `content` or `content_base64` | JSON validation report; `isError` on violations |
| `convert_hl7_to_fhir` | HL7 v2 message | FHIR R5 Bundle (JSON) and mapping warnings |
| `extract_text` | DOCX document | Plain text |
| `generate_invoice` | `invoice` JSON object | UBL 2.1 XML, refused if PEPPOL rules fail |

The protocol logic lives in the `tpt_doc_mcp` library (`handle_line`), so the
behaviour is tested without starting a process.
