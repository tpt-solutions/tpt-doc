//! # tpt-doc
//!
//! One dependency for the whole tpt-doc workspace. Each format lives behind a
//! Cargo feature and is re-exported as a module of the same name, so you pay
//! (in compile time and dependencies) only for what you enable:
//!
//! | Feature | Module | Crate |
//! |---|---|---|
//! | `pdf` | [`pdf`] | `tpt-doc-pdf` |
//! | `layout` | [`layout`] | `tpt-doc-layout` (HTML → PDF) |
//! | `sign` | [`sign`] | `tpt-doc-sign` (PAdES / XAdES / CAdES; pulls in `ring`) |
//! | `spreadsheet` | [`spreadsheet`] | `tpt-doc-spreadsheet` |
//! | `word` | [`word`] | `tpt-doc-word` |
//! | `fhir` | [`fhir`] | `tpt-doc-fhir` |
//! | `hl7v2` | [`hl7v2`] | `tpt-doc-hl7v2` |
//! | `edi` | [`edi`] | `tpt-doc-edi` |
//! | `ubl` | [`ubl`] | `tpt-doc-ubl` |
//! | `convert` | [`convert`] | `tpt-doc-convert` (HL7 v2 → FHIR) |
//! | `diff` | [`diff`] | `tpt-doc-diff` (structural diff) |
//! | `facturx` | — | UBL invoice embedded in a PDF |
//! | `serde` | — | serde support on validation reports |
//! | `full` | — | everything |
//!
//! The always-present [`core`] module holds the shared [`DocError`],
//! [`ValidationReport`] and [`Limits`] types.
//!
//! # Examples
//!
//! ```
//! use tpt_doc::prelude::*;
//!
//! fn check(report: &ValidationReport) -> Result<(), DocError> {
//!     if report.has_errors() {
//!         return Err(DocError::invalid_format("document failed validation"));
//!     }
//!     Ok(())
//! }
//! assert!(check(&ValidationReport::default()).is_ok());
//! ```

pub use tpt_doc_core as core;

#[cfg(feature = "diff")]
pub use tpt_doc_diff as diff;
#[cfg(feature = "convert")]
pub use tpt_doc_convert as convert;
#[cfg(feature = "edi")]
pub use tpt_doc_edi as edi;
#[cfg(feature = "fhir")]
pub use tpt_doc_fhir as fhir;
#[cfg(feature = "hl7v2")]
pub use tpt_doc_hl7v2 as hl7v2;
#[cfg(feature = "layout")]
pub use tpt_doc_layout as layout;
#[cfg(feature = "pdf")]
pub use tpt_doc_pdf as pdf;
#[cfg(feature = "sign")]
pub use tpt_doc_sign as sign;
#[cfg(feature = "spreadsheet")]
pub use tpt_doc_spreadsheet as spreadsheet;
#[cfg(feature = "ubl")]
pub use tpt_doc_ubl as ubl;
#[cfg(feature = "word")]
pub use tpt_doc_word as word;

pub use tpt_doc_core::{DocError, Limits, ValidationReport};

/// Common imports shared by every format.
///
/// Format-specific types stay under their module (`tpt_doc::fhir::prelude::*`)
/// because several formats export types with the same name (`Segment`, …).
pub mod prelude {
    pub use super::{DocError, Limits, ValidationReport};
}
