use serde::{Deserialize, Serialize};
use tpt_doc_core::Limits;

use crate::FhirValidationError;
use crate::types::{CodeableConcept, Quantity, Reference};
use crate::xml;

/// HL7 FHIR R5 Observation resource: measurements, lab results, vitals.
///
/// Built via [`Observation::builder()`]. FHIR makes both `status` and `code`
/// mandatory, so the type-state builder only offers `build()` once both have
/// been supplied — in either order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Observation {
    /// Resource type discriminant — always `"Observation"`.
    #[serde(rename = "resourceType")]
    resource_type: String,
    /// Elements this model does not represent (for example `extension`,
    /// `meta`, `text`), kept as read so that a JSON round trip loses nothing.
    #[serde(flatten, default, skip_serializing_if = "serde_json::Map::is_empty")]
    pub unmodelled: serde_json::Map<String, serde_json::Value>,
    /// Logical id of the resource.
    pub id: String,
    /// Lifecycle status of the result (mandatory).
    pub status: ObservationStatus,
    /// What was measured (mandatory, e.g. a LOINC code).
    pub code: CodeableConcept,
    /// The subject the observation is about.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<Reference>,
    /// The measured or stated value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_quantity: Option<Quantity>,
    /// When the measurement was taken.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effective_date_time: Option<String>,
}

/// Lifecycle status codes for an [`Observation`] (FHIR R5 `ObservationStatus`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ObservationStatus {
    /// Registered, not yet verified.
    Registered,
    /// Preliminary result.
    Preliminary,
    /// Verified final result.
    Final,
    /// Subsequent to a final result, amended.
    Amended,
    /// Subsequent to a final result, corrected.
    Corrected,
    /// The observation was cancelled.
    Cancelled,
    /// Entered in error.
    EnteredInError,
    /// The status is unknown.
    Unknown,
}

impl ObservationStatus {
    /// The wire-format code.
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::Registered => "registered",
            Self::Preliminary => "preliminary",
            Self::Final => "final",
            Self::Amended => "amended",
            Self::Corrected => "corrected",
            Self::Cancelled => "cancelled",
            Self::EnteredInError => "entered-in-error",
            Self::Unknown => "unknown",
        }
    }

    /// Parse a wire-format status code.
    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "registered" => Some(Self::Registered),
            "preliminary" => Some(Self::Preliminary),
            "final" => Some(Self::Final),
            "amended" => Some(Self::Amended),
            "corrected" => Some(Self::Corrected),
            "cancelled" => Some(Self::Cancelled),
            "entered-in-error" => Some(Self::EnteredInError),
            "unknown" => Some(Self::Unknown),
            _ => None,
        }
    }
}

impl Observation {
    /// Start building a new `Observation`.
    #[must_use]
    pub fn builder() -> ObservationBuilder<Unset, Unset> {
        ObservationBuilder {
            id: String::new(),
            status: Unset,
            code: Unset,
            subject: None,
            value_quantity: None,
            effective_date_time: None,
        }
    }

    /// The observation status as its wire-format code (e.g. `"final"`).
    #[must_use]
    pub fn status_code_text(&self) -> &'static str {
        self.status.code()
    }

    /// The measured value formatted as `"72.5 kg"`, when present.
    #[must_use]
    pub fn value_text(&self) -> Option<String> {
        self.value_quantity.as_ref().map(|q| {
            let number = q.value_text.clone().unwrap_or_else(|| q.value.to_string());
            match &q.unit {
                Some(unit) => format!("{number} {unit}"),
                None => number,
            }
        })
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
    /// Returns an error if the bytes are not valid FHIR Observation JSON.
    pub fn from_json(bytes: &[u8]) -> Result<Self, FhirValidationError> {
        let resource: Self = serde_json::from_slice(bytes).map_err(|e| {
            FhirValidationError::new("/", format!("JSON deserialization failed: {e}"))
        })?;
        crate::types::check_resource_type(&resource.resource_type, "Observation")?;
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
            format_args!(r#"<Observation xmlns="{}">"#, xml::FHIR_NS),
        );
        xml::value_element(&mut xml, "id", &self.id)?;
        xml::value_element(&mut xml, "status", self.status.code())?;
        xml::open_element(&mut xml, "code");
        write_concept(&mut xml, &self.code)?;
        xml::close_element(&mut xml, "code");
        if let Some(subject) = &self.subject {
            xml::open_element(&mut xml, "subject");
            write_reference(&mut xml, subject)?;
            xml::close_element(&mut xml, "subject");
        }
        if let Some(quantity) = &self.value_quantity {
            xml::open_element(&mut xml, "valueQuantity");
            write_quantity(&mut xml, quantity)?;
            xml::close_element(&mut xml, "valueQuantity");
        }
        if let Some(effective) = &self.effective_date_time {
            xml::value_element(&mut xml, "effectiveDateTime", effective)?;
        }
        xml::close_element(&mut xml, "Observation");
        Ok(xml.into_bytes())
    }

    /// Deserialize from FHIR-compliant XML bytes.
    ///
    /// # Errors
    /// Returns an error if the bytes are not a valid FHIR Observation XML
    /// document or a mandatory element is missing.
    pub fn from_xml(bytes: &[u8]) -> Result<Self, FhirValidationError> {
        Self::from_xml_with_limits(bytes, Limits::default())
    }

    /// Deserialize from FHIR XML bytes with explicit [`Limits`] applied.
    ///
    /// Nesting is bounded by `limits.max_xml_depth`.
    ///
    /// # Errors
    /// Returns an error if the bytes are not a valid FHIR Observation XML
    /// document, a mandatory element is missing, or the document nests deeper
    /// than `limits.max_xml_depth`.
    pub fn from_xml_with_limits(bytes: &[u8], limits: Limits) -> Result<Self, FhirValidationError> {
        let mut draft = ObservationDraft::default();
        crate::xml::read_resource(bytes, &limits, "Observation", &mut draft)?;
        let observation = draft.finish()?;
        observation.check_dates()?;
        Ok(observation)
    }
}

