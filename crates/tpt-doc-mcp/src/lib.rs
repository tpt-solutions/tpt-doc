//! A [Model Context Protocol] server exposing tpt-doc to AI agents.
//!
//! The transport is newline-delimited JSON-RPC 2.0 over stdio. This library
//! holds all the protocol logic as pure functions so it can be tested without
//! a process; the `tpt-mcp` binary only moves lines between stdin and stdout.
//!
//! Tools:
//!
//! - `validate_document` — sniff the format, parse, and validate; returns the
//!   structured report as JSON. `isError` is set when a rule is violated.
//! - `convert_hl7_to_fhir` — HL7 v2 → FHIR bundle, with mapping warnings.
//! - `extract_text` — plain text from a DOCX document.
//! - `generate_invoice` — UBL 2.1 XML from an invoice JSON object, refused
//!   when the PEPPOL rules fail.
//!
//! Every tool takes its document as either `content` (UTF-8 text, for HL7 and
//! EDIFACT) or `content_base64` (for binary formats).
//!
//! [Model Context Protocol]: https://modelcontextprotocol.io

use base64::Engine as _;
use serde_json::{Value, json};

use tpt_doc_cli::commands;
use tpt_doc_ubl::{Invoice, UblValidationError};

/// The protocol revision this server speaks.
pub const PROTOCOL_VERSION: &str = "2024-11-05";

const PARSE_ERROR: i64 = -32700;
const INVALID_REQUEST: i64 = -32600;
const METHOD_NOT_FOUND: i64 = -32601;
const INVALID_PARAMS: i64 = -32602;

/// Handle one line of input. Returns the response line, or `None` when the
/// input is a notification and no response is due.
#[must_use]
pub fn handle_line(line: &str) -> Option<String> {
    let request: Value = match serde_json::from_str(line) {
        Ok(value) => value,
        Err(e) => {
            return Some(error_response(
                &Value::Null,
                PARSE_ERROR,
                &format!("parse error: {e}"),
            ));
        }
    };
    handle_request(&request).map(|response| response.to_string())
}

fn handle_request(request: &Value) -> Option<Value> {
    let id = request.get("id").cloned();
    let Some(method) = request.get("method").and_then(Value::as_str) else {
        return Some(error_value(
            &id.unwrap_or(Value::Null),
            INVALID_REQUEST,
            "request has no method",
        ));
    };
    // Notifications carry no id and never get a response.
    let id = id?;
    let params = request.get("params").cloned().unwrap_or(Value::Null);

    let result = match method {
        "initialize" => Ok(json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": { "tools": {} },
            "serverInfo": {
                "name": "tpt-doc-mcp",
                "version": env!("CARGO_PKG_VERSION"),
            },
        })),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({ "tools": tool_definitions() })),
        "tools/call" => call_tool_request(&params),
        other => Err((METHOD_NOT_FOUND, format!("unknown method `{other}`"))),
    };

    Some(match result {
        Ok(value) => json!({ "jsonrpc": "2.0", "id": id, "result": value }),
        Err((code, message)) => error_value(&id, code, &message),
    })
}

fn error_response(id: &Value, code: i64, message: &str) -> String {
    error_value(id, code, message).to_string()
}

fn error_value(id: &Value, code: i64, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message },
    })
}

/// The tool catalogue returned by `tools/list`.
#[must_use]
pub fn tool_definitions() -> Value {
    let document_schema = json!({
        "type": "object",
        "properties": {
            "content": {
                "type": "string",
                "description": "The document as UTF-8 text (HL7 v2, EDIFACT, XML, JSON).",
            },
            "content_base64": {
                "type": "string",
                "description": "The document as base64 (binary formats such as DOCX, XLSX, PDF).",
            },
        },
    });
    json!([
        {
            "name": "validate_document",
            "description": "Detect the format, parse and validate a document. Returns a JSON report with severity, rule id and location for every issue; isError is set when any error-severity issue is found.",
            "inputSchema": document_schema,
        },
        {
            "name": "convert_hl7_to_fhir",
            "description": "Convert an HL7 v2 message (ADT or ORU) to a FHIR R5 Bundle in JSON. Returns the bundle and any mapping warnings.",
            "inputSchema": document_schema,
        },
        {
            "name": "extract_text",
            "description": "Extract the plain text of a DOCX document. Other formats are refused with an explanation.",
            "inputSchema": document_schema,
        },
        {
            "name": "generate_invoice",
            "description": "Build a UBL 2.1 invoice (XML) from an invoice JSON object. The PEPPOL BIS Billing 3.0 rules run first; if any fail, no XML is returned and the issues are reported.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "invoice": {
                        "type": "object",
                        "description": "An Invoice as serialized by tpt-doc-ubl (serde field names).",
                    },
                },
                "required": ["invoice"],
            },
        },
    ])
}

