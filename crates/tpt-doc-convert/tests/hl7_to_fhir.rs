//! Conversion tests over the hl7v2 crate's fixtures plus targeted edge cases.

use tpt_doc_convert::hl7v2_to_fhir;
use tpt_doc_fhir::{Bundle, EncounterStatus, Gender, ObservationStatus, Resource};

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(format!(
        "{}/../tpt-doc-hl7v2/tests/fixtures/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("fixture")
}

#[test]
fn adt_a01_yields_patient_and_encounter() {
    let c = hl7v2_to_fhir(&fixture("adt_a01.hl7")).expect("convert");
    assert_eq!(c.bundle.entries.len(), 2);

    let Resource::Patient(p) = &c.bundle.entries[0] else {
        panic!("first entry must be the Patient");
    };
    assert_eq!(p.id, "12345");
    assert_eq!(p.gender, Some(Gender::Male));
    assert_eq!(p.birth_date.as_deref(), Some("1970-01-01"));
    assert_eq!(p.name[0].family.as_deref(), Some("Smith"));
    assert_eq!(p.name[0].given, vec!["John".to_owned()]);
    assert_eq!(p.identifier[0].system.as_deref(), Some("urn:hl7v2:NZHPI"));

    let Resource::Encounter(e) = &c.bundle.entries[1] else {
        panic!("second entry must be the Encounter");
    };
    assert_eq!(e.status, EncounterStatus::InProgress);
    assert_eq!(e.class.code, "IMP");
    assert_eq!(
        e.subject.as_ref().and_then(|s| s.reference.as_deref()),
        Some("Patient/12345")
    );
    // EVN-2 has a time but no UTC offset: date kept, and the loss is reported.
    assert_eq!(
        e.period.as_ref().and_then(|p| p.start.as_deref()),
        Some("2026-10-01")
    );
    assert!(c.warnings.iter().any(|w| w.contains("no UTC offset")));
    assert!(!c.report.has_errors());
}

#[test]
fn oru_r01_yields_observations_and_reports_unmappable_values() {
    let c = hl7v2_to_fhir(&fixture("oru_r01.hl7")).expect("convert");
    let counts = c.bundle.counts();
    assert!(counts.contains(&("Patient", 1)));
    assert!(counts.contains(&("Observation", 2)));

    let Resource::Observation(glucose) = &c.bundle.entries[1] else {
        panic!("expected Observation");
    };
    // The fixture carries `F` in OBX-9, not the result-status field OBX-11, so
    // the status is unknown and the converter says so.
    assert_eq!(glucose.status, ObservationStatus::Unknown);
    assert!(c.warnings.iter().any(|w| w.contains("result status")));
    assert_eq!(glucose.code.coding[0].code, "GLU");
    let q = glucose.value_quantity.as_ref().expect("quantity");
    assert!((q.value - 5.2).abs() < f64::EPSILON);
    assert_eq!(q.unit.as_deref(), Some("mmol/L"));
    assert_eq!(glucose.id, "MSG00002-1");

    // The ST comment cannot be a valueQuantity: it must be reported, not lost silently.
    assert!(c.warnings.iter().any(|w| w.contains("value type `ST`")));
}

#[test]
fn bundle_survives_json_round_trip() {
    let c = hl7v2_to_fhir(&fixture("oru_r01.hl7")).expect("convert");
    let json = c.bundle.to_json().expect("json");
    let back = Bundle::from_json(&json).expect("parse back");
    assert_eq!(back, c.bundle);
}

#[test]
fn offset_timestamps_are_kept_as_fhir_datetimes() {
    let msg = b"MSH|^~\\&|A|B|C|D|||ORU^R01|M1|P|2.5.1\rPID|1||9^^^X||Doe^Jo\r\
                OBX|1|NM|HR^Heart rate||72|bpm|||||F|||20261001093000+1300";
    let c = hl7v2_to_fhir(msg).expect("convert");
    let Resource::Observation(o) = &c.bundle.entries[1] else {
        panic!("expected Observation");
    };
    assert_eq!(
        o.effective_date_time.as_deref(),
        Some("2026-10-01T09:30:00+13:00")
    );
    assert!(c.warnings.is_empty(), "{:?}", c.warnings);
}

#[test]
fn escapes_in_names_are_resolved() {
    let msg = b"MSH|^~\\&|A|B|C|D|||ADT^A08|M1|P|2.5.1\rPID|1||9||O'Neil\\T\\Sons^Jo";
    let c = hl7v2_to_fhir(msg).expect("convert");
    let Resource::Patient(p) = &c.bundle.entries[0] else {
        panic!("expected Patient");
    };
    assert_eq!(p.name[0].family.as_deref(), Some("O'Neil&Sons"));
}

#[test]
fn discharge_maps_to_finished_and_unknown_class_is_reported() {
    let msg = b"MSH|^~\\&|A|B|C|D|||ADT^A03|M1|P|2.5.1\rPID|1||9||Doe^Jo\rPV1|1|I";
    let c = hl7v2_to_fhir(msg).expect("convert");
    let Resource::Encounter(e) = &c.bundle.entries[1] else {
        panic!("expected Encounter");
    };
    assert_eq!(e.status, EncounterStatus::Finished);

    let msg = b"MSH|^~\\&|A|B|C|D|||ADT^A01|M1|P|2.5.1\rPID|1||9||Doe^Jo\rPV1|1|Z";
    let c = hl7v2_to_fhir(msg).expect("convert");
    assert_eq!(c.bundle.entries.len(), 1);
    assert!(c.warnings.iter().any(|w| w.contains("PV1-2")));
}

#[test]
fn unsupported_and_malformed_messages_are_errors_not_panics() {
    assert!(hl7v2_to_fhir(b"MSH|^~\\&|A|B|C|D|||ORM^O01|M1|P|2.5.1").is_err());
    assert!(hl7v2_to_fhir(b"PID|1||9").is_err());
    assert!(hl7v2_to_fhir(b"").is_err());
    assert!(hl7v2_to_fhir(&[0xFF, 0x00, b'|']).is_err());
}

#[test]
fn result_status_codes_map_per_obx_11() {
    for (code, expected) in [
        ("F", ObservationStatus::Final),
        ("P", ObservationStatus::Preliminary),
        ("C", ObservationStatus::Corrected),
        ("X", ObservationStatus::Cancelled),
        ("D", ObservationStatus::EnteredInError),
    ] {
        let msg = format!(
            "MSH|^~\\&|A|B|C|D|||ORU^R01|M1|P|2.5.1\rPID|1||9||Doe^Jo\r\
             OBX|1|NM|HR^Heart rate||72|bpm|||||{code}"
        );
        let c = hl7v2_to_fhir(msg.as_bytes()).expect("convert");
        let Resource::Observation(o) = &c.bundle.entries[1] else {
            panic!("expected Observation");
        };
        assert_eq!(o.status, expected, "OBX-11 `{code}`");
    }
}
