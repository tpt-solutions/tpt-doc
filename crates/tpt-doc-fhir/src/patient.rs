use serde::{Deserialize, Serialize};
use tpt_doc_core::Limits;

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
    /// Elements this model does not represent (for example `extension`,
    /// `meta`, `text`), kept as read so that a JSON round trip loses nothing.
    #[serde(flatten, default, skip_serializing_if = "serde_json::Map::is_empty")]
    pub unmodelled: serde_json::Map<String, serde_json::Value>,
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
    #[must_use]
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
        serde_json::to_vec_pretty(self)
            .map_err(|e| FhirValidationError::new("/", format!("JSON serialization failed: {e}")))
    }

    /// Deserialize from FHIR-compliant JSON bytes.
    ///
    /// # Errors
    /// Returns an error if the bytes are not valid FHIR Patient JSON.
    pub fn from_json(bytes: &[u8]) -> Result<Self, FhirValidationError> {
        let resource: Self = serde_json::from_slice(bytes).map_err(|e| {
            FhirValidationError::new("/", format!("JSON deserialization failed: {e}"))
        })?;
        crate::types::check_resource_type(&resource.resource_type, "Patient")?;
        resource.check_dates()?;
        Ok(resource)
    }

    /// Serialize to FHIR-compliant XML bytes.
    ///
    /// # Errors
    /// Returns an error if XML serialization fails.
    pub fn to_xml(&self) -> Result<Vec<u8>, FhirValidationError> {
        let mut xml = String::from(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
        let _ = std::fmt::Write::write_fmt(
            &mut xml,
            format_args!(r#"<Patient xmlns="{}">"#, crate::xml::FHIR_NS),
        );
        crate::xml::value_element(&mut xml, "id", &self.id)?;
        for identifier in &self.identifier {
            crate::xml::open_element(&mut xml, "identifier");
            if let Some(system) = &identifier.system {
                crate::xml::value_element(&mut xml, "system", system)?;
            }
            crate::xml::value_element(&mut xml, "value", &identifier.value)?;
            crate::xml::close_element(&mut xml, "identifier");
        }
        for name in &self.name {
            crate::xml::open_element(&mut xml, "name");
            if let Some(family) = &name.family {
                crate::xml::value_element(&mut xml, "family", family)?;
            }
            for given in &name.given {
                crate::xml::value_element(&mut xml, "given", given)?;
            }
            crate::xml::close_element(&mut xml, "name");
        }
        if let Some(gender) = &self.gender {
            let code = match gender {
                Gender::Male => "male",
                Gender::Female => "female",
                Gender::Other => "other",
                Gender::Unknown => "unknown",
            };
            crate::xml::value_element(&mut xml, "gender", code)?;
        }
        if let Some(birth_date) = &self.birth_date {
            crate::xml::value_element(&mut xml, "birthDate", birth_date)?;
        }
        crate::xml::close_element(&mut xml, "Patient");
        Ok(xml.into_bytes())
    }

    /// Deserialize from FHIR-compliant XML bytes.
    ///
    /// # Errors
    /// Returns an error if the bytes are not a valid FHIR Patient XML document.
    pub fn from_xml(bytes: &[u8]) -> Result<Self, FhirValidationError> {
        Self::from_xml_with_limits(bytes, Limits::default())
    }

    /// Deserialize from FHIR XML bytes with explicit [`Limits`] applied.
    ///
    /// Nesting is bounded by `limits.max_xml_depth`: a `Patient` resource is a
    /// handful of levels deep, so deeper nesting is corrupt or hostile.
    ///
    /// # Errors
    /// Returns an error if the bytes are not a valid FHIR Patient XML document
    /// or exceed `limits.max_xml_depth`.
    pub fn from_xml_with_limits(bytes: &[u8], limits: Limits) -> Result<Self, FhirValidationError> {
        let mut draft = PatientDraft::default();
        crate::xml::read_resource(bytes, &limits, "Patient", &mut draft)?;
        let patient = draft.finish();
        patient.check_dates()?;
        Ok(patient)
    }
}

/// Values collected while reading a `Patient`, filled from the elements that
/// define them at their documented position in the resource.
#[derive(Default)]
struct PatientDraft {
    id: String,
    names: Vec<HumanName>,
    identifiers: Vec<Identifier>,
    gender: Option<Gender>,
    birth_date: Option<String>,
    current_name: Option<HumanName>,
    current_identifier: Option<Identifier>,
}

impl PatientDraft {
    /// Apply an element's attributes at its position in the resource.
    fn apply(
        &mut self,
        path: &[String],
        start: &quick_xml::events::BytesStart<'_>,
        err: &impl Fn(String) -> FhirValidationError,
    ) -> Result<(), FhirValidationError> {
        let value = || crate::xml::attr_value(start, "value").unwrap_or_default();
        match path_refs(path).as_slice() {
            ["Patient", "id"] => self.id = value(),
            ["Patient", "name"] => {
                self.current_name = Some(HumanName {
                    family: None,
                    given: Vec::new(),
                });
            }
            ["Patient", "name", "family"] => {
                if let Some(name) = &mut self.current_name {
                    name.family = Some(value());
                }
            }
            ["Patient", "name", "given"] => {
                if let Some(name) = &mut self.current_name {
                    name.given.push(value());
                }
            }
            ["Patient", "identifier"] => {
                self.current_identifier = Some(Identifier {
                    system: None,
                    value: String::new(),
                });
            }
            ["Patient", "identifier", "system"] => {
                if let Some(identifier) = &mut self.current_identifier {
                    identifier.system = Some(value());
                }
            }
            ["Patient", "identifier", "value"] => {
                if let Some(identifier) = &mut self.current_identifier {
                    identifier.value = value();
                }
            }
            ["Patient", "gender"] => {
                let code = value();
                self.gender = Some(match code.as_str() {
                    "male" => Gender::Male,
                    "female" => Gender::Female,
                    "other" => Gender::Other,
                    "unknown" => Gender::Unknown,
                    other => return Err(err(format!("unknown gender `{other}`"))),
                });
            }
            ["Patient", "birthDate"] => self.birth_date = Some(value()),
            _ => {}
        }
        Ok(())
    }

    /// Commit a repeating element when its end is reached.
    fn end(&mut self, path: &[String]) {
        match path_refs(path).as_slice() {
            ["Patient", "name"] => self.names.extend(self.current_name.take()),
            ["Patient", "identifier"] => {
                self.identifiers.extend(self.current_identifier.take());
            }
            _ => {}
        }
    }

    fn finish(self) -> Patient {
        Patient {
            resource_type: "Patient".into(),
            unmodelled: serde_json::Map::new(),
            id: self.id,
            name: self.names,
            identifier: self.identifiers,
            gender: self.gender,
            birth_date: self.birth_date,
        }
    }
}

/// Borrow a path of owned element names as `&str`s for matching.
fn path_refs(path: &[String]) -> Vec<&str> {
    path.iter().map(String::as_str).collect()
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
    #[must_use]
    pub fn name(mut self, name: HumanName) -> Self {
        self.name.push(name);
        self
    }

    /// Add an identifier.
    #[must_use]
    pub fn identifier(mut self, id: Identifier) -> Self {
        self.identifier.push(id);
        self
    }

    /// Set the administrative gender.
    #[must_use]
    pub fn gender(mut self, gender: Gender) -> Self {
        self.gender = Some(gender);
        self
    }

    /// Set the date of birth (ISO 8601 partial date).
    #[must_use]
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
    #[must_use]
    pub fn build(self) -> Patient {
        Patient {
            resource_type: "Patient".into(),
            unmodelled: serde_json::Map::new(),
            id: self.id.0,
            name: self.name,
            identifier: self.identifier,
            gender: self.gender,
            birth_date: self.birth_date,
        }
    }
}

impl crate::xml::DraftReader for PatientDraft {
    fn open(
        &mut self,
        path: &[String],
        start: &quick_xml::events::BytesStart<'_>,
    ) -> Result<(), FhirValidationError> {
        let err = |msg: String| FhirValidationError::new("/f:Patient", msg);
        self.apply(path, start, &err)
    }

    fn close(&mut self, path: &[String]) {
        self.end(path);
    }
}

impl Patient {
    /// Check the date fields: `birthDate` is a FHIR `date`.
    ///
    /// # Errors
    /// Returns an error naming the field if a date is malformed.
    pub(crate) fn check_dates(&self) -> Result<(), FhirValidationError> {
        match &self.birth_date {
            Some(date) => crate::types::check_date("/birthDate", date),
            None => Ok(()),
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