/// The outcome of a tool call, before it is wrapped in the MCP envelope.
struct ToolOutput {
    texts: Vec<String>,
    is_error: bool,
}

impl ToolOutput {
    fn ok(text: String) -> Self {
        Self {
            texts: vec![text],
            is_error: false,
        }
    }

    fn error(text: String) -> Self {
        Self {
            texts: vec![text],
            is_error: true,
        }
    }
}

fn call_tool_request(params: &Value) -> Result<Value, (i64, String)> {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or((INVALID_PARAMS, "tools/call needs a `name`".to_owned()))?;
    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));

    let output = match name {
        "validate_document" => validate_document(&arguments),
        "convert_hl7_to_fhir" => convert_hl7_to_fhir(&arguments),
        "extract_text" => extract_text(&arguments),
        "generate_invoice" => generate_invoice(&arguments),
        other => return Err((INVALID_PARAMS, format!("unknown tool `{other}`"))),
    };

    let content: Vec<Value> = output
        .texts
        .into_iter()
        .map(|text| json!({ "type": "text", "text": text }))
        .collect();
    Ok(json!({ "content": content, "isError": output.is_error }))
}

/// Read the document from `content` or `content_base64`.
fn input_bytes(arguments: &Value) -> Result<Vec<u8>, String> {
    if let Some(text) = arguments.get("content").and_then(Value::as_str) {
        return Ok(text.as_bytes().to_vec());
    }
    if let Some(encoded) = arguments.get("content_base64").and_then(Value::as_str) {
        return base64::engine::general_purpose::STANDARD
            .decode(encoded.trim())
            .map_err(|e| format!("content_base64 is not valid base64: {e}"));
    }
    Err("pass the document as `content` (text) or `content_base64`".to_owned())
}

fn validate_document(arguments: &Value) -> ToolOutput {
    let bytes = match input_bytes(arguments) {
        Ok(bytes) => bytes,
        Err(message) => return ToolOutput::error(message),
    };
    match commands::validate(&bytes, true) {
        Ok(outcome) => {
            let text = outcome.output;
            if outcome.valid {
                ToolOutput::ok(text)
            } else {
                ToolOutput::error(text)
            }
        }
        Err(message) => ToolOutput::error(message),
    }
}

fn convert_hl7_to_fhir(arguments: &Value) -> ToolOutput {
    let bytes = match input_bytes(arguments) {
        Ok(bytes) => bytes,
        Err(message) => return ToolOutput::error(message),
    };
    match commands::convert(&bytes) {
        Ok((json, warnings)) => {
            let bundle = String::from_utf8_lossy(&json).into_owned();
            let mut texts = vec![bundle];
            if !warnings.is_empty() {
                texts.push(format!("warnings:\n{}", warnings.join("\n")));
            }
            ToolOutput {
                texts,
                is_error: false,
            }
        }
        Err(message) => ToolOutput::error(message),
    }
}

fn extract_text(arguments: &Value) -> ToolOutput {
    let bytes = match input_bytes(arguments) {
        Ok(bytes) => bytes,
        Err(message) => return ToolOutput::error(message),
    };
    match commands::text(&bytes) {
        Ok(text) => ToolOutput::ok(text),
        Err(message) => ToolOutput::error(message),
    }
}

fn generate_invoice(arguments: &Value) -> ToolOutput {
    let Some(value) = arguments.get("invoice") else {
        return ToolOutput::error("generate_invoice needs an `invoice` object".to_owned());
    };
    let invoice: Invoice = match serde_json::from_value(value.clone()) {
        Ok(invoice) => invoice,
        Err(e) => return ToolOutput::error(format!("invoice JSON is not valid: {e}")),
    };
    let violations = tpt_doc_ubl::validate(&invoice);
    if !violations.is_empty() {
        return ToolOutput::error(describe_violations(&violations));
    }
    match invoice.to_xml() {
        Ok(xml) => ToolOutput::ok(String::from_utf8_lossy(&xml).into_owned()),
        Err(e) => ToolOutput::error(e.to_string()),
    }
}

fn describe_violations(violations: &[UblValidationError]) -> String {
    let mut lines = vec!["invoice violates PEPPOL rules:".to_owned()];
    lines.extend(violations.iter().map(|v| format!("- {v}")));
    lines.join("\n")
}
