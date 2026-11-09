//! FHIR decimals keep their written precision: `5.40` and `5.4` are the same
//! number but are different quantities once they come from a document.

use tpt_doc_fhir::Observation;
use tpt_doc_fhir::types::Quantity;

fn quantity(json: &str) -> Quantity {
    serde_json::from_str(json).expect("quantity json")
}

#[test]
fn the_json_digits_are_written_back_unchanged() {
    let q = quantity(r#"{"value": 5.40, "unit": "mg"}"#);
    let out = serde_json::to_string(&q).expect("write");
    assert!(out.contains("5.40"), "{out}");
    assert_eq!(q.value, 5.4);
}

#[test]
fn trailing_zeros_are_precision_so_5_40_and_5_4_differ() {
    assert_ne!(
        quantity(r#"{"value": 5.40}"#),
        quantity(r#"{"value": 5.4}"#)
    );
}

#[test]
fn a_built_quantity_equals_its_own_read_back() {
    let built = Quantity::new(5.4, "mg");
    let read = quantity(&serde_json::to_string(&built).expect("write"));
    assert_eq!(built, read);
}

#[test]
fn a_built_quantity_still_equals_a_document_quantity_with_the_same_text_absent() {
    // A quantity built in code has no written text, so it compares on its
    // number alone against one read from a document.
    assert_eq!(
        Quantity::new(5.4, "mg"),
        quantity(r#"{"value": 5.4, "unit": "mg"}"#)
    );
}

#[test]
fn an_observation_xml_value_keeps_its_digits() {
    let xml = br#"<Observation xmlns="http://hl7.org/fhir"><status value="final"/>
        <code><text value="weight"/></code>
        <valueQuantity><value value="72.50"/><unit value="kg"/></valueQuantity></Observation>"#;
    let observation = Observation::from_xml(xml).expect("parse");
    assert_eq!(observation.value_text().as_deref(), Some("72.50 kg"));
    let out = String::from_utf8(observation.to_xml().expect("write")).expect("utf-8");
    assert!(out.contains(r#"<value value="72.50"/>"#), "{out}");
}

#[test]
fn an_observation_json_value_keeps_its_digits() {
    let json = br#"{"resourceType":"Observation","id":"o","status":"final",
        "code":{"text":"weight"},"valueQuantity":{"value":72.50,"unit":"kg"}}"#;
    let observation = Observation::from_json(json).expect("parse");
    let out = String::from_utf8(observation.to_json().expect("write")).expect("utf-8");
    assert!(out.contains("72.50"), "{out}");
}
