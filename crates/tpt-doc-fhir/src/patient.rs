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
        serde_json::from_slice(bytes)
            .map_err(|e| FhirValidationError::new("/", format!("JSON deserialization failed: {e}")))
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
        crate::xml::value_element(&mut xml, "id", &self.id);
        for identifier in &self.identifier {
            crate::xml::open_element(&mut xml, "identifier");
            if let Some(system) = &identifier.system {
                crate::xml::value_element(&mut xml, "system", system);
            }
            crate::xml::value_element(&mut xml, "value", &identifier.value);
            crate::xml::close_element(&mut xml, "identifier");
        }
        for name in &self.name {
            crate::xml::open_element(&mut xml, "name");
            if let Some(family) = &name.family {
                crate::xml::value_element(&mut xml, "family", family);
            }
            for given in &name.given {
                crate::xml::value_element(&mut xml, "given", given);
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
            crate::xml::value_element(&mut xml, "gender", code);
        }
        if let Some(birth_date) = &self.birth_date {
            crate::xml::value_element(&mut xml, "birthDate", birth_date);
        }
        crate::xml::close_element(&mut xml, "Patient");
        Ok(xml.into_bytes())
    }

    /// Deserialize from FHIR-compliant XML bytes.
    ///
    /// # Errors
    /// Returns an error if the bytes are not a valid FHIR Patient XML document.
    pub fn from_xml(bytes: &[u8]) -> Result<Self, FhirValidationError> {
        use quick_xml::events::Event;

        let mut reader = quick_xml::Reader::from_reader(bytes);
        let mut id = String::new();
        let mut names: Vec<HumanName> = Vec::new();
        let mut identifiers: Vec<Identifier> = Vec::new();
        let mut gender = None;
        let mut birth_date = None;
        let mut current_name = HumanName {
            family: None,
            given: Vec::new(),
        };
        let mut current_identifier = Identifier {
            system: None,
            value: String::new(),
        };
        let mut in_identifier = false;
        let err = |msg: String| FhirValidationError::new("/f:Patient", msg);

        loop {
            match reader.read_event() {
                Ok(Event::Start(start)) => {
                    if crate::xml::is_element(&start, "name") {
                        current_name = HumanName {
                            family: None,
                            given: Vec::new(),
                        };
                    } else if crate::xml::is_element(&start, "identifier") {
                        current_identifier = Identifier {
                            system: None,
                            value: String::new(),
                        };
                        in_identifier = true;
                    }
                }
                Ok(Event::Empty(start)) => {
                    let value = || crate::xml::attr_value(&start, "value").unwrap_or_default();
                    match start.local_name().as_ref() {
                        "id" => id = value(),
                        "family" => current_name.family = Some(value()),
                        "given" => current_name.given.push(value()),
                        "system" => current_identifier.system = Some(value()),
                        "value" if in_identifier => current_identifier.value = value(),
                        "gender" => {
                            let code = value();
                            gender = Some(match code.as_str() {
                                "male" => Gender::Male,
                                "female" => Gender::Female,
                                "other" => Gender::Other,
                                "unknown" => Gender::Unknown,
                                other => return Err(err(format!("unknown gender `{other}`"))),
                            });
                        }
                        "birthDate" => birth_date = Some(value()),
                        _ => {}
                    }
                }
                Ok(Event::End(end)) => match end.local_name().as_ref() {
                    "name" => names.push(std::mem::replace(
                        &mut current_name,
                        HumanName {
                            family: None,
                            given: Vec::new(),
                        },
                    )),
                    "identifier" => {
                        in_identifier = false;
                        identifiers.push(std::mem::replace(
                            &mut current_identifier,
                            Identifier {
                                system: None,
                                value: String::new(),
                            },
                        ));
                    }
                    _ => {}
                },
                Ok(Event::Eof) => break,
                Ok(_) => {}
                Err(e) => return Err(err(format!("XML error: {e}"))),
            }
        }

        Ok(Patient {
            resource_type: "Patient".into(),
            id,
            name: names,
            identifier: identifiers,
            gender,
            birth_date,
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
