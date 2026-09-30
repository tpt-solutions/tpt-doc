use serde::{Deserialize, Serialize};

use crate::FhirValidationError;
use crate::types::{Coding, Period, Reference};

/// HL7 FHIR R5 Encounter resource: an interaction between a patient and
/// healthcare provider(s).
///
/// FHIR makes `status` and `class` mandatory; the type-state builder only
/// offers `build()` once both have been supplied, in either order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Encounter {
    /// Resource type discriminant — always `"Encounter"`.
    #[serde(rename = "resourceType")]
    resource_type: String,
    /// Logical id of the resource.
    pub id: String,
    /// Lifecycle status of the encounter (mandatory).
    pub status: EncounterStatus,
    /// Classification of the encounter (mandatory, e.g. inpatient).
    #[serde(rename = "class")]
    pub class: Coding,
    /// The patient present at the encounter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<Reference>,
    /// Start and end time of the encounter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub period: Option<Period>,
}

/// Lifecycle status codes for an [`Encounter`] (FHIR R5 `EncounterStatus`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EncounterStatus {
    /// The encounter has been planned.
    Planned,
    /// The patient is present.
    InProgress,
    /// The encounter has ended.
    Finished,
    /// The patient left before being seen.
    Cancelled,
    /// The encounter entered in error.
    EnteredInError,
    /// The status is unknown.
    Unknown,
}

impl EncounterStatus {
    /// The wire-format code.
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::Planned => "planned",
            Self::InProgress => "in-progress",
            Self::Finished => "finished",
            Self::Cancelled => "cancelled",
            Self::EnteredInError => "entered-in-error",
            Self::Unknown => "unknown",
        }
    }

    /// Parse a wire-format status code.
    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "planned" => Some(Self::Planned),
            "in-progress" => Some(Self::InProgress),
            "finished" => Some(Self::Finished),
            "cancelled" => Some(Self::Cancelled),
            "entered-in-error" => Some(Self::EnteredInError),
            "unknown" => Some(Self::Unknown),
            _ => None,
        }
    }
}

impl Encounter {
    /// Start building a new `Encounter`.
    #[must_use]
    pub fn builder() -> EncounterBuilder<Unset, Unset> {
        EncounterBuilder {
            id: String::new(),
            status: Unset,
            class: Unset,
            subject: None,
            period: None,
        }
    }

    /// Serialize to FHIR-compliant JSON bytes.
    ///
    /// # Errors
    /// Returns an error if JSON serialization fails.
    pub fn to_json(&self) -> Result<Vec<u8>, FhirValidationError> {
        serde_json::to_vec_pretty(self)
            .map_err(|e| FhirValidationError::new("/", format!("JSON serialization failed: {e}")))
    }

    /// Deserialize from FHIR-compliant JSON bytes.
    ///
    /// # Errors
    /// Returns an error if the bytes are not valid FHIR Encounter JSON.
    pub fn from_json(bytes: &[u8]) -> Result<Self, FhirValidationError> {
        serde_json::from_slice(bytes)
            .map_err(|e| FhirValidationError::new("/", format!("JSON deserialization failed: {e}")))
    }
}

/// Type-state marker: mandatory field not yet provided.
pub struct Unset;
/// Type-state marker: the field has been provided.
pub struct Set<T>(T);

/// Builder for [`Encounter`] enforcing `status` + `class` at compile time.
pub struct EncounterBuilder<StatusState, ClassState> {
    id: String,
    status: StatusState,
    class: ClassState,
    subject: Option<Reference>,
    period: Option<Period>,
}

impl<A, B> EncounterBuilder<A, B> {
    /// Provide the optional resource `id`.
    #[must_use]
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Set the patient present at the encounter.
    #[must_use]
    pub fn subject(mut self, subject: Reference) -> Self {
        self.subject = Some(subject);
        self
    }

    /// Set the encounter period.
    #[must_use]
    pub fn period(mut self, period: Period) -> Self {
        self.period = Some(period);
        self
    }
}

impl EncounterBuilder<Unset, Unset> {
    /// Provide the mandatory status first.
    #[must_use]
    pub fn status(self, status: EncounterStatus) -> EncounterBuilder<Set<EncounterStatus>, Unset> {
        EncounterBuilder {
            id: self.id,
            status: Set(status),
            class: self.class,
            subject: self.subject,
            period: self.period,
        }
    }

    /// Provide the mandatory class first.
    #[must_use]
    pub fn class(self, class: Coding) -> EncounterBuilder<Unset, Set<Coding>> {
        EncounterBuilder {
            id: self.id,
            status: self.status,
            class: Set(class),
            subject: self.subject,
            period: self.period,
        }
    }
}

impl EncounterBuilder<Set<EncounterStatus>, Unset> {
    /// Provide the mandatory class, completing both required fields.
    #[must_use]
    pub fn class(self, class: Coding) -> EncounterBuilder<Set<EncounterStatus>, Set<Coding>> {
        EncounterBuilder {
            id: self.id,
            status: self.status,
            class: Set(class),
            subject: self.subject,
            period: self.period,
        }
    }
}

impl EncounterBuilder<Unset, Set<Coding>> {
    /// Provide the mandatory status, completing both required fields.
    #[must_use]
    pub fn status(
        self,
        status: EncounterStatus,
    ) -> EncounterBuilder<Set<EncounterStatus>, Set<Coding>> {
        EncounterBuilder {
            id: self.id,
            status: Set(status),
            class: self.class,
            subject: self.subject,
            period: self.period,
        }
    }
}

impl EncounterBuilder<Set<EncounterStatus>, Set<Coding>> {
    /// Finalize and construct the [`Encounter`].
    ///
    /// Only callable once `status` and `class` have both been supplied.
    #[must_use]
    pub fn build(self) -> Encounter {
        Encounter {
            resource_type: "Encounter".into(),
            id: self.id,
            status: self.status.0,
            class: self.class.0,
            subject: self.subject,
            period: self.period,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Encounter {
        Encounter::builder()
            .id("enc-1")
            .status(EncounterStatus::Finished)
            .class(Coding::new(
                "http://terminology.hl7.org/CodeSystem/v3-ActCode",
                "AMB",
            ))
            .subject(Reference::to("Patient/nz-12345"))
            .period(Period::new("2026-01-15", "2026-01-16"))
            .build()
    }

    #[test]
    fn builder_accepts_mandatory_fields_in_either_order() {
        let class = Coding::new("http://terminology.hl7.org/CodeSystem/v3-ActCode", "IMP");
        let a = Encounter::builder()
            .status(EncounterStatus::InProgress)
            .class(class.clone())
            .build();
        let b = Encounter::builder()
            .class(class)
            .status(EncounterStatus::InProgress)
            .build();
        assert_eq!(a, b);
    }

    #[test]
    fn json_round_trip() {
        let encounter = sample();
        let restored = Encounter::from_json(&encounter.to_json().expect("json")).expect("parse");
        assert_eq!(encounter, restored);
    }

    #[test]
    fn class_serializes_with_plain_key() {
        let json = String::from_utf8(sample().to_json().expect("json")).expect("UTF-8");
        assert!(json.contains(r#""class""#));
        assert!(!json.contains("r#class"));
    }

    #[test]
    fn status_codes_round_trip() {
        for code in [
            "planned",
            "in-progress",
            "finished",
            "cancelled",
            "entered-in-error",
            "unknown",
        ] {
            assert_eq!(
                EncounterStatus::from_code(code).map(|s| s.code()),
                Some(code)
            );
        }
        assert_eq!(EncounterStatus::from_code("bogus"), None);
    }
}
