//! Format converters for the tpt-doc workspace.
//!
//! Currently: HL7 v2 → FHIR R5 ([`hl7v2_to_fhir`]).
//!
//! # Examples
//!
//! ```
//! use tpt_doc_convert::hl7v2_to_fhir;
//!
//! let message = b"MSH|^~\\&|HIS|ACME|RIS|ACME|20261001120000||ADT^A01|MSG1|P|2.5.1\r\
//!                 PID|1||12345^^^NZHPI^MR||Smith^John||19700101|M\r\
//!                 PV1|1|I|WARD4^101^A";
//! let conversion = hl7v2_to_fhir(message)?;
//! assert_eq!(conversion.bundle.entries.len(), 2); // Patient + Encounter
//! # Ok::<(), tpt_doc_core::DocError>(())
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod hl7_to_fhir;

pub use hl7_to_fhir::{Conversion, hl7v2_to_fhir};
