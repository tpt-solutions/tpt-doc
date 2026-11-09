//! Protocol-level tests: drive `handle_line` with JSON-RPC requests.

use serde_json::{Value, json};
use tpt_doc_mcp::handle_line;

fn call(request: Value) -> Value {
    let line = request.to_string();
    let response = handle_line(&line).expect("a response for a request with an id");
    serde_json::from_str(&response).expect("response is JSON")
}

fn tool_call(name: &str, arguments: Value) -> Value {
    call(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": { "name": name, "arguments": arguments },
    }))
}

fn text_of(response: &Value) -> String {
    response["result"]["content"][0]["text"]
        .as_str()
        .expect("text content")
        .to_owned()
}

#[test]
fn initialize_reports_tools_capability() {
    let response = call(json!({"jsonrpc":"2.0","id":0,"method":"initialize","params":{}}));
    assert_eq!(response["result"]["protocolVersion"], "2024-11-05");
    assert!(response["result"]["capabilities"]["tools"].is_object());
    assert_eq!(response["result"]["serverInfo"]["name"], "tpt-doc-mcp");
}

#[test]
fn notifications_get_no_response() {
    let line = json!({"jsonrpc":"2.0","method":"notifications/initialized"}).to_string();
    assert!(handle_line(&line).is_none());
}

#[test]
fn tools_list_names_all_four_tools() {
    let response = call(json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}));
    let names: Vec<&str> = response["result"]["tools"]
        .as_array()
        .expect("tools array")
        .iter()
        .map(|t| t["name"].as_str().expect("name"))
        .collect();
    assert_eq!(
        names,
        [
            "validate_document",
            "convert_hl7_to_fhir",
            "extract_text",
            "generate_invoice"
        ]
    );
}

#[test]
fn malformed_json_is_a_parse_error() {
    let response: Value =
        serde_json::from_str(&handle_line("{nope").expect("response")).expect("json");
    assert_eq!(response["error"]["code"], -32700);
}

#[test]
fn unknown_method_is_method_not_found() {
    let response = call(json!({"jsonrpc":"2.0","id":3,"method":"resources/list"}));
    assert_eq!(response["error"]["code"], -32601);
}

#[test]
fn validate_document_returns_a_json_report() {
    let hl7 = "MSH|^~\\&|A|B|C|D|20260101||ADT^A01|1|P|2.5.1\rPID|1||12345\r";
    let response = tool_call("validate_document", json!({ "content": hl7 }));
    let report: Value = serde_json::from_str(&text_of(&response)).expect("report json");
    assert!(report.is_object(), "{report}");
}

#[test]
fn validate_document_needs_some_content() {
    let response = tool_call("validate_document", json!({}));
    assert_eq!(response["result"]["isError"], true);
}

#[test]
fn invalid_base64_is_an_error_result_not_a_protocol_error() {
    let response = tool_call("extract_text", json!({ "content_base64": "%%%" }));
    assert_eq!(response["result"]["isError"], true);
    assert!(response.get("error").is_none());
}

#[test]
fn convert_hl7_to_fhir_returns_a_bundle() {
    let hl7 = "MSH|^~\\&|A|B|C|D|20260101120000||ADT^A01|1|P|2.5.1\rPID|1||12345^^^HOSP^MR||Doe^Jane||19700101|F\rPV1|1|I\r";
    let response = tool_call("convert_hl7_to_fhir", json!({ "content": hl7 }));
    assert_eq!(response["result"]["isError"], false, "{response}");
    let bundle: Value = serde_json::from_str(&text_of(&response)).expect("bundle json");
    assert_eq!(bundle["resourceType"], "Bundle");
}

#[test]
fn generate_invoice_rejects_an_invalid_invoice() {
    let response = tool_call("generate_invoice", json!({ "invoice": { "id": "" } }));
    assert_eq!(response["result"]["isError"], true);
}

#[test]
fn generate_invoice_requires_an_invoice_object() {
    let response = tool_call("generate_invoice", json!({}));
    assert_eq!(response["result"]["isError"], true);
}

#[test]
fn unknown_tool_is_an_invalid_params_error() {
    let response = tool_call("launch_missiles", json!({}));
    assert_eq!(response["error"]["code"], -32602);
}

#[test]
fn generate_invoice_emits_ubl_xml_for_a_valid_invoice() {
    let xml = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../tpt-doc-ubl/tests/fixtures/peppol_invoice.xml"
    ))
    .expect("fixture");
    let invoice = tpt_doc_ubl::Invoice::from_xml(&xml).expect("parse fixture");
    let as_json = serde_json::to_value(&invoice).expect("invoice to json");

    let response = tool_call("generate_invoice", json!({ "invoice": as_json }));
    assert_eq!(response["result"]["isError"], false, "{response}");
    let out = text_of(&response);
    assert!(out.contains("Invoice"), "{out}");
    let back = tpt_doc_ubl::Invoice::from_xml(out.as_bytes()).expect("output parses");
    assert_eq!(back, invoice);
}
