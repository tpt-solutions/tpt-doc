use std::fmt;

/// A FHIR-specific validation error with field path reporting.
#[derive(Debug, Clone)]
pub struct FhirValidationError {
    /// JSON-pointer-style path to the field that failed validation.
    pub field_path: String,
    /// Human-readable description of the constraint that was violated.
    pub message: String,
}

impl FhirValidationError {
    /// Construct a new validation error.
    pub fn new(field_path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field_path: field_path.into(),
            message: message.into(),
        }
    }
}

impl fmt::Display for FhirValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.field_path, self.message)
    }
}

impl std::error::Error for FhirValidationError {}
