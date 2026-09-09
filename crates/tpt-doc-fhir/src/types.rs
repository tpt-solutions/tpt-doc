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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CodeableConcept {
    /// Short human-readable display text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

impl CodeableConcept {
    /// Construct a concept with display text only.
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            text: Some(text.into()),
        }
    }
}
