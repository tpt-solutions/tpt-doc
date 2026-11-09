//! XML for `Encounter` and `Organization`: the writer and reader must agree,
//! and the reader follows the same rules as the other resources (root must
//! match, values read by position, mandatory elements reported).

use tpt_doc_fhir::prelude::*;

const NS: &str = r#"xmlns="http://hl7.org/fhir""#;

fn encounter() -> Encounter {
    Encounter::from_json(
        br#"{
            "resourceType": "Encounter",
            "id": "e1",
            "status": "finished",
            "class": {"system": "http://terminology.hl7.org/CodeSystem/v3-ActCode", "code": "AMB", "display": "ambulatory"},
            "subject": {"reference": "Patient/p1", "display": "Jane"},
            "period": {"start": "2026-01-01T09:00:00Z", "end": "2026-01-01T10:00:00Z"}
        }"#,
    )
    .expect("sample encounter")
}

fn organization() -> Organization {
    Organization::from_json(
        br#"{"resourceType": "Organization", "id": "o1", "active": false, "name": "Acme Hospital"}"#,
    )
    .expect("sample organization")
}

#[test]
fn encounter_xml_round_trips() {
    let original = encounter();
    let xml = original.to_xml().expect("write");
    assert_eq!(Encounter::from_xml(&xml).expect("read"), original);
}

#[test]
fn organization_xml_round_trips_including_false_active() {
    let original = organization();
    let xml = original.to_xml().expect("write");
    let back = Organization::from_xml(&xml).expect("read");
    assert_eq!(back, original);
    assert_eq!(back.active, Some(false));
}

#[test]
fn encounter_xml_rejects_another_root() {
    let xml = format!(r#"<Organization {NS}><id value="o1"/></Organization>"#);
    let err = Encounter::from_xml(xml.as_bytes()).expect_err("wrong root");
    assert!(err.to_string().contains("Encounter"), "{err}");
}

#[test]
fn organization_xml_rejects_another_root() {
    let xml = format!(r#"<Encounter {NS}><id value="e1"/></Encounter>"#);
    let err = Organization::from_xml(xml.as_bytes()).expect_err("wrong root");
    assert!(err.to_string().contains("Organization"), "{err}");
}

#[test]
fn encounter_xml_requires_a_class() {
    let xml = format!(r#"<Encounter {NS}><id value="e1"/><status value="finished"/></Encounter>"#);
    let err = Encounter::from_xml(xml.as_bytes()).expect_err("class is mandatory");
    assert!(err.to_string().contains("class"), "{err}");
}

#[test]
fn nested_id_does_not_replace_the_encounter_id() {
    let xml = format!(
        r#"<Encounter {NS}><id value="e1"/><status value="planned"/>
           <class><code value="AMB"/></class>
           <subject><id value="other"/><reference value="Patient/p1"/></subject></Encounter>"#
    );
    let encounter = Encounter::from_xml(xml.as_bytes()).expect("parse");
    assert_eq!(encounter.id, "e1");
    assert_eq!(encounter.status, EncounterStatus::Planned);
}

#[test]
fn organization_xml_requires_a_name() {
    let xml = format!(r#"<Organization {NS}><id value="o1"/></Organization>"#);
    let err = Organization::from_xml(xml.as_bytes()).expect_err("name is mandatory");
    assert!(err.to_string().contains("name"), "{err}");
}

#[test]
fn organization_active_must_be_a_boolean() {
    let xml = format!(
        r#"<Organization {NS}><id value="o1"/><active value="maybe"/><name value="A"/></Organization>"#
    );
    assert!(Organization::from_xml(xml.as_bytes()).is_err());
}
