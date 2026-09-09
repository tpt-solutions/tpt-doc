#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::pedantic)]
//! HL7 FHIR R5 typed parsing, validation, and serialization.
//!
//! Resources are built via type-state builders that enforce mandatory fields
//! at compile time. The `build()` method is only available once all required
//! fields have been provided.

pub mod patient;
pub mod types;
pub mod validate;

pub use patient::{Patient, PatientBuilder};
pub use types::{CodeableConcept, HumanName, Identifier};
pub use validate::FhirValidationError;

/// Convenience re-export for common imports.
pub mod prelude {
    pub use super::{
        CodeableConcept, FhirValidationError, HumanName, Identifier, Patient, PatientBuilder,
    };
}
