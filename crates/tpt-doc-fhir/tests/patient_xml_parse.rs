//! `Patient::from_xml` must read the resource structurally: the root must be
//! a `Patient`, values must be taken only from the elements that define them
//! (not from same-named elements nested elsewhere), and `<x value="…"></x>`
//! must be read the same as `<x value="…"/>`.

use tpt_doc_fhir::prelude::*;

const NS: &str = r#"xmlns="http://hl7.org/fhir""#;

#[test]
fn rejects_a_non_patient_root() {
    let xml = format!(r#"<Observation {NS}><id value="x"/></Observation>"#);
    let err = Patient::from_xml(xml.as_bytes()).expect_err("wrong root must fail");
    assert!(err.to_string().contains("Patient"), "{err}");
}

#[test]
fn rejects_input_without_any_resource() {
    let err = Patient::from_xml(b"<?xml version=\"1.0\"?>").expect_err("no root");
    assert!(err.to_string().contains("Patient"), "{err}");
}

#[test]
fn rejects_an_unclosed_element() {
    let xml = format!(r#"<Patient {NS}><id value="a"/><name><family value="B"/>"#);
    assert!(Patient::from_xml(xml.as_bytes()).is_err());
}

#[test]
fn nested_names_do_not_leak_into_the_patient() {
    let xml = format!(
        r#"<Patient {NS}><id value="p1"/>
             <name><family value="Real"/></name>
             <contact><name><family value="Nested"/></name></contact>
           </Patient>"#
    );
    let patient = Patient::from_xml(xml.as_bytes()).expect("parse");
    assert_eq!(patient.name.len(), 1, "{:?}", patient.name);
    assert_eq!(patient.name[0].family.as_deref(), Some("Real"));
}

#[test]
fn nested_gender_and_birth_date_are_ignored() {
    let xml = format!(
        r#"<Patient {NS}><id value="p1"/>
             <contact><gender value="female"/><birthDate value="1900-01-01"/></contact>
           </Patient>"#
    );
    let patient = Patient::from_xml(xml.as_bytes()).expect("parse");
    assert_eq!(patient.gender, None);
    assert_eq!(patient.birth_date, None);
}

#[test]
fn nested_identifier_value_does_not_overwrite_the_identifier() {
    let xml = format!(
        r#"<Patient {NS}><id value="p1"/>
             <identifier><system value="urn:a"/><value value="123"/></identifier>
             <extension><value value="999"/></extension>
           </Patient>"#
    );
    let patient = Patient::from_xml(xml.as_bytes()).expect("parse");
    assert_eq!(patient.identifier.len(), 1);
    assert_eq!(patient.identifier[0].value, "123");
}

#[test]
fn start_end_pair_with_value_is_read_like_an_empty_element() {
    let xml = format!(
        r#"<Patient {NS}><id value="p1"></id>
             <name><family value="Smith"></family><given value="Ann"></given></name>
             <birthDate value="1970-01-01"></birthDate>
           </Patient>"#
    );
    let patient = Patient::from_xml(xml.as_bytes()).expect("parse");
    assert_eq!(patient.id, "p1");
    assert_eq!(patient.name[0].family.as_deref(), Some("Smith"));
    assert_eq!(patient.name[0].given, vec!["Ann".to_string()]);
    assert_eq!(patient.birth_date.as_deref(), Some("1970-01-01"));
}

#[test]
fn round_trips_through_to_xml() {
    let patient = Patient::builder().id("p9").birth_date("1980-02-02").build();
    let xml = patient.to_xml().expect("write");
    let back = Patient::from_xml(&xml).expect("read");
    assert_eq!(back.id, "p9");
    assert_eq!(back.birth_date.as_deref(), Some("1980-02-02"));
}