/// Values collected while reading an `Observation`, filled from the elements
/// that define them at their documented position in the resource.
#[derive(Debug, Default)]
struct ObservationDraft {
    id: String,
    status: Option<ObservationStatus>,
    code: CodeableConcept,
    subject: Option<Reference>,
    value_quantity: Option<Quantity>,
    effective: Option<String>,
    current_coding: Option<crate::Coding>,
    current_subject: Option<Reference>,
    current_quantity: Option<Quantity>,
}

impl ObservationDraft {
    /// Apply an element's attributes at its position in the resource.
    fn apply(
        &mut self,
        path: &[String],
        start: &quick_xml::events::BytesStart<'_>,
        err: &impl Fn(String) -> FhirValidationError,
    ) -> Result<(), FhirValidationError> {
        let value = || crate::xml::attr_value(start, "value").unwrap_or_default();
        match path_refs(path).as_slice() {
            ["Observation", "id"] => self.id = value(),
            ["Observation", "status"] => {
                let code = value();
                self.status = Some(
                    ObservationStatus::from_code(&code)
                        .ok_or_else(|| err(format!("unknown status code `{code}`")))?,
                );
            }
            ["Observation", "effectiveDateTime"] => self.effective = Some(value()),
            ["Observation", "code", "text"] => self.code.text = Some(value()),
            ["Observation", "code", "coding"] => {
                self.current_coding = Some(crate::Coding {
                    system: None,
                    code: String::new(),
                    display: None,
                });
            }
            ["Observation", "code", "coding", "system"] => {
                if let Some(coding) = &mut self.current_coding {
                    coding.system = Some(value());
                }
            }
            ["Observation", "code", "coding", "code"] => {
                if let Some(coding) = &mut self.current_coding {
                    coding.code = value();
                }
            }
            ["Observation", "code", "coding", "display"] => {
                if let Some(coding) = &mut self.current_coding {
                    coding.display = Some(value());
                }
            }
            ["Observation", "subject"] => {
                self.current_subject = Some(Reference {
                    reference: None,
                    display: None,
                });
            }
            ["Observation", "subject", "reference"] => {
                if let Some(subject) = &mut self.current_subject {
                    subject.reference = Some(value());
                }
            }
            ["Observation", "subject", "display"] => {
                if let Some(subject) = &mut self.current_subject {
                    subject.display = Some(value());
                }
            }
            ["Observation", "valueQuantity"] => {
                self.current_quantity = Some(Quantity {
                    value: 0.0,
                    value_text: None,
                    unit: None,
                    system: None,
                    code: None,
                });
            }
            ["Observation", "valueQuantity", "value"] => {
                let attr = value();
                let number = attr
                    .parse::<f64>()
                    .map_err(|_| err(format!("non-numeric quantity `{attr}`")))?;
                if let Some(quantity) = &mut self.current_quantity {
                    quantity.value = number;
                    quantity.value_text = Some(attr);
                }
            }
            ["Observation", "valueQuantity", "unit"] => {
                if let Some(quantity) = &mut self.current_quantity {
                    quantity.unit = Some(value());
                }
            }
            ["Observation", "valueQuantity", "system"] => {
                if let Some(quantity) = &mut self.current_quantity {
                    quantity.system = Some(value());
                }
            }
            ["Observation", "valueQuantity", "code"] => {
                if let Some(quantity) = &mut self.current_quantity {
                    quantity.code = Some(value());
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Commit a repeating or nested element when its end is reached.
    fn end(&mut self, path: &[String]) {
        match path_refs(path).as_slice() {
            ["Observation", "code", "coding"] => {
                self.code.coding.extend(self.current_coding.take());
            }
            ["Observation", "subject"] => {
                self.subject = self.current_subject.take();
            }
            ["Observation", "valueQuantity"] => {
                self.value_quantity = self.current_quantity.take();
            }
            _ => {}
        }
    }

    fn finish(self) -> Result<Observation, FhirValidationError> {
        let status = self.status.ok_or_else(|| {
            FhirValidationError::new(
                "/f:Observation",
                "missing mandatory element `status`".to_owned(),
            )
        })?;
        Ok(Observation {
            resource_type: "Observation".into(),
            unmodelled: serde_json::Map::new(),
            id: self.id,
            status,
            code: self.code,
            subject: self.subject,
            value_quantity: self.value_quantity,
            effective_date_time: self.effective,
        })
    }
}

/// Borrow a path of owned element names as `&str`s for matching.
fn path_refs(path: &[String]) -> Vec<&str> {
    path.iter().map(String::as_str).collect()
}

/// Serialize a `CodeableConcept`'s children inside an open `<code>` element.
fn write_concept(xml: &mut String, concept: &CodeableConcept) -> Result<(), FhirValidationError> {
    for coding in &concept.coding {
        xml::open_element(xml, "coding");
        if let Some(system) = &coding.system {
            xml::value_element(xml, "system", system)?;
        }
        xml::value_element(xml, "code", &coding.code)?;
        if let Some(display) = &coding.display {
            xml::value_element(xml, "display", display)?;
        }
        xml::close_element(xml, "coding");
    }
    if let Some(text) = &concept.text {
        xml::value_element(xml, "text", text)?;
    }
    Ok(())
}

/// Serialize a `Reference`'s children inside an open `<subject>` element.
fn write_reference(xml: &mut String, reference: &Reference) -> Result<(), FhirValidationError> {
    if let Some(target) = &reference.reference {
        xml::value_element(xml, "reference", target)?;
    }
    if let Some(display) = &reference.display {
        xml::value_element(xml, "display", display)?;
    }
    Ok(())
}

/// Serialize a `Quantity`'s children inside an open `valueQuantity` element.
fn write_quantity(xml: &mut String, quantity: &Quantity) -> Result<(), FhirValidationError> {
    let number = quantity
        .value_text
        .clone()
        .unwrap_or_else(|| quantity.value.to_string());
    xml::value_element(xml, "value", &number)?;
    if let Some(unit) = &quantity.unit {
        xml::value_element(xml, "unit", unit)?;
    }
    if let Some(system) = &quantity.system {
        xml::value_element(xml, "system", system)?;
    }
    if let Some(code) = &quantity.code {
        xml::value_element(xml, "code", code)?;
    }
    Ok(())
}

// --- Type-state builder enforcing both mandatory fields ---

/// Type-state marker: mandatory field not yet provided.
pub struct Unset;
/// Type-state marker: the field has been provided.
pub struct Set<T>(T);

/// Builder for [`Observation`] enforcing `status` + `code` at compile time.
///
/// `build()` exists only on the fully-set state; the mandatory fields may be
/// provided in either order.
pub struct ObservationBuilder<StatusState, CodeState> {
    id: String,
    status: StatusState,
    code: CodeState,
    subject: Option<Reference>,
    value_quantity: Option<Quantity>,
    effective_date_time: Option<String>,
}

impl<A, B> ObservationBuilder<A, B> {
    /// Provide the optional resource `id`.
    #[must_use]
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Set the subject reference.
    #[must_use]
    pub fn subject(mut self, subject: Reference) -> Self {
        self.subject = Some(subject);
        self
    }

    /// Set the measured value.
    #[must_use]
    pub fn value_quantity(mut self, quantity: Quantity) -> Self {
        self.value_quantity = Some(quantity);
        self
    }

    /// Set when the measurement was taken.
    #[must_use]
    pub fn effective_date_time(mut self, date_time: impl Into<String>) -> Self {
        self.effective_date_time = Some(date_time.into());
        self
    }
}

impl ObservationBuilder<Unset, Unset> {
    /// Provide the mandatory status first.
    #[must_use]
    pub fn status(
        self,
        status: ObservationStatus,
    ) -> ObservationBuilder<Set<ObservationStatus>, Unset> {
        ObservationBuilder {
            id: self.id,
            status: Set(status),
            code: self.code,
            subject: self.subject,
            value_quantity: self.value_quantity,
            effective_date_time: self.effective_date_time,
        }
    }

    /// Provide the mandatory code first.
    #[must_use]
    pub fn code(self, code: CodeableConcept) -> ObservationBuilder<Unset, Set<CodeableConcept>> {
        ObservationBuilder {
            id: self.id,
            status: self.status,
            code: Set(code),
            subject: self.subject,
            value_quantity: self.value_quantity,
            effective_date_time: self.effective_date_time,
        }
    }
}

impl ObservationBuilder<Set<ObservationStatus>, Unset> {
    /// Provide the mandatory code, completing both required fields.
    #[must_use]
    pub fn code(
        self,
        code: CodeableConcept,
    ) -> ObservationBuilder<Set<ObservationStatus>, Set<CodeableConcept>> {
        ObservationBuilder {
            id: self.id,
            status: self.status,
            code: Set(code),
            subject: self.subject,
            value_quantity: self.value_quantity,
            effective_date_time: self.effective_date_time,
        }
    }
}

impl ObservationBuilder<Unset, Set<CodeableConcept>> {
    /// Provide the mandatory status, completing both required fields.
    #[must_use]
    pub fn status(
        self,
        status: ObservationStatus,
    ) -> ObservationBuilder<Set<ObservationStatus>, Set<CodeableConcept>> {
        ObservationBuilder {
            id: self.id,
            status: Set(status),
            code: self.code,
            subject: self.subject,
            value_quantity: self.value_quantity,
            effective_date_time: self.effective_date_time,
        }
    }
}

impl ObservationBuilder<Set<ObservationStatus>, Set<CodeableConcept>> {
    /// Finalize and construct the [`Observation`].
    ///
    /// Only callable once `status` and `code` have both been supplied.
    #[must_use]
    pub fn build(self) -> Observation {
        Observation {
            resource_type: "Observation".into(),
            unmodelled: serde_json::Map::new(),
            id: self.id,
            status: self.status.0,
            code: self.code.0,
            subject: self.subject,
            value_quantity: self.value_quantity,
            effective_date_time: self.effective_date_time,
        }
    }
}

impl crate::xml::DraftReader for ObservationDraft {
    fn open(
        &mut self,
        path: &[String],
        start: &quick_xml::events::BytesStart<'_>,
    ) -> Result<(), FhirValidationError> {
        let err = |msg: String| FhirValidationError::new("/f:Observation", msg);
        self.apply(path, start, &err)
    }

    fn close(&mut self, path: &[String]) {
        self.end(path);
    }
}

impl Observation {
    /// Check the date fields: `effectiveDateTime` is a FHIR `dateTime`.
    ///
    /// # Errors
    /// Returns an error naming the field if a date-time is malformed.
    pub(crate) fn check_dates(&self) -> Result<(), FhirValidationError> {
        match &self.effective_date_time {
            Some(value) => crate::types::check_date_time("/effectiveDateTime", value),
            None => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Coding;

    fn sample() -> Observation {
        Observation::builder()
            .id("obs-1")
            .status(ObservationStatus::Final)
            .code(CodeableConcept::from_coding(
                Coding::new("http://loinc.org", "29463-7").display("Body Weight"),
            ))
            .subject(Reference::to("Patient/nz-12345"))
            .value_quantity(Quantity::new(72.5, "kg"))
            .effective_date_time("2026-01-15T09:30:00Z")
            .build()
    }

    #[test]
    fn builder_accepts_mandatory_fields_in_either_order() {
        let a = Observation::builder()
            .status(ObservationStatus::Preliminary)
            .code(CodeableConcept::text("glucose"))
            .build();
        let b = Observation::builder()
            .code(CodeableConcept::text("glucose"))
            .status(ObservationStatus::Preliminary)
            .build();
        assert_eq!(a, b);
    }

    #[test]
    fn json_round_trip() {
        let observation = sample();
        let restored =
            Observation::from_json(&observation.to_json().expect("json")).expect("parse");
        assert_eq!(observation, restored);
    }

    #[test]
    fn xml_round_trip() {
        let observation = sample();
        let xml_bytes = observation.to_xml().expect("xml");
        let restored = Observation::from_xml(&xml_bytes).expect("parse xml");
        assert_eq!(observation, restored);
    }

    #[test]
    fn xml_rejects_missing_status() {
        let bytes = br#"<Observation xmlns="http://hl7.org/fhir"><id value="x"/></Observation>"#;
        let error = Observation::from_xml(bytes).expect_err("must fail");
        assert!(error.to_string().contains("status"));
    }

    #[test]
    fn status_codes_round_trip() {
        for code in [
            "registered",
            "preliminary",
            "final",
            "amended",
            "corrected",
            "cancelled",
            "entered-in-error",
            "unknown",
        ] {
            assert_eq!(
                ObservationStatus::from_code(code).map(|s| s.code()),
                Some(code)
            );
        }
        assert_eq!(ObservationStatus::from_code("bogus"), None);
    }
}
