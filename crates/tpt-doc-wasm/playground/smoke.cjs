// Smoke test for the wasm build. Run with `node smoke.cjs` after
// `build-playground.sh` has generated the pkg-node glue.
const { handle, version } = require("./pkg-node/tpt_doc_wasm.js");
const assert = require("node:assert/strict");

// The HL7 encoding characters are ^~\& ; build the backslash explicitly.
const BS = String.fromCharCode(92);
const MSH = "MSH|^~" + BS + "&|";

assert.match(version(), /^\d+\.\d+\.\d+$/);

// A notification gets no response.
assert.equal(
  handle(JSON.stringify({ jsonrpc: "2.0", method: "notifications/initialized" })),
  undefined,
);

// Initialize reports the server.
const init = JSON.parse(
  handle(JSON.stringify({ jsonrpc: "2.0", id: 1, method: "initialize", params: {} })),
);
assert.equal(init.result.serverInfo.name, "tpt-doc-mcp");

const call = (id, name, args) =>
  JSON.parse(
    handle(JSON.stringify({ jsonrpc: "2.0", id, method: "tools/call", params: { name, arguments: args } })),
  );

// Validate an HL7 admission without a patient name: the report names PID-5.
const check = call(2, "validate_document", {
  content: MSH + "A|B|C|D|20260101||ADT^A01|1|P|2.5.1\rPID|1||12345\r",
});
assert.equal(check.result.isError, true, JSON.stringify(check));
const report = JSON.parse(check.result.content[0].text);
assert.equal(report.valid, false);
assert.ok(report.issues.some((i) => i.message.includes("PID-5")), JSON.stringify(report));

// A message with a patient name is accepted.
const valid = call(5, "validate_document", {
  content: MSH + "A|B|C|D|20260101||ADT^A01|1|P|2.5.1\rPID|1||12345^^^HOSP^MR||Doe^Jane||19700101|F\r",
});
assert.equal(valid.result.isError, false, JSON.stringify(valid));

// Convert an ADT message to a FHIR bundle.
const convert = call(3, "convert_hl7_to_fhir", {
  content: MSH + "A|B|C|D|20260101120000||ADT^A01|1|P|2.5.1\rPID|1||12345^^^HOSP^MR||Doe^Jane||19700101|F\r",
});
assert.equal(convert.result.isError, false, JSON.stringify(convert));
const bundle = JSON.parse(convert.result.content[0].text);
assert.equal(bundle.resourceType, "Bundle");

// Garbage is an error result, not a crash.
const bad = call(4, "convert_hl7_to_fhir", { content: "not hl7" });
assert.equal(bad.result.isError, true);

console.log("wasm smoke test passed, tpt-doc", version());
