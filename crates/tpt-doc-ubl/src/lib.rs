//! # Examples
//!
//! ```
//! use tpt_doc_ubl::prelude::*;
//!
//! let date = time::Date::from_calendar_date(2026, time::Month::January, 15).unwrap();
//! let mut invoice = Invoice::new(
//!     "INV-001",
//!     InvoiceTypeCode::Invoice,
//!     date,
//!     Party::new("Supplier Ltd"),
//!     Party::new("Customer Ltd"),
//! );
//! invoice.push_line(InvoiceLine::new(1, "Consulting", 2.into(), 150.into()).unwrap());
//! assert_eq!(invoice.total(), Money::from_cents(30000)) ;
//! let xml = invoice.to_xml()?;
//! assert!(xml.windows(18).any(|w| w == b"urn:cen.eu:en16931"));
//! # Ok::<(), tpt_doc_core::DocError>(())
//! ```
#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::pedantic)]
//! UBL 2.1/2.3 and PEPPOL BIS Billing 3.0 e-invoicing.
//!
//! Invoices are modelled as typed structs covering the mandatory
//! PEPPOL BIS Billing 3.0 elements, with built-in arithmetic validation
//! against the EN 16931 business rules.

/// The `Invoice` document model and XML (de)serialization hooks.
pub mod invoice;
/// Exact decimal types (`Money`, `Decimal`) for invoice arithmetic.
pub mod money;
/// Shared UBL/PEPPOL data types.
pub mod types;
/// PEPPOL BIS Billing 3.0 validation rules.
pub mod validate;
mod vat;

pub use invoice::{Invoice, InvoiceTypeCode};
pub use money::{Decimal, Money};
pub use types::{
    Address, InvoiceLine, IssueDate, LineTax, MonetaryTotal, Party, PaymentMeans, TaxCategory,
    TaxSubtotal,
};
pub use validate::{UblValidationError, validate};

/// Convenience re-export for common imports.
pub mod prelude {
    pub use super::{
        Address, Decimal, Invoice, InvoiceLine, InvoiceTypeCode, LineTax, Money, Party,
        TaxCategory, TaxSubtotal, UblValidationError, validate,
    };
}
