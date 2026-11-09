//! `Observation::from_xml` reads by element position, like `Patient::from_xml`:
//! the root must be an `Observation`, a value is read only where the resource
//! defines it, and `<x value="…"></x>` is read like `<x value="…"/>`.

use tpt_doc_fhir::prelude::*;

const NS: &str = r#"xmlns="http://hl7.org/fhir""#;

fn document(body: &str) -> String {
    format!(r#"<Observation {NS}>{body}</Observation>"#)
}

const VALID_BODY: &str = r#"
    <id value="obs1"/>
    <status value="final"/>
    <code>
      <coding><system value="urn:loinc"/><code value="2345-7"/><display value="Glucose"/></coding>
      <text value="Glucose"/>
    </code>
    <subject><reference value="Patient/p1"/></subject>
    <valueQuantity><value value="5.4"/><unit value="mmol/L"/></valueQuantity>
    <effectiveDateTime value="2026-01-01T10:00:00Z"/>
"#;

#[test]
fn parses_a_complete_document() {
    let observation = Observation::from_xml(document(VALID_BODY).as_bytes()).expect("parse");
    assert_eq!(observation.id, "obs1");
    assert_eq!(observation.code.coding[0].code, "2345-7");
    assert_eq!(observation.code.text.as_deref(), Some("Glucose"));
    let quantity = observation.value_quantity.expect("quantity");
    assert!((quantity.value - 5.4).abs() < f64::EPSILON);
    assert_eq!(quantity.unit.as_deref(), Some("mmol/L"));
}

#[test]
fn rejects_a_non_observation_root() {
    let xml = format!(r#"<Patient {NS}><id value="p1"/></Patient>"#);
    let err = Observation::from_xml(xml.as_bytes()).expect_err("wrong root");
    assert!(err.to_string().contains("Observation"), "{err}");
}

#[test]
fn reads_status_written_as_a_start_end_pair() {
    let body = VALID_BODY.replace(
        r#"<status value="final"/>"#,
        r#"<status value="final"></status>"#,
    );
    let observation = Observation::from_xml(document(&body).as_bytes()).expect("parse");
    assert_eq!(observation.status, ObservationStatus::Final);
}

#[test]
fn nested_status_does_not_set_the_observation_status() {
    let body = r#"<status value="final"/>
           <code><coding><code value="1"/></coding><status value="bogus"/></code>"#;
    let observation = Observation::from_xml(document(body).as_bytes()).expect("parse");
    assert_eq!(observation.status, ObservationStatus::Final);
}

#[test]
fn nested_id_does_not_replace_the_resource_id() {
    let body = r#"<id value="obs1"/><status value="final"/>
           <subject><id value="sub"/><reference value="Patient/p1"/></subject>"#;
    let observation = Observation::from_xml(document(body).as_bytes()).expect("parse");
    assert_eq!(observation.id, "obs1");
}

#[test]
fn quantity_value_written_as_a_start_end_pair_is_read() {
    let body = r#"<status value="final"/>
           <valueQuantity><value value="2.5"></value><unit value="g"></unit></valueQuantity>"#;
    let observation = Observation::from_xml(document(body).as_bytes()).expect("parse");
    let quantity = observation.value_quantity.expect("quantity");
    assert!((quantity.value - 2.5).abs() < f64::EPSILON);
    assert_eq!(quantity.unit.as_deref(), Some("g"));
}

#[test]
fn missing_status_is_reported() {
    let err = Observation::from_xml(document(r#"<id value="obs1"/>"#).as_bytes())
        .expect_err("status is mandatory");
    assert!(err.to_string().contains("status"), "{err}");
}

#[test]
fn unclosed_element_is_rejected() {
    let xml = format!(r#"<Observation {NS}><status value="final"/><code>"#);
    assert!(Observation::from_xml(xml.as_bytes()).is_err());
}
