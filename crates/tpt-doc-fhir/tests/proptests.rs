//! Property-based tests: builder → JSON round-trips for every resource type.

use proptest::prelude::*;
use tpt_doc_fhir::patient::Gender;
use tpt_doc_fhir::prelude::*;

fn json_safe(s: &str) -> bool {
    s.chars().all(|c| {
        matches!(
            c,
            '\t' | '\n' | '\r' | '\u{20}'..='\u{D7FF}' | '\u{E000}'..='\u{FFFD}' | '\u{10000}'..='\u{10FFFF}'
        )
    })
}

fn arb_text(max: usize) -> impl Strategy<Value = String> {
    any::<String>().prop_filter("json-safe text", move |s| json_safe(s) && s.len() < max)
}

fn arb_gender() -> impl Strategy<Value = Gender> {
    prop_oneof![
        Just(Gender::Male),
        Just(Gender::Female),
        Just(Gender::Other),
        Just(Gender::Unknown),
    ]
}

fn arb_observation_status() -> impl Strategy<Value = ObservationStatus> {
    prop_oneof![
        Just(ObservationStatus::Registered),
        Just(ObservationStatus::Preliminary),
        Just(ObservationStatus::Final),
        Just(ObservationStatus::Amended),
        Just(ObservationStatus::Corrected),
        Just(ObservationStatus::Cancelled),
        Just(ObservationStatus::EnteredInError),
        Just(ObservationStatus::Unknown),
    ]
}

fn arb_encounter_status() -> impl Strategy<Value = EncounterStatus> {
    prop_oneof![
        Just(EncounterStatus::Planned),
        Just(EncounterStatus::InProgress),
        Just(EncounterStatus::Finished),
        Just(EncounterStatus::Cancelled),
        Just(EncounterStatus::EnteredInError),
        Just(EncounterStatus::Unknown),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    #[test]
    fn patient_round_trips_through_json(
        id in arb_text(32),
        family in arb_text(32),
        given in arb_text(32),
        gender in arb_gender(),
        birth_date in arb_text(10),
    ) {
        let patient = Patient::builder()
            .id(id.clone())
            .name(HumanName::new(family, given))
            .gender(gender)
            .birth_date(birth_date)
            .build();
        let restored = Patient::from_json(&patient.to_json().expect("json")).expect("parse");
        prop_assert_eq!(restored, patient);
    }

    #[test]
    fn observation_round_trips_through_json(
        id in arb_text(32),
        status in arb_observation_status(),
        code in arb_text(32),
        value in any::<f64>().prop_filter("finite", |f: &f64| f.is_finite()),
        unit in arb_text(12),
    ) {
        let observation = Observation::builder()
            .id(id.clone())
            .status(status)
            .code(CodeableConcept::from_coding(Coding::new("http://loinc.org", code)))
            .value_quantity(Quantity::new(value, unit))
            .build();
        let restored =
            Observation::from_json(&observation.to_json().expect("json")).expect("parse");
        prop_assert_eq!(restored, observation);
    }

    #[test]
    fn encounter_round_trips_through_json(
        id in arb_text(32),
        status in arb_encounter_status(),
        class_code in arb_text(8),
        start in arb_text(10),
        end in arb_text(10),
    ) {
        let encounter = Encounter::builder()
            .id(id.clone())
            .status(status)
            .class(Coding::new(
                "http://terminology.hl7.org/CodeSystem/v3-ActCode",
                class_code,
            ))
            .period(Period::new(start, end))
            .build();
        let restored = Encounter::from_json(&encounter.to_json().expect("json")).expect("parse");
        prop_assert_eq!(restored, encounter);
    }

    #[test]
    fn organization_round_trips_through_json(id in arb_text(32), name in arb_text(48), active in any::<bool>()) {
        let org = Organization::builder()
            .id(id.clone())
            .active(active)
            .name(name)
            .build();
        let restored = Organization::from_json(&org.to_json().expect("json")).expect("parse");
        prop_assert_eq!(restored, org);
    }

    #[test]
    fn bundle_round_trips_through_json(
        patient_id in arb_text(16),
        observation_id in arb_text(16),
        org_name in arb_text(32),
    ) {
        let bundle = Bundle::collection(vec![
            Resource::Patient(Patient::builder().id(patient_id).build()),
            Resource::Observation(
                Observation::builder()
                    .id(observation_id)
                    .status(ObservationStatus::Final)
                    .code(CodeableConcept::text("vital"))
                    .build(),
            ),
            Resource::Organization(Organization::builder().name(org_name).build()),
        ]);
        let restored = Bundle::from_json(&bundle.to_json().expect("json")).expect("parse");
        prop_assert_eq!(restored, bundle);
    }
}
