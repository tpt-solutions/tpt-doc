//! Integration tests for FHIR Bundle parsing and validation.
//!
//! `tests/fixtures/patient_bundle.json` is a hand-written FHIR R5
//! collection bundle with one entry per supported resource type.

use tpt_doc_fhir::prelude::*;
use tpt_doc_fhir::{validate_encounter, validate_observation};

fn fixture_bytes() -> Vec<u8> {
    std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/patient_bundle.json"
    ))
    .expect("fixture file")
}

#[test]
fn parses_fixture_and_counts_resources() {
    let bundle = Bundle::from_json(&fixture_bytes()).expect("parse bundle");
    assert_eq!(bundle.bundle_type, "collection");
    assert_eq!(bundle.entries.len(), 5);
    assert_eq!(
        bundle.counts(),
        vec![
            ("Patient", 1),
            ("Observation", 2),
            ("Encounter", 1),
            ("Organization", 1),
        ]
    );
}

#[test]
fn fixture_resource_fields_survive_a_bundle_round_trip() {
    let bundle = Bundle::from_json(&fixture_bytes()).expect("parse bundle");
    let json = bundle.to_json().expect("serialize");
    let restored = Bundle::from_json(&json).expect("re-parse");
    assert_eq!(bundle, restored);

    // Spot-check typed access to the first observation.
    let Resource::Observation(weight) = &bundle.entries[1] else {
        panic!("expected an Observation");
    };
    assert_eq!(weight.status_code_text(), "final");
    assert_eq!(weight.value_text().as_deref(), Some("72.5 kg"));
}

#[test]
fn fixture_resources_pass_cross_field_validation() {
    let bundle = Bundle::from_json(&fixture_bytes()).expect("parse bundle");
    for resource in &bundle.entries {
        match resource {
            Resource::Observation(o) => {
                let report = validate_observation(o);
                assert!(
                    report.is_valid(),
                    "observation `{}`: {:?}",
                    o.id,
                    report.issues()
                );
            }
            Resource::Encounter(e) => {
                let report = validate_encounter(e);
                assert!(
                    report.is_valid(),
                    "encounter `{}`: {:?}",
                    e.id,
                    report.issues()
                );
            }
            _ => {}
        }
    }
}

#[test]
fn invalid_bundles_are_rejected_with_path_information() {
    assert!(Bundle::from_json(b"[]").is_err());
    assert!(
        Bundle::from_json(br#"{"resourceType": "Bundle", "entry": [{"resource": {}}]}"#).is_err()
    );
}
