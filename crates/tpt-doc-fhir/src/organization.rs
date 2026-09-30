use serde::{Deserialize, Serialize};

use crate::FhirValidationError;

/// HL7 FHIR R5 Organization resource: a formal or informal group with a
/// collective purpose (hospital, lab, department).
///
/// Built via [`Organization::builder()`]; FHIR makes `name` mandatory, so
/// `build()` is only available once it has been provided.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Organization {
    /// Resource type discriminant — always `"Organization"`.
    #[serde(rename = "resourceType")]
    resource_type: String,
    /// Logical id of the resource.
    pub id: String,
    /// Whether the organization is still operational.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active: Option<bool>,
    /// Legal name of the organization (mandatory).
    pub name: String,
}

impl Organization {
    /// Start building a new `Organization`.
    #[must_use]
    pub fn builder() -> OrganizationBuilder<NoName> {
        OrganizationBuilder {
            id: String::new(),
            active: None,
            name: NoName,
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
    /// Returns an error if the bytes are not valid FHIR Organization JSON.
    pub fn from_json(bytes: &[u8]) -> Result<Self, FhirValidationError> {
        serde_json::from_slice(bytes)
            .map_err(|e| FhirValidationError::new("/", format!("JSON deserialization failed: {e}")))
    }
}

// --- Type-state builder ---

/// Type-state marker: `name` not yet provided.
pub struct NoName;
/// Type-state marker: `name` has been provided.
pub struct HasName(String);

/// Builder for [`Organization`] that enforces the mandatory `name`.
pub struct OrganizationBuilder<NameState> {
    id: String,
    active: Option<bool>,
    name: NameState,
}

impl<NameState> OrganizationBuilder<NameState> {
    /// Provide the optional resource `id`.
    #[must_use]
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Set whether the organization is active.
    #[must_use]
    pub fn active(mut self, active: bool) -> Self {
        self.active = Some(active);
        self
    }
}

impl OrganizationBuilder<NoName> {
    /// Provide the mandatory legal name.
    #[must_use]
    pub fn name(self, name: impl Into<String>) -> OrganizationBuilder<HasName> {
        OrganizationBuilder {
            id: self.id,
            active: self.active,
            name: HasName(name.into()),
        }
    }
}

impl OrganizationBuilder<HasName> {
    /// Finalize and construct the [`Organization`].
    ///
    /// Only callable once `name` has been supplied (enforced at compile time).
    #[must_use]
    pub fn build(self) -> Organization {
        Organization {
            resource_type: "Organization".into(),
            id: self.id,
            active: self.active,
            name: self.name.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_requires_name() {
        let org = Organization::builder()
            .id("org-1")
            .active(true)
            .name("ACME Health Board")
            .build();
        assert_eq!(org.name, "ACME Health Board");
        assert_eq!(org.active, Some(true));
    }

    #[test]
    fn json_round_trip() {
        let org = Organization::builder()
            .id("org-2")
            .name("Wellington Labs")
            .build();
        let restored = Organization::from_json(&org.to_json().expect("json")).expect("parse");
        assert_eq!(org, restored);
    }
}
