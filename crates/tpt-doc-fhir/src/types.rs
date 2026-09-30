use serde::{Deserialize, Serialize};

/// A human name with family and given name components.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HumanName {
    /// Family name (surname).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub family: Option<String>,
    /// Given names (first name, middle names).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub given: Vec<String>,
}

impl HumanName {
    /// Construct a name with a single family and given name.
    pub fn new(family: impl Into<String>, given: impl Into<String>) -> Self {
        Self {
            family: Some(family.into()),
            given: vec![given.into()],
        }
    }
}

/// A FHIR Identifier: a system + value pair.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Identifier {
    /// URI that defines the namespace of the identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    /// The identifier value within the namespace.
    pub value: String,
}

/// A codeable concept: a code from a terminology system, plus optional display text.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CodeableConcept {
    /// Coded entries from a terminology system.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub coding: Vec<Coding>,
    /// Short human-readable display text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

impl CodeableConcept {
    /// Construct a concept with display text only.
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            coding: Vec::new(),
            text: Some(text.into()),
        }
    }

    /// Construct a concept from a single coding.
    #[must_use]
    pub fn from_coding(coding: Coding) -> Self {
        Self {
            coding: vec![coding],
            text: None,
        }
    }
}

/// A reference to a code defined by a terminology system.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Coding {
    /// URI of the terminology system (e.g. `"http://loinc.org"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    /// The code value in the system.
    pub code: String,
    /// Human-readable representation of the code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
}

impl Coding {
    /// Construct a coding with system and code.
    #[must_use]
    pub fn new(system: impl Into<String>, code: impl Into<String>) -> Self {
        Self {
            system: Some(system.into()),
            code: code.into(),
            display: None,
        }
    }

    /// Attach a human-readable display string.
    #[must_use]
    pub fn display(mut self, display: impl Into<String>) -> Self {
        self.display = Some(display.into());
        self
    }
}

/// A reference from one resource to another.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Reference {
    /// Literal reference, e.g. `"Patient/nz-12345"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    /// Human-readable text summary of the target.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
}

impl Reference {
    /// Construct a literal reference to `resource/id`.
    #[must_use]
    pub fn to(target: impl Into<String>) -> Self {
        Self {
            reference: Some(target.into()),
            display: None,
        }
    }
}

/// A time period defined by a start and (optional) end date/time.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Period {
    /// Starting time (ISO 8601 date or date-time).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<String>,
    /// End time, inclusive.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<String>,
}

impl Period {
    /// Construct a period from optional start and end strings.
    #[must_use]
    pub fn new(start: impl Into<String>, end: impl Into<String>) -> Self {
        Self {
            start: Some(start.into()),
            end: Some(end.into()),
        }
    }
}

/// A measured or stated amount (`Quantity` datatype).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Quantity {
    /// The numerical value.
    pub value: f64,
    /// Human-readable unit (e.g. `"mg"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    /// URI of the unit code system (e.g. UCUM).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    /// The unit as a code in `system`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

impl Quantity {
    /// Construct a quantity with a value and human-readable unit.
    #[must_use]
    pub fn new(value: f64, unit: impl Into<String>) -> Self {
        Self {
            value,
            unit: Some(unit.into()),
            system: None,
            code: None,
        }
    }
}
