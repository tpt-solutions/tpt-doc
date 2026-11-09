//! Implementations of the shared `tpt-doc-core` traits for the FHIR resources.
//!
//! The canonical format for FHIR here is JSON, so [`DocReader::from_bytes`]
//! and [`DocWriter::to_bytes`] are the JSON round trip, equal to `from_json`
//! and `to_json`. XML stays available through each type's `from_xml`/`to_xml`.

use tpt_doc_core::{DocError, DocReader, DocWriter, Location, Validate, ValidationReport};

use crate::bundle::Bundle;
use crate::encounter::Encounter;
use crate::observation::Observation;
use crate::organization::Organization;
use crate::patient::Patient;
use crate::validate::{FhirValidationError, validate_encounter, validate_observation};

impl From<FhirValidationError> for DocError {
    /// Keep the message and the path; the path becomes the error location.
    fn from(error: FhirValidationError) -> Self {
        DocError::InvalidFormat(error.message).with_location(Location::Path(error.field_path))
    }
}

macro_rules! json_traits {
    ($($ty:ty),* $(,)?) => {$(
        impl DocReader for $ty {
            /// Parse the FHIR JSON representation.
            fn from_bytes(bytes: &[u8]) -> Result<Self, DocError> {
                Ok(<$ty>::from_json(bytes)?)
            }
        }

        impl DocWriter for $ty {
            /// Serialize to the FHIR JSON representation.
            fn to_bytes(&self) -> Result<Vec<u8>, DocError> {
                Ok(<$ty>::to_json(self)?)
            }
        }
    )*};
}

json_traits!(Patient, Observation, Encounter, Organization, Bundle);

impl Validate for Observation {
    /// Run the FHIR R5 invariants and the status/value rules.
    fn validate(&self) -> ValidationReport {
        validate_observation(self)
    }
}

impl Validate for Encounter {
    /// Run the FHIR R5 invariants and the period rules.
    fn validate(&self) -> ValidationReport {
        validate_encounter(self)
    }
}
