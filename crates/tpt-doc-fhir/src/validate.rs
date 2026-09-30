use std::fmt;

use tpt_doc_core::ValidationReport;

use crate::encounter::{Encounter, EncounterStatus};
use crate::observation::{Observation, ObservationStatus};

/// A FHIR-specific validation error with field path reporting.
#[derive(Debug, Clone)]
pub struct FhirValidationError {
    /// JSON-pointer-style path to the field that failed validation.
    pub field_path: String,
    /// Human-readable description of the constraint that was violated.
    pub message: String,
}

impl FhirValidationError {
    /// Construct a new validation error.
    pub fn new(field_path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field_path: field_path.into(),
            message: message.into(),
        }
    }
}

impl fmt::Display for FhirValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.field_path, self.message)
    }
}

impl std::error::Error for FhirValidationError {}

/// Validate cross-field constraints of an [`Observation`] and return a
/// structured report of every violation found.
///
/// Rules checked (invariant fields, `required-if` semantics):
///
/// - `status` must be a known `ObservationStatus` (always true by
///   construction, checked for JSON-parsed values where relevant).
/// - **required-if**: a `final`, `amended`, or `corrected` observation must
///   carry a value or an effective time — a verified result with no value and
///   no timestamp cannot be acted on.
#[must_use]
pub fn validate_observation(observation: &Observation) -> ValidationReport {
    let mut report = ValidationReport::default();
    let verified = matches!(
        observation.status,
        ObservationStatus::Final | ObservationStatus::Amended | ObservationStatus::Corrected
    );
    if verified && observation.value_quantity.is_none() && observation.effective_date_time.is_none()
    {
        report.push(
            "/valueQuantity",
            format!(
                "a `{}` observation must have a value or an effective time",
                observation.status.code()
            ),
        );
    }
    report
}

/// Validate cross-field constraints of an [`Encounter`].
///
/// Rules checked:
///
/// - **date range**: `period.end`, when present, must not be before
///   `period.start` (compared as ISO `YYYY-MM-DD` dates; other formats are
///   out of scope for this check).
/// - **required-if**: a `finished` encounter must have a period end.
#[must_use]
pub fn validate_encounter(encounter: &Encounter) -> ValidationReport {
    let mut report = ValidationReport::default();
    if let Some(period) = &encounter.period {
        if let (Some(start), Some(end)) = (&period.start, &period.end) {
            if let (Ok(start_date), Ok(end_date)) = (
                time::Date::parse(
                    start,
                    &time::format_description::well_known::Iso8601::DEFAULT,
                ),
                time::Date::parse(end, &time::format_description::well_known::Iso8601::DEFAULT),
            ) && end_date < start_date
            {
                report.push(
                    "/period/end",
                    format!("period end `{end}` is before period start `{start}`"),
                );
            }
        }
    }
    if encounter.status == EncounterStatus::Finished
        && encounter.period.as_ref().is_none_or(|p| p.end.is_none())
    {
        report.push(
            "/period/end",
            "a `finished` encounter must have a period end",
        );
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{CodeableConcept, Coding, Period};

    #[test]
    fn verified_observation_requires_value_or_time() {
        let valid = Observation::builder()
            .status(ObservationStatus::Final)
            .code(CodeableConcept::text("weight"))
            .effective_date_time("2026-01-15")
            .build();
        assert!(validate_observation(&valid).is_valid());

        let preliminary = Observation::builder()
            .status(ObservationStatus::Preliminary)
            .code(CodeableConcept::text("weight"))
            .build();
        assert!(validate_observation(&preliminary).is_valid());

        let invalid = Observation::builder()
            .status(ObservationStatus::Final)
            .code(CodeableConcept::text("weight"))
            .build();
        let report = validate_observation(&invalid);
        assert!(!report.is_valid());
        assert_eq!(report.issues()[0].field_path, "/valueQuantity");
    }

    #[test]
    fn encounter_period_end_must_not_precede_start() {
        let valid = Encounter::builder()
            .status(EncounterStatus::Finished)
            .class(Coding::new(
                "http://terminology.hl7.org/CodeSystem/v3-ActCode",
                "AMB",
            ))
            .period(Period::new("2026-01-15", "2026-01-16"))
            .build();
        assert!(validate_encounter(&valid).is_valid());

        let reversed = Encounter::builder()
            .status(EncounterStatus::Finished)
            .class(Coding::new(
                "http://terminology.hl7.org/CodeSystem/v3-ActCode",
                "AMB",
            ))
            .period(Period::new("2026-01-16", "2026-01-15"))
            .build();
        let report = validate_encounter(&reversed);
        assert!(!report.is_valid());
        assert_eq!(report.issues()[0].field_path, "/period/end");
    }

    #[test]
    fn finished_encounter_requires_period_end() {
        let open = Encounter::builder()
            .status(EncounterStatus::Finished)
            .class(Coding::new(
                "http://terminology.hl7.org/CodeSystem/v3-ActCode",
                "AMB",
            ))
            .period(Period::new("2026-01-15", "2026-01-15"))
            .build();
        assert!(validate_encounter(&open).is_valid());

        let no_period = Encounter::builder()
            .status(EncounterStatus::Finished)
            .class(Coding::new(
                "http://terminology.hl7.org/CodeSystem/v3-ActCode",
                "AMB",
            ))
            .build();
        assert!(!validate_encounter(&no_period).is_valid());

        let ongoing = Encounter::builder()
            .status(EncounterStatus::InProgress)
            .class(Coding::new(
                "http://terminology.hl7.org/CodeSystem/v3-ActCode",
                "AMB",
            ))
            .build();
        assert!(validate_encounter(&ongoing).is_valid());
    }
}
