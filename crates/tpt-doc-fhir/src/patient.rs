use serde::{Deserialize, Serialize};

use crate::{FhirValidationError, HumanName, Identifier};

/// HL7 FHIR R5 Patient resource.
///
/// Built via [`Patient::builder()`] which enforces mandatory fields at compile
/// time through the type-state pattern.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Patient {
    /// Resource type discriminant — always `"Patient"`.
    #[serde(rename = "resourceType")]
    resource_type: String,
    /// Logical id of the resource.
    pub id: String,
    /// A name associated with the patient.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub name: Vec<HumanName>,
    /// An identifier for this patient.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub identifier: Vec<Identifier>,
    /// Administrative gender.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gender: Option<Gender>,
    /// Date of birth (ISO 8601 partial date: YYYY, YYYY-MM, or YYYY-MM-DD).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub birth_date: Option<String>,
}

/// Administrative gender codes (FHIR R5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Gender {
    /// Male.
    Male,
    /// Female.
    Female,
    /// Other.
    Other,
    /// Unknown.
    Unknown,
}

impl Patient {
    /// Start building a new `Patient`.
    pub fn builder() -> PatientBuilder<NoId> {
        PatientBuilder {
            id: NoId,
            name: Vec::new(),
            identifier: Vec::new(),
            gender: None,
            birth_date: None,
        }
    }

    /// Serialize to FHIR-compliant JSON bytes.
    ///
    /// # Errors
    /// Returns an error if JSON serialization fails.
    pub fn to_json(&self) -> Result<Vec<u8>, FhirValidationError> {
        serde_json::to_vec_pretty(self).map_err(|e| {
            FhirValidationError::new("/", format!("JSON serialization failed: {e}"))
        })
    }

    /// Deserialize from FHIR-compliant JSON bytes.
    ///
    /// # Errors
    /// Returns an error if the bytes are not valid FHIR Patient JSON.
    pub fn from_json(bytes: &[u8]) -> Result<Self, FhirValidationError> {
        serde_json::from_slice(bytes).map_err(|e| {
            FhirValidationError::new("/", format!("JSON deserialization failed: {e}"))
        })
    }
}

// --- Type-state builder ---

/// Type-state marker: `id` not yet provided.
pub struct NoId;
/// Type-state marker: `id` has been provided.
pub struct HasId(String);

/// Builder for [`Patient`] that enforces mandatory fields at compile time.
pub struct PatientBuilder<IdState> {
    id: IdState,
    name: Vec<HumanName>,
    identifier: Vec<Identifier>,
    gender: Option<Gender>,
    birth_date: Option<String>,
}

impl<IdState> PatientBuilder<IdState> {
    /// Add a name.
    pub fn name(mut self, name: HumanName) -> Self {
        self.name.push(name);
        self
    }

    /// Add an identifier.
    pub fn identifier(mut self, id: Identifier) -> Self {
        self.identifier.push(id);
        self
    }

    /// Set the administrative gender.
    pub fn gender(mut self, gender: Gender) -> Self {
        self.gender = Some(gender);
        self
    }

    /// Set the date of birth (ISO 8601 partial date).
    pub fn birth_date(mut self, date: impl Into<String>) -> Self {
        self.birth_date = Some(date.into());
        self
    }
}

impl PatientBuilder<NoId> {
    /// Provide the mandatory resource `id`.
    pub fn id(self, id: impl Into<String>) -> PatientBuilder<HasId> {
        PatientBuilder {
            id: HasId(id.into()),
            name: self.name,
            identifier: self.identifier,
            gender: self.gender,
            birth_date: self.birth_date,
        }
    }
}

impl PatientBuilder<HasId> {
    /// Finalize and construct the [`Patient`].
    ///
    /// Only callable once `id` has been supplied (enforced at compile time).
    pub fn build(self) -> Patient {
        Patient {
            resource_type: "Patient".into(),
            id: self.id.0,
            name: self.name,
            identifier: self.identifier,
            gender: self.gender,
            birth_date: self.birth_date,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_requires_id() {
        // This test verifies the happy path compiles and runs correctly.
        let patient = Patient::builder()
            .id("nz-12345")
            .name(HumanName::new("Smith", "John"))
            .gender(Gender::Male)
            .build();

        assert_eq!(patient.id, "nz-12345");
        assert_eq!(patient.resource_type, "Patient");
        assert_eq!(patient.name.len(), 1);
        assert_eq!(patient.gender, Some(Gender::Male));
    }

    #[test]
    fn json_round_trip() {
        let patient = Patient::builder()
            .id("nz-99999")
            .name(HumanName::new("Doe", "Jane"))
            .build();

        let json = patient.to_json().unwrap();
        let restored = Patient::from_json(&json).unwrap();
        assert_eq!(patient, restored);
    }
}
