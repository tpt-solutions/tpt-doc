//! FHIR JSON keeps the elements the model does not represent, and the date
//! fields are checked against the FHIR date and dateTime grammar.

use serde_json::Value;
use tpt_doc_fhir::prelude::*;

fn round_trip_keys(bytes: &[u8], parse: impl Fn(&[u8]) -> Vec<u8>) -> Value {
    serde_json::from_slice(&parse(bytes)).expect("json")
}

#[test]
fn patient_keeps_extension_meta_and_text() {
    let json = br#"{"resourceType":"Patient","id":"p1","meta":{"versionId":"3"},
        "text":{"status":"generated","div":"<div>x</div>"},
        "extension":[{"url":"urn:example:flag","valueBoolean":true}]}"#;
    let out = round_trip_keys(json, |b| {
        Patient::from_json(b)
            .expect("parse")
            .to_json()
            .expect("write")
    });
    assert_eq!(out["meta"]["versionId"], "3");
    assert_eq!(out["text"]["status"], "generated");
    assert_eq!(out["extension"][0]["url"], "urn:example:flag");
}

#[test]
fn observation_keeps_notes_it_does_not_model() {
    let json = br#"{"resourceType":"Observation","id":"o1","status":"final",
        "code":{"text":"glucose"},"note":[{"text":"fasting"}]}"#;
    let out = round_trip_keys(json, |b| {
        Observation::from_json(b)
            .expect("parse")
            .to_json()
            .expect("write")
    });
    assert_eq!(out["note"][0]["text"], "fasting");
}

#[test]
fn encounter_and_organization_keep_their_unmodelled_elements() {
    let encounter = br#"{"resourceType":"Encounter","id":"e1","status":"finished",
        "class":{"code":"AMB"},"serviceProvider":{"display":"Ward 3"}}"#;
    let out = round_trip_keys(encounter, |b| {
        Encounter::from_json(b)
            .expect("parse")
            .to_json()
            .expect("write")
    });
    assert_eq!(out["serviceProvider"]["display"], "Ward 3");

    let organization = br#"{"resourceType":"Organization","id":"o1","name":"Acme",
        "telecom":[{"system":"phone","value":"555"}]}"#;
    let out = round_trip_keys(organization, |b| {
        Organization::from_json(b)
            .expect("parse")
            .to_json()
            .expect("write")
    });
    assert_eq!(out["telecom"][0]["value"], "555");
}

#[test]
fn a_resource_without_extra_elements_writes_none() {
    let json = br#"{"resourceType":"Patient","id":"p1"}"#;
    let out = round_trip_keys(json, |b| {
        Patient::from_json(b)
            .expect("parse")
            .to_json()
            .expect("write")
    });
    let keys: Vec<&str> = out
        .as_object()
        .expect("object")
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(keys, ["id", "resourceType"]);
}

#[test]
fn birth_dates_follow_the_fhir_date_grammar() {
    for ok in ["1970", "1970-02", "1970-02-01", "1972-02-29"] {
        let json = format!(r#"{{"resourceType":"Patient","id":"p","birthDate":"{ok}"}}"#);
        assert!(
            Patient::from_json(json.as_bytes()).is_ok(),
            "{ok} should be accepted"
        );
    }
    for bad in [
        "1970-02-30",
        "1970-02-29",
        "1970-13",
        "1970-1-01",
        "70-01-01",
        "1970-00-10",
    ] {
        let json = format!(r#"{{"resourceType":"Patient","id":"p","birthDate":"{bad}"}}"#);
        let err = Patient::from_json(json.as_bytes()).expect_err(bad);
        assert!(err.to_string().contains("birthDate"), "{bad}: {err}");
    }
}

#[test]
fn date_times_accept_offsets_and_reject_impossible_clocks() {
    for ok in [
        "2026-01-01",
        "2026-01-01T10:00:00Z",
        "2026-01-01T10:00:00.250+13:00",
        "2026-01-01T10:00",
    ] {
        let json = format!(
            r#"{{"resourceType":"Observation","id":"o","status":"final","code":{{"text":"x"}},"effectiveDateTime":"{ok}"}}"#
        );
        assert!(
            Observation::from_json(json.as_bytes()).is_ok(),
            "{ok} should be accepted"
        );
    }
    for bad in [
        "2026-01-01T25:00:00Z",
        "2026-01-01T10:00:00+15:00",
        "2026-01-01T10:60",
        "2026-01-01 10:00",
    ] {
        let json = format!(
            r#"{{"resourceType":"Observation","id":"o","status":"final","code":{{"text":"x"}},"effectiveDateTime":"{bad}"}}"#
        );
        assert!(
            Observation::from_json(json.as_bytes()).is_err(),
            "{bad} should be refused"
        );
    }
}

#[test]
fn an_encounter_period_start_is_checked() {
    let json = br#"{"resourceType":"Encounter","id":"e1","status":"finished",
        "class":{"code":"AMB"},"period":{"start":"2026-13-01T09:00:00Z"}}"#;
    let err = Encounter::from_json(json).expect_err("month 13");
    assert!(err.to_string().contains("/period/start"), "{err}");
}

#[test]
fn an_xml_patient_with_an_impossible_birth_date_is_refused() {
    let xml = br#"<Patient xmlns="http://hl7.org/fhir"><id value="p1"/><birthDate value="1970-02-30"/></Patient>"#;
    let err = Patient::from_xml(xml).expect_err("impossible date");
    assert!(err.to_string().contains("birthDate"), "{err}");
}
