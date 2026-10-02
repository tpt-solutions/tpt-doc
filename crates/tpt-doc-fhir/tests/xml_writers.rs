//! XML writer validation: a control character cannot be represented in XML,
//! so serialization must fail rather than emit malformed FHIR XML.

use tpt_doc_fhir::prelude::*;

#[test]
fn illegal_control_character_in_patient_xml_is_rejected() {
    let patient = Patient::builder()
        .id("nz-12345")
        .name(HumanName::new("Bad\u{1}Name", "John"))
        .build();

    let err = patient.to_xml().expect_err("U+0001 must be rejected");
    assert!(
        err.to_string().contains("U+0001"),
        "error should name the code point: {err}"
    );
}

#[test]
fn illegal_control_character_in_patient_json_is_unaffected() {
    // JSON may legitimately contain control characters once escaped, so the
    // XML-only restriction must not leak into the JSON writer.
    let patient = Patient::builder()
        .id("nz-12345")
        .name(HumanName::new("Bad\u{1}Name", "John"))
        .build();

    let json = patient.to_json().expect("JSON permits escaped controls");
    let text = String::from_utf8_lossy(&json).into_owned();
    assert!(text.contains("\\u0001"), "JSON should escape the control: {text}");
}

#[test]
fn tab_newline_and_return_are_accepted_in_xml() {
    let patient = Patient::builder()
        .id("nz-12345")
        .name(HumanName::new("a\tb\nc\rd", "John"))
        .build();

    assert!(patient.to_xml().is_ok(), "these controls are legal XML");
}
