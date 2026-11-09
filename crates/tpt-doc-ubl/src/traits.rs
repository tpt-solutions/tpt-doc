//! Implementations of the shared `tpt-doc-core` traits for [`Invoice`].
//!
//! The canonical format for UBL is XML, so [`DocReader::from_bytes`] and
//! [`DocWriter::to_bytes`] are the XML round trip, equal to
//! [`Invoice::from_xml`] and [`Invoice::to_xml`].

use tpt_doc_core::{DocError, DocReader, DocWriter, Validate, ValidationReport};

use crate::{Invoice, UblValidationError};

impl DocReader for Invoice {
    /// Parse a UBL 2.1 `Invoice` or `CreditNote` document.
    fn from_bytes(bytes: &[u8]) -> Result<Self, DocError> {
        Self::from_xml(bytes)
    }
}

impl DocWriter for Invoice {
    /// Serialize to UBL 2.1 XML.
    fn to_bytes(&self) -> Result<Vec<u8>, DocError> {
        self.to_xml()
    }
}

impl Validate for Invoice {
    /// Run the PEPPOL BIS Billing 3.0 rules and report every violation.
    fn validate(&self) -> ValidationReport {
        UblValidationError::to_report(&crate::validate::validate(self))
    }
}
