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
    /// Elements this model does not represent (for example `extension`,
    /// `meta`, `text`), kept as read so that a JSON round trip loses nothing.
    #[serde(flatten, default, skip_serializing_if = "serde_json::Map::is_empty")]
    pub unmodelled: serde_json::Map<String, serde_json::Value>,
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
        let resource: Self = serde_json::from_slice(bytes).map_err(|e| {
            FhirValidationError::new("/", format!("JSON deserialization failed: {e}"))
        })?;
        crate::types::check_resource_type(&resource.resource_type, "Organization")?;
        Ok(resource)
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
            unmodelled: serde_json::Map::new(),
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

impl Organization {
    /// Serialize to FHIR XML bytes.
    ///
    /// # Errors
    /// Returns an error if a value contains a character XML cannot represent.
    pub fn to_xml(&self) -> Result<Vec<u8>, FhirValidationError> {
        use std::fmt::Write as _;

        let mut xml = String::from(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
        let _ = write!(xml, r#"<Organization xmlns="{}">"#, crate::xml::FHIR_NS);
        crate::xml::value_element(&mut xml, "id", &self.id)?;
        if let Some(active) = self.active {
            let text = if active { "true" } else { "false" };
            crate::xml::value_element(&mut xml, "active", text)?;
        }
        crate::xml::value_element(&mut xml, "name", &self.name)?;
        crate::xml::close_element(&mut xml, "Organization");
        Ok(xml.into_bytes())
    }

    /// Deserialize from FHIR XML bytes.
    ///
    /// # Errors
    /// Returns an error if the bytes are not a valid FHIR Organization
    /// document, `active` is not a boolean, or `name` is missing.
    pub fn from_xml(bytes: &[u8]) -> Result<Self, FhirValidationError> {
        Self::from_xml_with_limits(bytes, tpt_doc_core::Limits::default())
    }

    /// Deserialize from FHIR XML bytes with explicit limits applied.
    ///
    /// # Errors
    /// As [`Organization::from_xml`], and if the document nests deeper than
    /// `limits.max_xml_depth`.
    pub fn from_xml_with_limits(
        bytes: &[u8],
        limits: tpt_doc_core::Limits,
    ) -> Result<Self, FhirValidationError> {
        let mut draft = OrganizationDraft::default();
        crate::xml::read_resource(bytes, &limits, "Organization", &mut draft)?;
        draft.finish()
    }
}

/// Values collected while reading an `Organization`.
#[derive(Default)]
struct OrganizationDraft {
    id: String,
    active: Option<bool>,
    name: Option<String>,
}

impl crate::xml::DraftReader for OrganizationDraft {
    fn open(
        &mut self,
        path: &[String],
        start: &quick_xml::events::BytesStart<'_>,
    ) -> Result<(), FhirValidationError> {
        use crate::xml::path_refs;

        let value = || crate::xml::attr_value(start, "value").unwrap_or_default();
        match path_refs(path).as_slice() {
            ["Organization", "id"] => self.id = value(),
            ["Organization", "active"] => {
                self.active = Some(match value().as_str() {
                    "true" => true,
                    "false" => false,
                    other => {
                        return Err(FhirValidationError::new(
                            "/f:Organization",
                            format!("`active` must be true or false, found `{other}`"),
                        ));
                    }
                });
            }
            ["Organization", "name"] => self.name = Some(value()),
            _ => {}
        }
        Ok(())
    }

    fn close(&mut self, _path: &[String]) {}
}

impl OrganizationDraft {
    fn finish(self) -> Result<Organization, FhirValidationError> {
        let name = self.name.ok_or_else(|| {
            FhirValidationError::new(
                "/f:Organization",
                "missing mandatory element `name`".to_owned(),
            )
        })?;
        Ok(Organization {
            resource_type: "Organization".into(),
            unmodelled: serde_json::Map::new(),
            id: self.id,
            active: self.active,
            name,
        })
    }
}
