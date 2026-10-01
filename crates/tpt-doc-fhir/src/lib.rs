//! # Examples
//!
//! ```
//! use tpt_doc_fhir::prelude::*;
//!
//! // `build()` is only available once the mandatory fields are set.
//! let patient = Patient::builder()
//!     .id("nz-12345")
//!     .name(HumanName::new("Smith", "John"))
//!     .build();
//! let bytes = patient.to_json()?;
//! let json = String::from_utf8_lossy(&bytes);
//! assert!(json.contains(r#""resourceType": "Patient""#));
//! # Ok::<(), tpt_doc_fhir::FhirValidationError>(())
//! ```
#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::pedantic)]
//! HL7 FHIR R5 typed parsing, validation, and serialization.
//!
//! Resources are built via type-state builders that enforce mandatory fields
//! at compile time. The `build()` method is only available once all required
//! fields have been provided. Runtime cross-field constraints (date ranges,
//! required-if rules) are checked by the functions in [`validate`].

/// FHIR Bundles: typed collections of resources.
pub mod bundle;
/// The `Encounter` resource and its type-state builder.
pub mod encounter;
/// The `Observation` resource and its type-state builder.
pub mod observation;
/// The `Organization` resource and its type-state builder.
pub mod organization;
/// The `Patient` resource and its type-state builder.
pub mod patient;
/// Shared FHIR R5 data types.
pub mod types;
/// Cross-field validation returning structured reports.
pub mod validate;
/// Shared FHIR XML serialization helpers.
pub(crate) mod xml;

pub use bundle::{Bundle, Resource};
pub use encounter::{Encounter, EncounterBuilder, EncounterStatus};
pub use observation::{Observation, ObservationBuilder, ObservationStatus};
pub use organization::{Organization, OrganizationBuilder};
pub use patient::{Patient, PatientBuilder};
pub use types::{CodeableConcept, Coding, HumanName, Identifier, Period, Quantity, Reference};
pub use validate::{FhirValidationError, validate_encounter, validate_observation};

/// Convenience re-export for common imports.
pub mod prelude {
    pub use super::{
        Bundle, CodeableConcept, Coding, Encounter, EncounterBuilder, EncounterStatus,
        FhirValidationError, HumanName, Identifier, Observation, ObservationBuilder,
        ObservationStatus, Organization, OrganizationBuilder, Patient, PatientBuilder, Period,
        Quantity, Reference, Resource, validate_encounter, validate_observation,
    };
}
