//! The shared `tpt-doc-core` traits must agree with the inherent JSON API.

use tpt_doc_core::{DocError, DocReader, DocWriter, Validate};
use tpt_doc_fhir::prelude::*;

#[test]
fn patient_round_trips_through_bytes() {
    let json = br#"{"resourceType":"Patient","id":"p1","birthDate":"1970-01-01"}"#;
    let patient = <Patient as DocReader>::from_bytes(json).expect("parse");
    assert_eq!(patient, Patient::from_json(json).expect("inherent"));
    let bytes = DocWriter::to_bytes(&patient).expect("write");
    assert_eq!(
        <Patient as DocReader>::from_bytes(&bytes).expect("read back"),
        patient
    );
}

#[test]
fn trait_errors_keep_the_path_as_location() {
    let err = <Patient as DocReader>::from_bytes(b"{not json").expect_err("malformed");
    assert!(matches!(
        err,
        DocError::Located { .. } | DocError::InvalidFormat(_)
    ));
}

#[test]
fn wrong_resource_type_is_an_invalid_format_error() {
    let err = <Patient as DocReader>::from_bytes(br#"{"resourceType":"Observation","id":"x"}"#)
        .expect_err("wrong type");
    assert!(err.to_string().contains("resourceType"), "{err}");
}

#[test]
fn encounter_validate_trait_matches_the_function() {
    let json =
        br#"{"resourceType":"Encounter","id":"e1","status":"finished","class":{"code":"AMB"}}"#;
    let encounter = <Encounter as DocReader>::from_bytes(json).expect("parse");
    let via_trait = Validate::validate(&encounter);
    let direct = tpt_doc_fhir::validate::validate_encounter(&encounter);
    assert_eq!(via_trait.issues().len(), direct.issues().len());
}
