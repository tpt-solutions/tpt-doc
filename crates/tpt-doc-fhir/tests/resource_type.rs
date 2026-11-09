//! A JSON document must declare the resource type it is parsed as. Without
//! the check, a Patient parser happily accepts an Observation document.

use tpt_doc_fhir::prelude::*;

#[test]
fn patient_from_json_rejects_another_resource_type() {
    let err = Patient::from_json(br#"{"resourceType":"Observation","id":"x"}"#)
        .expect_err("wrong resourceType must fail");
    assert!(err.to_string().contains("resourceType"), "{err}");
}

#[test]
fn organization_from_json_rejects_another_resource_type() {
    let err = Organization::from_json(br#"{"resourceType":"Patient","id":"x","name":"n"}"#)
        .expect_err("wrong resourceType must fail");
    assert!(err.to_string().contains("resourceType"), "{err}");
}

#[test]
fn patient_from_json_accepts_its_own_resource_type() {
    let patient = Patient::from_json(br#"{"resourceType":"Patient","id":"x"}"#).expect("parse");
    assert_eq!(patient.id, "x");
}
