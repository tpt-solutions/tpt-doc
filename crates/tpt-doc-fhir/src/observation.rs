use serde::{Deserialize, Serialize};

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
        self.value_quantity.as_ref().map(|q| match &q.unit {
            Some(unit) => format!("{} {unit}", q.value),
            None => q.value.to_string(),
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
            format_args!(r#"<Observation xmlns="{}">"#, xml::FHIR_NS),
        );
        xml::value_element(&mut xml, "id", &self.id);
        xml::value_element(&mut xml, "status", self.status.code());
        xml::open_element(&mut xml, "code");
        write_concept(&mut xml, &self.code);
        xml::close_element(&mut xml, "code");
        if let Some(subject) = &self.subject {
            xml::open_element(&mut xml, "subject");
            write_reference(&mut xml, subject);
            xml::close_element(&mut xml, "subject");
        }
        if let Some(quantity) = &self.value_quantity {
            xml::open_element(&mut xml, "valueQuantity");
            write_quantity(&mut xml, quantity);
            xml::close_element(&mut xml, "valueQuantity");
        }
        if let Some(effective) = &self.effective_date_time {
            xml::value_element(&mut xml, "effectiveDateTime", effective);
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
        let mut reader = quick_xml::Reader::from_reader(bytes);
        let mut fields = ObservationFields::default();
        loop {
            match reader.read_event() {
                Ok(quick_xml::events::Event::Start(start)) => fields.enter(&start)?,
                Ok(quick_xml::events::Event::Empty(start)) => fields.apply_empty(&start)?,
                Ok(quick_xml::events::Event::End(end)) => {
                    fields.exit(end.local_name().as_ref());
                }
                Ok(quick_xml::events::Event::Eof) => break,
                Ok(_) => {}
                Err(e) => {
                    return Err(FhirValidationError::new(
                        "/f:Observation",
                        format!("XML error: {e}"),
                    ));
                }
            }
        }
        fields.into_observation()
    }
}

/// Scratch state accumulated while parsing an `<Observation>` document.
#[derive(Debug, Default)]
struct ObservationFields {
    id: String,
    status: Option<ObservationStatus>,
    code: CodeableConcept,
    subject: Option<Reference>,
    value_quantity: Option<Quantity>,
    effective: Option<String>,
    in_code: bool,
    in_subject: bool,
    in_quantity: bool,
}

impl ObservationFields {
    fn enter(
        &mut self,
        start: &quick_xml::events::BytesStart<'_>,
    ) -> Result<(), FhirValidationError> {
        let name = start.local_name().as_ref().to_owned();
        match name.as_str() {
            "Observation" | "coding" => {}
            "code" => self.in_code = true,
            "subject" => {
                self.in_subject = true;
                self.subject = Some(Reference {
                    reference: None,
                    display: None,
                });
            }
            "valueQuantity" => {
                self.in_quantity = true;
                self.value_quantity = Some(Quantity {
                    value: 0.0,
                    unit: None,
                    system: None,
                    code: None,
                });
            }
            other => {
                return Err(FhirValidationError::new(
                    "/f:Observation",
                    format!("unexpected element `{other}`"),
                ));
            }
        }
        Ok(())
    }

    fn exit(&mut self, name: &str) {
        match name {
            "code" => self.in_code = false,
            "subject" => self.in_subject = false,
            "valueQuantity" => self.in_quantity = false,
            _ => {}
        }
    }

    fn apply_empty(
        &mut self,
        start: &quick_xml::events::BytesStart<'_>,
    ) -> Result<(), FhirValidationError> {
        let name = start.local_name().as_ref().to_owned();
        let value = || xml::attr_value(start, "value").unwrap_or_default();
        let err = |msg: String| FhirValidationError::new("/f:Observation", msg);
        match name.as_str() {
            "id" => self.id = value(),
            "status" => {
                let code = value();
                self.status = Some(
                    ObservationStatus::from_code(&code)
                        .ok_or_else(|| err(format!("unknown status code `{code}`")))?,
                );
            }
            "text" if self.in_code => self.code.text = Some(value()),
            "system" | "code" | "display" if self.in_code => {
                let value = value();
                if self.code.coding.is_empty() {
                    self.code.coding.push(crate::Coding {
                        system: None,
                        code: String::new(),
                        display: None,
                    });
                }
                if let Some(coding) = self.code.coding.last_mut() {
                    match name.as_str() {
                        "system" => coding.system = Some(value),
                        "code" => coding.code = value,
                        "display" => coding.display = Some(value),
                        _ => {}
                    }
                }
            }
            "reference" | "display" if self.in_subject => {
                let value = value();
                if let Some(reference) = self.subject.as_mut() {
                    if name == "reference" {
                        reference.reference = Some(value);
                    } else {
                        reference.display = Some(value);
                    }
                }
            }
            "value" | "unit" if self.in_quantity => {
                let attr = value();
                if let Some(quantity) = self.value_quantity.as_mut() {
                    match name.as_str() {
                        "value" => {
                            quantity.value = attr
                                .parse::<f64>()
                                .map_err(|_| err(format!("non-numeric quantity `{attr}`")))?;
                        }
                        _ => quantity.unit = Some(attr),
                    }
                }
            }
            "system" | "code" if self.in_quantity => {
                let attr = value();
                if let Some(quantity) = self.value_quantity.as_mut() {
                    if name == "system" {
                        quantity.system = Some(attr);
                    } else {
                        quantity.code = Some(attr);
                    }
                }
            }
            "effectiveDateTime" => self.effective = Some(value()),
            _ => {}
        }
        Ok(())
    }

    fn into_observation(self) -> Result<Observation, FhirValidationError> {
        let status = self.status.ok_or_else(|| {
            FhirValidationError::new(
                "/f:Observation",
                "missing mandatory element `status`".to_owned(),
            )
        })?;
        Ok(Observation {
            resource_type: "Observation".into(),
            id: self.id,
            status,
            code: self.code,
            subject: self.subject,
            value_quantity: self.value_quantity,
            effective_date_time: self.effective,
        })
    }
}

/// Serialize a `CodeableConcept`'s children inside an open `<code>` element.
fn write_concept(xml: &mut String, concept: &CodeableConcept) {
    for coding in &concept.coding {
        xml::open_element(xml, "coding");
        if let Some(system) = &coding.system {
            xml::value_element(xml, "system", system);
        }
        xml::value_element(xml, "code", &coding.code);
        if let Some(display) = &coding.display {
            xml::value_element(xml, "display", display);
        }
        xml::close_element(xml, "coding");
    }
    if let Some(text) = &concept.text {
        xml::value_element(xml, "text", text);
    }
}

/// Serialize a `Reference`'s children inside an open `<subject>` element.
fn write_reference(xml: &mut String, reference: &Reference) {
    if let Some(target) = &reference.reference {
        xml::value_element(xml, "reference", target);
    }
    if let Some(display) = &reference.display {
        xml::value_element(xml, "display", display);
    }
}

/// Serialize a `Quantity`'s children inside an open `valueQuantity` element.
fn write_quantity(xml: &mut String, quantity: &Quantity) {
    xml::value_element(xml, "value", &quantity.value.to_string());
    if let Some(unit) = &quantity.unit {
        xml::value_element(xml, "unit", unit);
    }
    if let Some(system) = &quantity.system {
        xml::value_element(xml, "system", system);
    }
    if let Some(code) = &quantity.code {
        xml::value_element(xml, "code", code);
    }
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
            id: self.id,
            status: self.status.0,
            code: self.code.0,
            subject: self.subject,
            value_quantity: self.value_quantity,
            effective_date_time: self.effective_date_time,
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
