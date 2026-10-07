use serde::{Deserialize, Serialize};
use tpt_doc_core::DocError;
use tpt_doc_core::Limits;
use tpt_doc_core::Location;
use tpt_doc_core::escape_xml_text;

use crate::types::{
    Address, Decimal, InvoiceLine, IssueDate, LineTax, MonetaryTotal, Money, Party, PaymentMeans,
    TaxCategory, TaxSubtotal,
};

/// UBL document type code (`cbc:InvoiceTypeCode` / `cbc:CreditNoteTypeCode`).
///
/// The two named variants are the ones PEPPOL BIS Billing 3.0 requires. Any
/// other UN/EDIFACT 1001 code round-trips through [`InvoiceTypeCode::Other`]
/// rather than being rejected — a document type code is data the sender chose,
/// not a value this crate gets to refuse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InvoiceTypeCode {
    /// `380` — commercial invoice.
    Invoice,
    /// `381` — credit note.
    CreditNote,
    /// Any other UN/EDIFACT 1001 code, preserved verbatim.
    ///
    /// Common values include `384` (self-billed credit note), `389` (self-billed
    /// debit note), `386` (prepayment invoice), and `261`/`262` (self-billed
    /// invoice/debit note).
    Other(u16),
}

impl InvoiceTypeCode {
    /// The UBL type code value.
    #[must_use]
    pub fn value(&self) -> u16 {
        match self {
            Self::Invoice => 380,
            Self::CreditNote => 381,
            Self::Other(code) => *code,
        }
    }

    /// Parse a UN/EDIFACT 1001 document type code.
    ///
    /// # Errors
    /// Returns [`DocError::InvalidFormat`] if `code` is not a number in the
    /// UN/EDIFACT 1001 range (one to four digits).
    pub fn from_code(code: &str) -> Result<Self, DocError> {
        let trimmed = code.trim();
        let value: u16 = trimmed.parse().map_err(|_| {
            DocError::invalid_format(format!("document type code is not a number: `{trimmed}`"))
        })?;
        // UN/EDIFACT 1001 codes are one to four digits, e.g. `80` (consolidated
        // invoice), `261` (self-billed invoice), `380` (commercial invoice).
        if !(1..=9999).contains(&value) {
            return Err(DocError::invalid_format(format!(
                "document type code `{trimmed}` is outside the UN/EDIFACT 1001 range"
            )));
        }
        Ok(match value {
            380 => Self::Invoice,
            381 => Self::CreditNote,
            other => Self::Other(other),
        })
    }

    /// Whether this code denotes a credit note.
    ///
    /// A credit note reverses an invoice, so its line items are
    /// `CreditNoteLine` and its quantities `CreditedQuantity`.
    #[must_use]
    pub fn is_credit_note(&self) -> bool {
        matches!(self, Self::CreditNote)
    }

    /// Resolve the type code of a document with the given root element.
    ///
    /// A `CreditNote` root *is* the credit note, whatever sub-code it carries:
    /// `CreditNoteTypeCode` is where `384` (self-billed credit note) and `381`
    /// both land, and the line elements follow the root, not the number.
    /// Normalising here keeps serialize/parse round-trips stable — keeping the
    /// sub-code would emit a `CreditNote` document as an `Invoice` on the way
    /// back out, because the root is chosen from this value.
    pub(crate) fn for_root(root_is_credit_note: bool, code: &str) -> Result<Self, DocError> {
        let parsed = Self::from_code(code)?;
        Ok(if root_is_credit_note {
            Self::CreditNote
        } else {
            parsed
        })
    }
}

/// A UBL 2.1 invoice covering the PEPPOL BIS Billing 3.0 mandatory elements:
/// document ID, issue date, supplier, customer, invoice lines, tax totals,
/// and legal monetary totals.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Invoice {
    /// `cbc:CustomizationID` — the EN 16931 specification identifier (BT-24).
    pub customization_id: String,
    /// `cbc:ProfileID` — the PEPPOL BIS billing profile identifier (BT-23).
    pub profile_id: String,
    /// `cbc:ID` — the invoice document number.
    pub id: String,
    /// `cbc:InvoiceTypeCode` — invoice or credit note.
    pub type_code: InvoiceTypeCode,
    /// `cbc:IssueDate` — the invoice issue date.
    pub issue_date: IssueDate,
    /// `cac:AccountingSupplierParty`.
    pub supplier: Party,
    /// `cac:AccountingCustomerParty`.
    pub customer: Party,
    /// `cac:InvoiceLine` items, in order.
    pub lines: Vec<InvoiceLine>,
    /// `cac:TaxSubtotal` entries, one per tax category.
    pub tax_subtotals: Vec<TaxSubtotal>,
    /// `cac:LegalMonetaryTotal`.
    pub monetary_total: MonetaryTotal,
    /// `cbc:DocumentCurrencyCode` (ISO 4217, e.g. `"NZD"`).
    pub currency: String,
    /// `cbc:BuyerReference` (BT-10) — the buyer's routing or order reference.
    /// PEPPOL-EN16931-R003 requires this or an order reference.
    pub buyer_reference: Option<String>,
    /// `cbc:DueDate` (BT-9). Only written for invoices: a `CreditNote` has no
    /// document-level due date.
    pub due_date: Option<IssueDate>,
    /// `cac:PaymentMeans` (BG-16).
    pub payment_means: Option<PaymentMeans>,
}

impl Invoice {
    /// Create an invoice with no lines, zero totals, and `NZD` currency.
    ///
    /// Totals are recomputed automatically as lines and tax subtotals are
    /// added via [`push_line`](Self::push_line) and
    /// [`push_tax_subtotal`](Self::push_tax_subtotal).
    pub fn new(
        id: impl Into<String>,
        type_code: InvoiceTypeCode,
        issue_date: IssueDate,
        supplier: Party,
        customer: Party,
    ) -> Self {
        Self {
            customization_id: "urn:cen.eu:en16931:2017".to_owned(),
            profile_id: "urn:fdc:peppol.eu:2017:poacc:billing:01:1.0".to_owned(),
            id: id.into(),
            type_code,
            issue_date,
            supplier,
            customer,
            lines: Vec::new(),
            tax_subtotals: Vec::new(),
            monetary_total: MonetaryTotal::default(),
            currency: "NZD".to_owned(),
            buyer_reference: None,
            due_date: None,
            payment_means: None,
        }
    }

    /// Append an invoice line and recompute document totals.
    pub fn push_line(&mut self, line: InvoiceLine) {
        self.lines.push(line);
        self.recalculate_totals();
    }

    /// Append a tax subtotal and recompute document totals.
    pub fn push_tax_subtotal(&mut self, subtotal: TaxSubtotal) {
        self.tax_subtotals.push(subtotal);
        self.recalculate_totals();
    }

    /// The amount payable (`PayableAmount`).
    #[must_use]
    pub fn total(&self) -> Money {
        self.monetary_total.payable
    }

    /// Recompute `LegalMonetaryTotal` from lines and tax subtotals:
    ///
    /// - `line_extension` = Σ line net amounts (BR-CO-10)
    /// - `tax_exclusive` = `line_extension` (BR-CO-13; no allowances or charges yet)
    /// - `tax_inclusive` = `tax_exclusive` + Σ tax amounts (BR-CO-15)
    /// - `payable` = `tax_inclusive` (BR-CO-16)
    fn recalculate_totals(&mut self) {
        let tax_exclusive: Money = self.lines.iter().map(|l| l.line_extension_amount).sum();
        let tax_amount: Money = self.tax_subtotals.iter().map(|t| t.tax_amount).sum();
        let tax_inclusive = tax_exclusive + tax_amount;
        self.monetary_total = MonetaryTotal {
            line_extension: tax_exclusive,
            tax_exclusive,
            tax_inclusive,
            payable: tax_inclusive,
        };
    }

    /// Serialize to a UBL 2.1 document with PEPPOL BIS 3.0 identification.
    ///
    /// The root element is `Invoice` or `CreditNote` per [`Self::type_code`];
    /// namespaces follow `urn:oasis:names:specification:ubl:schema:xsd:*-2`.
    ///
    /// # Errors
    /// Returns [`DocError`] if XML serialization fails.
    pub fn to_xml(&self) -> Result<Vec<u8>, DocError> {
        use std::fmt::Write as _;

        // Only a credit note uses the `CreditNote` root; any other code is an
        // invoice document that happens to carry a different type code.
        let root = if self.type_code.is_credit_note() {
            "CreditNote"
        } else {
            "Invoice"
        };
        let mut xml = String::with_capacity(2048);
        let _ = write!(xml, r#"<?xml version="1.0" encoding="UTF-8"?>"#);
        let _ = write!(
            xml,
            r#"<{root} xmlns="urn:oasis:names:specification:ubl:schema:xsd:{root}-2" xmlns:cac="urn:oasis:names:specification:ubl:schema:xsd:CommonAggregateComponents-2" xmlns:cbc="urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2">"#
        );
        cbc(&mut xml, "CustomizationID", &self.customization_id)?;
        cbc(&mut xml, "ProfileID", &self.profile_id)?;
        cbc(&mut xml, "ID", &self.id)?;
        let issue_date = self
            .issue_date
            .format(&DATE_FORMAT)
            .map_err(|e| DocError::invalid_format(format!("IssueDate formatting failed: {e}")))?;
        cbc(&mut xml, "IssueDate", &issue_date)?;
        if let (Some(due), false) = (self.due_date, self.type_code.is_credit_note()) {
            let due = due
                .format(&DATE_FORMAT)
                .map_err(|e| DocError::invalid_format(format!("DueDate formatting failed: {e}")))?;
            cbc(&mut xml, "DueDate", &due)?;
        }
        // A `CreditNote` root must carry `cbc:CreditNoteTypeCode`; emitting
        // `cbc:InvoiceTypeCode` there is not valid UBL and PEPPOL receivers
        // reject the document.
        let type_code_element = if self.type_code.is_credit_note() {
            "CreditNoteTypeCode"
        } else {
            "InvoiceTypeCode"
        };
        cbc(
            &mut xml,
            type_code_element,
            &self.type_code.value().to_string(),
        )?;
        cbc(&mut xml, "DocumentCurrencyCode", &self.currency)?;
        if let Some(reference) = &self.buyer_reference {
            cbc(&mut xml, "BuyerReference", reference)?;
        }

        party_block(&mut xml, "AccountingSupplierParty", &self.supplier)?;
        party_block(&mut xml, "AccountingCustomerParty", &self.customer)?;

        if let Some(means) = &self.payment_means {
            write_payment_means(&mut xml, means)?;
        }

        let tax_total: Money = self.tax_subtotals.iter().map(|t| t.tax_amount).sum();
        xml.push_str("<cac:TaxTotal>");
        cbc_amount(&mut xml, "TaxAmount", tax_total, &self.currency)?;
        for subtotal in &self.tax_subtotals {
            write_tax_subtotal(&mut xml, subtotal, &self.currency)?;
        }
        xml.push_str("</cac:TaxTotal>");

        xml.push_str("<cac:LegalMonetaryTotal>");
        cbc_amount(
            &mut xml,
            "LineExtensionAmount",
            self.monetary_total.line_extension,
            &self.currency,
        )?;
        cbc_amount(
            &mut xml,
            "TaxExclusiveAmount",
            self.monetary_total.tax_exclusive,
            &self.currency,
        )?;
        cbc_amount(
            &mut xml,
            "TaxInclusiveAmount",
            self.monetary_total.tax_inclusive,
            &self.currency,
        )?;
        cbc_amount(
            &mut xml,
            "PayableAmount",
            self.monetary_total.payable,
            &self.currency,
        )?;
        xml.push_str("</cac:LegalMonetaryTotal>");

        // The line element name and the quantity element name both follow the document
        // type: a credit note has `CreditNoteLine` and `CreditedQuantity`.
        let (line_element, quantity_element) = if self.type_code.is_credit_note() {
            ("CreditNoteLine", "CreditedQuantity")
        } else {
            ("InvoiceLine", "InvoicedQuantity")
        };
        for line in &self.lines {
            write_line(
                &mut xml,
                line,
                (line_element, quantity_element),
                &self.currency,
            )?;
        }

        let _ = write!(xml, "</{root}>");
        Ok(xml.into_bytes())
    }

    /// Parse a UBL 2.1 `Invoice` or `CreditNote` document.
    ///
    /// Totals and tax subtotals are taken from the document; arithmetic
    /// consistency can then be checked with [`validate`](crate::validate).
    ///
    /// # Errors
    /// Returns [`DocError`] if the XML is not a valid UBL invoice document.
    pub fn from_xml(bytes: &[u8]) -> Result<Self, DocError> {
        Self::from_xml_with_limits(bytes, Limits::default())
    }

    /// Parse a UBL 2.1 invoice with explicit [`Limits`] applied.
    ///
    /// Nesting is bounded by `limits.max_xml_depth`; a UBL document is only a
    /// few levels deep, so anything deeper is refused rather than walked.
    ///
    /// # Errors
    /// Returns [`DocError`] if the XML is not a valid UBL invoice document or
    /// exceeds `limits.max_xml_depth`.
    pub fn from_xml_with_limits(bytes: &[u8], limits: Limits) -> Result<Self, DocError> {
        use quick_xml::events::Event;

        let mut reader = quick_xml::Reader::from_reader(bytes);
        let mut fields = InvoiceFields::default();
        let mut path: Vec<String> = Vec::new();
        // Text is buffered per leaf element: entity references and text
        // arrive as separate events and must concatenate.
        let mut pending_text = String::new();
        loop {
            match reader.read_event() {
                Ok(Event::Start(start)) => {
                    let name = start.local_name().as_ref().to_owned();
                    // The root element is validated but never pushed: every
                    // path handed to `apply_element` is relative to the root, so
                    // a root-relative "direct child" test is just a length check.
                    if path.is_empty() {
                        match name.as_str() {
                            "Invoice" | "CreditNote" => {
                                // The root element decides the document kind;
                                // the type code refines it.
                                fields.root_is_credit_note = name == "CreditNote";
                            }
                            other => {
                                return Err(DocError::invalid_format(format!(
                                    "unexpected root element `{other}`"
                                )));
                            }
                        }
                        // `path` stays empty for the root's children, so push the
                        // root name back on Enter and pop it on its End.
                        path.push(name);
                    } else {
                        match name.as_str() {
                            "AccountingSupplierParty" => fields.party_target = 0,
                            "AccountingCustomerParty" => fields.party_target = 1,
                            _ => {}
                        }
                        path.push(name);
                    }
                    limits.check_xml_depth(path.len())?;
                    // A new element starts a fresh text buffer.
                    pending_text.clear();
                    // `schemeID` (endpoint) and `unitCode` (quantity) are the
                    // only attributes the model keeps.
                    fields.leaf_attr = start
                        .attributes()
                        .filter_map(std::result::Result::ok)
                        .find(|a| matches!(a.key.local_name().as_ref(), "schemeID" | "unitCode"))
                        .and_then(|a| {
                            a.normalized_value(quick_xml::XmlVersion::Implicit1_0)
                                .ok()
                                .map(std::borrow::Cow::into_owned)
                        });
                }
                // Empty elements carry no text data we model — they fall
                // through to the catch-all with their End handler running.
                Ok(Event::Text(text)) => {
                    let chunk = quick_xml::escape::unescape(&text).map_err(|e| {
                        DocError::invalid_format(format!("invalid XML entities: {e}"))
                    })?;
                    pending_text.push_str(&chunk);
                }
                // Entity references arrive as separate events and concatenate
                // with surrounding text.
                Ok(Event::GeneralRef(reference)) => {
                    // The parser knows the offset, so an unresolvable entity
                    // points at itself rather than only at the message.
                    let at = reader.buffer_position();
                    pending_text.push_str(
                        &resolve_ref(&reference)
                            .map_err(|e| e.with_location(Location::Offset(at)))?,
                    );
                }
                Ok(Event::End(end)) => {
                    let name = end.local_name().as_ref().to_owned();
                    // `path[0]` is the root; `apply_element` wants the ancestors
                    // below it, with the leaf itself popped. The root's own End
                    // has nothing left to hand over.
                    let is_root = path.len() == 1;
                    path.pop();
                    if !is_root {
                        fields.apply_element(&path[1..], &name, &pending_text)?;
                    }
                    pending_text.clear();
                    match name.as_str() {
                        // A credit note's lines are `CreditNoteLine`, so
                        // handling only `InvoiceLine` silently dropped every
                        // line of a credit note.
                        "InvoiceLine" | "CreditNoteLine" => fields.finish_line(),
                        "TaxSubtotal" => fields.finish_subtotal(),
                        _ => {}
                    }
                }
                Ok(Event::Eof) => break,
                Ok(_) => {}
                Err(e) => {
                    // The parser knows where it gave up, so the message says so
                    // rather than leaving the reader to re-scan the document.
                    return Err(DocError::invalid_format(format!("XML error: {e}"))
                        .with_location(Location::Offset(reader.buffer_position())));
                }
            }
        }
        fields.into_invoice()
    }
}

/// Emit `cac:PaymentMeans`.
///
/// # Errors
/// Returns [`DocError::InvalidFormat`] if any text contains an illegal character.
fn write_payment_means(xml: &mut String, means: &PaymentMeans) -> Result<(), DocError> {
    xml.push_str("<cac:PaymentMeans>");
    cbc(xml, "PaymentMeansCode", &means.code)?;
    if let Some(id) = &means.payment_id {
        cbc(xml, "PaymentID", id)?;
    }
    if let Some(account) = &means.account_id {
        xml.push_str("<cac:PayeeFinancialAccount>");
        cbc(xml, "ID", account)?;
        xml.push_str("</cac:PayeeFinancialAccount>");
    }
    xml.push_str("</cac:PaymentMeans>");
    Ok(())
}

/// Emit one `InvoiceLine` / `CreditNoteLine`. `names` is
/// `(line element, quantity element)`, which follow the document type.
///
/// # Errors
/// Returns [`DocError::InvalidFormat`] if any text contains an illegal character.
fn write_line(
    xml: &mut String,
    line: &InvoiceLine,
    names: (&str, &str),
    currency: &str,
) -> Result<(), DocError> {
    use std::fmt::Write as _;
    let (line_element, quantity_element) = names;
    let _ = write!(xml, "<cac:{line_element}>");
    cbc(xml, "ID", &line.id.to_string())?;
    let quantity = escape_xml_text(&line.quantity.to_string())?;
    let unit_code = escape_xml_text(&line.unit_code)?;
    let _ = write!(
        xml,
        "<cbc:{quantity_element} unitCode=\"{unit_code}\">{quantity}</cbc:{quantity_element}>"
    );
    cbc_amount(
        xml,
        "LineExtensionAmount",
        line.line_extension_amount,
        currency,
    )?;
    xml.push_str("<cac:Item>");
    cbc(xml, "Name", &line.item_name)?;
    if let Some(tax) = &line.tax {
        xml.push_str("<cac:ClassifiedTaxCategory>");
        write_tax_category_body(xml, tax.category, tax.rate_percent, None)?;
        xml.push_str("</cac:ClassifiedTaxCategory>");
    }
    xml.push_str("</cac:Item>");
    xml.push_str("<cac:Price>");
    cbc_price(xml, "PriceAmount", line.unit_price, currency)?;
    xml.push_str("</cac:Price>");
    let _ = write!(xml, "</cac:{line_element}>");
    Ok(())
}

/// `cbc:IssueDate` carries a bare ISO calendar date (`YYYY-MM-DD`).
const DATE_FORMAT: &[time::format_description::BorrowedFormatItem] =
    time::macros::format_description!("[year]-[month]-[day]");

/// Append `<cbc:{name}>{escaped}</cbc:{name}>`.
///
/// # Errors
/// Returns [`DocError::InvalidFormat`] if `value` contains a character XML
/// 1.0 forbids; emitting it raw yields an invoice that PEPPOL receivers reject.
fn cbc(xml: &mut String, name: &str, value: &str) -> Result<(), DocError> {
    use std::fmt::Write as _;
    let escaped = escape_xml_text(value)?;
    let _ = write!(xml, "<cbc:{name}>{escaped}</cbc:{name}>");
    Ok(())
}

/// Like [`cbc_amount`] for a price, which EN 16931 allows more than two decimals.
///
/// # Errors
/// Returns [`DocError::InvalidFormat`] if `currency` contains an illegal
/// character.
fn cbc_price(xml: &mut String, name: &str, price: Decimal, currency: &str) -> Result<(), DocError> {
    use std::fmt::Write as _;
    let escaped_currency = escape_xml_text(currency)?;
    let _ = write!(
        xml,
        "<cbc:{name} currencyID=\"{escaped_currency}\">{}</cbc:{name}>",
        price.to_string_min_scale(2)
    );
    Ok(())
}

/// Append `<cbc:{name} currencyID="{ccy}">{amount}</cbc:{name}>` (two decimals).
///
/// # Errors
/// Returns [`DocError::InvalidFormat`] if `currency` contains an illegal
/// character.
fn cbc_amount(xml: &mut String, name: &str, amount: Money, currency: &str) -> Result<(), DocError> {
    use std::fmt::Write as _;
    let escaped_currency = escape_xml_text(currency)?;
    let _ = write!(
        xml,
        "<cbc:{name} currencyID=\"{escaped_currency}\">{amount}</cbc:{name}>",
    );
    Ok(())
}

/// Emit one `<cac:TaxSubtotal>` block with its tax category.
///
/// # Errors
/// Returns [`DocError::InvalidFormat`] if any emitted text contains an
/// illegal character.
fn write_tax_subtotal(
    xml: &mut String,
    subtotal: &TaxSubtotal,
    currency: &str,
) -> Result<(), DocError> {
    xml.push_str("<cac:TaxSubtotal>");
    cbc_amount(xml, "TaxableAmount", subtotal.taxable_amount, currency)?;
    cbc_amount(xml, "TaxAmount", subtotal.tax_amount, currency)?;
    xml.push_str("<cac:TaxCategory>");
    write_tax_category_body(
        xml,
        subtotal.category,
        subtotal.rate_percent,
        subtotal.exemption_reason.as_deref(),
    )?;
    xml.push_str("</cac:TaxCategory>");
    xml.push_str("</cac:TaxSubtotal>");
    Ok(())
}

/// The children shared by `cac:TaxCategory` and `cac:ClassifiedTaxCategory`:
/// `ID`, `Percent` and the `TaxScheme` (always `VAT` in BIS Billing 3.0).
///
/// # Errors
/// Returns [`DocError::InvalidFormat`] if the percentage text is not XML-safe.
fn write_tax_category_body(
    xml: &mut String,
    category: TaxCategory,
    rate_percent: Decimal,
    exemption_reason: Option<&str>,
) -> Result<(), DocError> {
    cbc(xml, "ID", category.code())?;
    cbc(xml, "Percent", &rate_percent.to_string())?;
    if let Some(reason) = exemption_reason {
        cbc(xml, "TaxExemptionReason", reason)?;
    }
    xml.push_str("<cac:TaxScheme>");
    cbc(xml, "ID", "VAT")?;
    xml.push_str("</cac:TaxScheme>");
    Ok(())
}

/// Emit an `Accounting{Supplier,Customer}Party` block.
///
/// # Errors
/// Returns [`DocError::InvalidFormat`] if any party text contains an illegal
/// character.
fn party_block(xml: &mut String, wrapper: &str, party: &Party) -> Result<(), DocError> {
    use std::fmt::Write as _;
    let _ = write!(xml, "<cac:{wrapper}><cac:Party>");
    if let Some(endpoint) = &party.endpoint_id {
        let escaped = escape_xml_text(endpoint)?;
        match &party.endpoint_scheme {
            Some(scheme) => {
                let scheme = escape_xml_text(scheme)?;
                let _ = write!(
                    xml,
                    "<cbc:EndpointID schemeID=\"{scheme}\">{escaped}</cbc:EndpointID>"
                );
            }
            None => {
                let _ = write!(xml, "<cbc:EndpointID>{escaped}</cbc:EndpointID>");
            }
        }
    }
    xml.push_str("<cac:PartyName>");
    cbc(xml, "Name", &party.name)?;
    xml.push_str("</cac:PartyName>");
    if let Some(address) = &party.address {
        xml.push_str("<cac:PostalAddress>");
        if let Some(street) = &address.street {
            cbc(xml, "StreetName", street)?;
        }
        if let Some(city) = &address.city {
            cbc(xml, "CityName", city)?;
        }
        if let Some(postal) = &address.postal_code {
            cbc(xml, "PostalZone", postal)?;
        }
        if let Some(country) = &address.country_code {
            xml.push_str("<cac:Country>");
            cbc(xml, "IdentificationCode", country)?;
            xml.push_str("</cac:Country>");
        }
        xml.push_str("</cac:PostalAddress>");
    }
    if let Some(tax_id) = &party.tax_id {
        xml.push_str("<cac:PartyTaxScheme>");
        cbc(xml, "CompanyID", tax_id)?;
        xml.push_str("<cac:TaxScheme>");
        cbc(xml, "ID", "VAT")?;
        xml.push_str("</cac:TaxScheme>");
        xml.push_str("</cac:PartyTaxScheme>");
    }
    xml.push_str("<cac:PartyLegalEntity>");
    cbc(
        xml,
        "RegistrationName",
        party.legal_name.as_deref().unwrap_or(&party.name),
    )?;
    if let Some(legal_id) = &party.legal_id {
        cbc(xml, "CompanyID", legal_id)?;
    }
    xml.push_str("</cac:PartyLegalEntity>");
    let _ = write!(xml, "</cac:Party></cac:{wrapper}>");
    Ok(())
}

/// Scratch state accumulated while parsing an UBL invoice document.
#[derive(Debug, Default)]
struct InvoiceFields {
    customization_id: String,
    profile_id: String,
    id: String,
    type_code: Option<InvoiceTypeCode>,
    /// Whether the root element was `CreditNote`, which decides the document
    /// kind independently of the numeric type code.
    root_is_credit_note: bool,
    issue_date: Option<IssueDate>,
    currency: String,
    party_target: usize, // 0 = supplier, 1 = customer
    parties: [PartyFields; 2],
    line: LineFields,
    lines: Vec<InvoiceLine>,
    tax_subtotals: Vec<TaxSubtotal>,
    subtotal: Option<SubtotalFields>,
    monetary: MonetaryTotal,
    buyer_reference: String,
    due_date: Option<IssueDate>,
    payment: Option<PaymentMeans>,
    /// `schemeID` / `unitCode` of the element that just ended.
    leaf_attr: Option<String>,
}

#[derive(Debug, Default)]
struct PartyFields {
    name: String,
    city: String,
    postal_code: String,
    country: String,
    tax_id: String,
    endpoint: String,
    endpoint_scheme: String,
    street: String,
    legal_name: String,
    legal_id: String,
}

#[derive(Debug, Default)]
struct LineFields {
    id: u32,
    quantity: Decimal,
    extension: Money,
    name: String,
    price: Decimal,
    unit_code: String,
    tax_category: Option<TaxCategory>,
    tax_percent: Decimal,
}

#[derive(Debug, Default)]
struct SubtotalFields {
    taxable: Money,
    tax: Money,
    category: Option<TaxCategory>,
    percent: Decimal,
    exemption_reason: String,
}

impl InvoiceFields {
    /// Close the line being parsed and keep it.
    fn finish_line(&mut self) {
        let line = std::mem::take(&mut self.line);
        self.lines.push(InvoiceLine {
            id: line.id,
            item_name: line.name,
            quantity: line.quantity,
            unit_price: line.price,
            line_extension_amount: line.extension,
            unit_code: line.unit_code,
            tax: line.tax_category.map(|category| LineTax {
                category,
                rate_percent: line.tax_percent,
            }),
        });
    }

    /// Close the tax subtotal being parsed; one without a category is dropped.
    fn finish_subtotal(&mut self) {
        if let Some(sub) = self.subtotal.take()
            && let Some(category) = sub.category
        {
            self.tax_subtotals.push(TaxSubtotal {
                category,
                rate_percent: sub.percent,
                taxable_amount: sub.taxable,
                tax_amount: sub.tax,
                exemption_reason: Some(sub.exemption_reason).filter(|r| !r.is_empty()),
            });
        }
    }

    /// Apply a completed leaf element's text. `parents` holds the element path
    /// below the root, excluding the leaf itself.
    ///
    /// Every field is scoped by its structural position rather than by element
    /// name alone. UBL reuses names across unrelated parts of the document —
    /// `cbc:ID` is both a tax category and a tax scheme, `cac:Party` is both a
    /// supplier and a delivery location — so matching on the name alone
    /// silently writes one part's value over another's.
    fn apply_element(
        &mut self,
        parents: &[String],
        name: &str,
        value: &str,
    ) -> Result<(), DocError> {
        let path = Parents::new(parents);

        if let Some(target) = path.accounting_party() {
            self.apply_party_field(target, &path, name, value);
            return Ok(());
        }
        if path.in_subtotal() {
            return self.apply_subtotal_field(&path, name, value);
        }
        if path.in_monetary_total() {
            match name {
                "LineExtensionAmount" => self.monetary.line_extension = parse_money(name, value)?,
                "TaxExclusiveAmount" => self.monetary.tax_exclusive = parse_money(name, value)?,
                "TaxInclusiveAmount" => self.monetary.tax_inclusive = parse_money(name, value)?,
                "PayableAmount" => self.monetary.payable = parse_money(name, value)?,
                _ => {}
            }
            return Ok(());
        }
        if path.invoice_line() {
            return self.apply_line_field(&path, name, value);
        }
        if path.in_payment_means() {
            let means = self.payment.get_or_insert_with(|| PaymentMeans {
                code: String::new(),
                payment_id: None,
                account_id: None,
            });
            match name {
                "PaymentMeansCode" => value.trim().clone_into(&mut means.code),
                "PaymentID" => means.payment_id = Some(value.to_string()),
                "ID" if path.direct_child_of("PayeeFinancialAccount") => {
                    means.account_id = Some(value.to_string());
                }
                _ => {}
            }
            return Ok(());
        }

        match name {
            "CustomizationID" if path.at_root() => {
                self.customization_id = value.to_string();
            }
            "ProfileID" if path.at_root() => self.profile_id = value.to_string(),
            // The document ID is a direct child of the root. `cbc:ID` also
            // appears on line items and inside every nested aggregate.
            "ID" if path.at_root() => self.id = value.to_string(),
            "IssueDate" => {
                self.issue_date =
                    Some(IssueDate::parse(value.trim(), DATE_FORMAT).map_err(|e| {
                        DocError::invalid_format(format!("bad IssueDate `{value}`: {e}"))
                    })?);
            }
            "InvoiceTypeCode" | "CreditNoteTypeCode" => {
                self.type_code = Some(InvoiceTypeCode::for_root(self.root_is_credit_note, value)?);
            }
            "DocumentCurrencyCode" => self.currency = value.to_string(),
            "BuyerReference" if path.at_root() => self.buyer_reference = value.to_string(),
            "DueDate" if path.at_root() => {
                self.due_date = Some(IssueDate::parse(value.trim(), DATE_FORMAT).map_err(|e| {
                    DocError::invalid_format(format!("bad DueDate `{value}`: {e}"))
                })?);
            }
            _ => {}
        }
        Ok(())
    }

    /// Apply a field scoped to an `InvoiceLine`/`CreditNoteLine`.
    fn apply_line_field(
        &mut self,
        path: &Parents<'_>,
        name: &str,
        value: &str,
    ) -> Result<(), DocError> {
        // The line's own `cbc:ID` is a direct child of the line; nested
        // aggregates (tax category, tax scheme) reuse the element name.
        let line_level = path.depth() == 1;
        match name {
            "ID" if path.direct_child_of("ClassifiedTaxCategory") => {
                let code = value.trim();
                self.line.tax_category = Some(TaxCategory::from_code(code).ok_or_else(|| {
                    DocError::invalid_format(format!("unknown tax category `{code}`"))
                })?);
            }
            "Percent" if path.direct_child_of("ClassifiedTaxCategory") => {
                self.line.tax_percent = parse_decimal(name, value)?;
            }
            "ID" if line_level => {
                // A missing or unparsable line ID falls back to position, which
                // is what a sender relying on document order would have meant.
                let fallback = u32::try_from(self.lines.len()).map_or(u32::MAX, |n| n + 1);
                self.line.id = value.trim().parse().unwrap_or(fallback);
            }
            "Name" => self.line.name = value.to_string(),
            "InvoicedQuantity" | "CreditedQuantity" => {
                self.line.quantity = parse_decimal(name, value)?;
                self.line.unit_code = self.leaf_attr.take().unwrap_or_default();
            }
            "LineExtensionAmount" => {
                self.line.extension = parse_money(name, value)?;
            }
            "PriceAmount" => self.line.price = parse_decimal(name, value)?,
            _ => {}
        }
        Ok(())
    }

    /// Route a party-scoped field to the supplier or customer accumulator.
    fn apply_party_field(&mut self, target: usize, path: &Parents<'_>, name: &str, value: &str) {
        let party = &mut self.parties[target];
        match name {
            "Name" => party.name = value.to_string(),
            "EndpointID" => {
                party.endpoint = value.to_string();
                party.endpoint_scheme = self.leaf_attr.take().unwrap_or_default();
            }
            "StreetName" => party.street = value.to_string(),
            "RegistrationName" => party.legal_name = value.to_string(),
            "CityName" => party.city = value.to_string(),
            "PostalZone" => party.postal_code = value.to_string(),
            "IdentificationCode" => party.country = value.to_string(),
            // `CompanyID` is the VAT/tax number. It appears under both
            // `PartyTaxScheme` and `PartyLegalEntity`, either of which is
            // correct for this field.
            "CompanyID" if path.direct_child_of("PartyLegalEntity") => {
                party.legal_id = value.to_string();
            }
            "CompanyID" if path.direct_child_of("PartyTaxScheme") => {
                party.tax_id = value.to_string();
            }
            _ => {}
        }
    }

    /// Apply a `TaxSubtotal`-scoped field.
    fn apply_subtotal_field(
        &mut self,
        path: &Parents<'_>,
        name: &str,
        value: &str,
    ) -> Result<(), DocError> {
        let subtotal = self.subtotal.get_or_insert_with(SubtotalFields::default);
        match name {
            // Only a direct child of `TaxCategory` is the category. The same
            // `cbc:ID` element name also appears under `TaxScheme`, whose value
            // ("VAT", "GST") is not a category and must not overwrite it.
            "ID" if path.direct_child_of("TaxCategory") => {
                let code = value.trim();
                subtotal.category = Some(TaxCategory::from_code(code).ok_or_else(|| {
                    DocError::invalid_format(format!("unknown tax category `{code}`"))
                })?);
            }
            "Percent" if path.direct_child_of("TaxCategory") => {
                subtotal.percent = parse_decimal(name, value)?;
            }
            "TaxExemptionReason" if path.direct_child_of("TaxCategory") => {
                subtotal.exemption_reason = value.to_string();
            }
            "TaxableAmount" => subtotal.taxable = parse_money(name, value)?,
            "TaxAmount" => subtotal.tax = parse_money(name, value)?,
            _ => {}
        }
        Ok(())
    }

    fn into_invoice(mut self) -> Result<Invoice, DocError> {
        let type_code = self
            .type_code
            .ok_or_else(|| DocError::invalid_format("missing InvoiceTypeCode"))?;
        let issue_date = self
            .issue_date
            .ok_or_else(|| DocError::invalid_format("missing IssueDate"))?;
        let to_party = |fields: &PartyFields| {
            let address = Address {
                street: Some(fields.street.clone()).filter(|c| !c.is_empty()),
                city: Some(fields.city.clone()).filter(|c| !c.is_empty()),
                postal_code: Some(fields.postal_code.clone()).filter(|c| !c.is_empty()),
                country_code: Some(fields.country.clone()).filter(|c| !c.is_empty()),
            };
            let address = if address.street.is_none()
                && address.city.is_none()
                && address.postal_code.is_none()
                && address.country_code.is_none()
            {
                None
            } else {
                Some(address)
            };
            Party {
                name: fields.name.clone(),
                address,
                tax_id: Some(fields.tax_id.clone()).filter(|c| !c.is_empty()),
                endpoint_id: Some(fields.endpoint.clone()).filter(|c| !c.is_empty()),
                endpoint_scheme: Some(fields.endpoint_scheme.clone()).filter(|c| !c.is_empty()),
                // The writer defaults `RegistrationName` to the trading name, so
                // only a differing value is real data.
                legal_name: Some(fields.legal_name.clone())
                    .filter(|n| !n.is_empty() && *n != fields.name),
                legal_id: Some(fields.legal_id.clone()).filter(|c| !c.is_empty()),
            }
        };
        let invoice = Invoice {
            customization_id: self.customization_id,
            profile_id: self.profile_id,
            id: self.id,
            type_code,
            issue_date,
            supplier: to_party(&self.parties[0]),
            customer: to_party(&self.parties[1]),
            lines: std::mem::take(&mut self.lines),
            tax_subtotals: std::mem::take(&mut self.tax_subtotals),
            monetary_total: self.monetary,
            currency: self.currency,
            buyer_reference: Some(self.buyer_reference).filter(|r| !r.is_empty()),
            due_date: self.due_date,
            payment_means: self.payment,
        };
        Ok(invoice)
    }
}

/// Parse a monetary or percentage value, refusing to guess.
///
/// Defaulting an unparsable amount to `0.0` turned a corrupt document into one
/// that parsed cleanly and then failed an arithmetic validation rule, sending
/// the reader after the wrong field.
///
/// # Errors
/// Returns [`DocError::InvalidFormat`] if `value` is not a finite number.
fn parse_money(name: &str, value: &str) -> Result<Money, DocError> {
    Money::parse(value).map_err(|e| DocError::invalid_format(format!("{name}: {e}")))
}

/// Parse a quantity, price or rate as an exact decimal.
///
/// # Errors
/// Returns [`DocError::InvalidFormat`] if `value` is not a plain decimal.
fn parse_decimal(name: &str, value: &str) -> Result<Decimal, DocError> {
    Decimal::parse(value).map_err(|e| DocError::invalid_format(format!("{name}: {e}")))
}

/// A view over an element's ancestor path, for structural matching.
struct Parents<'a> {
    names: &'a [String],
}

impl<'a> Parents<'a> {
    fn new(names: &'a [String]) -> Self {
        Self { names }
    }

    /// A direct child of the document root: with the root stripped from the
    /// path, that is an empty path.
    fn at_root(&self) -> bool {
        self.names.is_empty()
    }

    /// Inside `InvoiceLine` or `CreditNoteLine`, which are direct children of
    /// the root.
    fn invoice_line(&self) -> bool {
        const LINES: [&str; 2] = ["InvoiceLine", "CreditNoteLine"];
        LINES
            .iter()
            .any(|line| self.names.first().is_some_and(|n| n == line))
    }

    /// Number of ancestors below the root.
    fn depth(&self) -> usize {
        self.names.len()
    }

    /// Inside `PaymentMeans`.
    fn in_payment_means(&self) -> bool {
        self.names.iter().any(|n| n == "PaymentMeans")
    }

    /// Inside `TaxSubtotal`.
    fn in_subtotal(&self) -> bool {
        self.names.iter().any(|n| n == "TaxSubtotal")
    }

    /// Inside `LegalMonetaryTotal`.
    fn in_monetary_total(&self) -> bool {
        self.names.iter().any(|n| n == "LegalMonetaryTotal")
    }

    /// The leaf is an immediate child of `parent`.
    fn direct_child_of(&self, parent: &str) -> bool {
        self.names.last().is_some_and(|n| n == parent)
    }

    /// Which accounting party slot the path is inside, if any.
    ///
    /// Only a `cac:Party` that is a *direct child* of one of the two
    /// `Accounting*Party` wrappers counts. `DeliveryParty`, `PayeeParty`,
    /// `TaxRepresentativeParty` and a bare `cac:Party` elsewhere in the document
    /// are different things that happen to share the element name.
    fn accounting_party(&self) -> Option<usize> {
        let index = self.names.iter().position(|n| n == "Party")?;
        let parent = index.checked_sub(1).map(|i| self.names[i].as_str());
        match parent {
            Some("AccountingSupplierParty") => Some(0),
            Some("AccountingCustomerParty") => Some(1),
            _ => None,
        }
    }
}

/// Resolve an XML general or character reference (`&amp;`, `&#38;`) to text.
///
/// UBL documents define no custom entities, so only the five predefined
/// references and character references are accepted.
fn resolve_ref(r: &quick_xml::events::BytesRef<'_>) -> Result<String, DocError> {
    if r.is_char_ref() {
        return match r.resolve_char_ref() {
            Ok(Some(ch)) => Ok(ch.to_string()),
            _ => Err(DocError::invalid_format(format!(
                "malformed character reference `&{};`",
                r.as_ref()
            ))),
        };
    }
    match quick_xml::escape::resolve_predefined_entity(r) {
        Some(text) => Ok(text.to_owned()),
        None => Err(DocError::invalid_format(format!(
            "unknown entity reference `&{};`",
            r.as_ref()
        ))),
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::types::{Address, TaxCategory};

    fn party(name: &str) -> Party {
        Party {
            address: Some(Address {
                city: Some("Wellington".to_owned()),
                country_code: Some("NZ".to_owned()),
                ..Address::default()
            }),
            tax_id: Some("123456789".to_owned()),
            endpoint_id: Some("9429041234567".to_owned()),
            endpoint_scheme: Some("0088".to_owned()),
            legal_name: None,
            legal_id: Some("9429041234567".to_owned()),
            name: name.to_owned(),
        }
    }

    fn sample_invoice() -> Invoice {
        let date =
            time::Date::from_calendar_date(2026, time::Month::January, 15).expect("valid date");
        let mut inv = Invoice::new(
            "INV-001",
            InvoiceTypeCode::Invoice,
            date,
            party("Supplier Ltd"),
            party("Customer Ltd"),
        );
        inv.buyer_reference = Some("PO-1".to_owned());
        for line in [
            InvoiceLine::new(1, "Consulting", 2.into(), 150.into()).unwrap(),
            InvoiceLine::new(2, "Hosting", 12.into(), 10.into()).unwrap(),
        ] {
            inv.push_line(line.with_tax(TaxCategory::Standard, 15.into()));
        }
        inv.push_tax_subtotal(TaxSubtotal {
            category: TaxCategory::Standard,
            rate_percent: 15.into(),
            taxable_amount: Money::from_cents(42000),
            tax_amount: Money::from_cents(6300),
            exemption_reason: None,
        });
        inv
    }

    #[test]
    fn totals_recompute_from_lines_and_tax() {
        let inv = sample_invoice();
        assert_eq!(inv.monetary_total.tax_exclusive, Money::from_cents(42000));
        assert_eq!(inv.monetary_total.tax_inclusive, Money::from_cents(48300));
        assert_eq!(inv.total(), Money::from_cents(48300));
    }

    #[test]
    fn empty_invoice_has_zero_totals() {
        let date =
            time::Date::from_calendar_date(2026, time::Month::January, 15).expect("valid date");
        let inv = Invoice::new(
            "INV-002",
            InvoiceTypeCode::CreditNote,
            date,
            party("A"),
            party("B"),
        );
        assert_eq!(inv.total(), Money::from_cents(0));
        assert_eq!(inv.type_code.value(), 381);
    }

    #[test]
    fn valid_invoice_passes_validation() {
        let inv = sample_invoice();
        assert!(crate::validate::validate(&inv).is_empty());
    }

    #[test]
    fn xml_round_trip_preserves_all_fields() {
        let invoice = sample_invoice();
        let bytes = Invoice::to_xml(&invoice).expect("serialize");
        let restored = Invoice::from_xml(&bytes).expect("parse");
        assert_eq!(restored, invoice);
    }

    #[test]
    fn serialized_xml_uses_ubl_namespaces_and_peppol_ids() {
        let invoice = sample_invoice();
        let text = String::from_utf8(Invoice::to_xml(&invoice).expect("serialize")).expect("UTF-8");
        assert!(text.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
        assert!(text.contains("xmlns=\"urn:oasis:names:specification:ubl:schema:xsd:Invoice-2\""));
        assert!(
            text.contains("<cbc:CustomizationID>urn:cen.eu:en16931:2017</cbc:CustomizationID>")
        );
        assert!(text.contains("urn:fdc:peppol.eu:2017:poacc:billing:01:1.0"));
        assert!(text.contains("<cbc:InvoiceTypeCode>380</cbc:InvoiceTypeCode>"));
        assert!(text.contains("currencyID=\"NZD\">420.00</cbc:TaxableAmount>"));
    }

    #[test]
    fn credit_note_serializes_with_credit_note_root() {
        let date = time::Date::from_calendar_date(2026, time::Month::March, 1).expect("valid date");
        let mut credit = Invoice::new(
            "CN-1",
            InvoiceTypeCode::CreditNote,
            date,
            party("Supplier"),
            party("Customer"),
        );
        credit.push_line(InvoiceLine::new(1, "Refund", 1.into(), 50.into()).unwrap());
        let text = String::from_utf8(Invoice::to_xml(&credit).expect("serialize")).expect("UTF-8");
        assert!(text.contains("xsd:CreditNote-2"));
        // A `CreditNote` root carries `cbc:CreditNoteTypeCode`; the invoice
        // spelling is not valid UBL there.
        assert!(
            text.contains("<cbc:CreditNoteTypeCode>381</cbc:CreditNoteTypeCode>"),
            "{text}"
        );
        assert!(
            !text.contains("<cbc:InvoiceTypeCode>"),
            "an invoice type code must not appear on a credit note"
        );
        // Lines follow the document type too.
        assert!(text.contains("<cac:CreditNoteLine>"), "{text}");
        assert!(text.contains("<cbc:CreditedQuantity "), "{text}");
        assert!(!text.contains("<cac:InvoiceLine>"), "{text}");

        let restored = Invoice::from_xml(text.as_bytes()).expect("parse");
        assert_eq!(restored, credit, "credit note must round-trip intact");
    }

    /// `CreditNoteLine` was not in the parser's end-element handler, so every
    /// line of a credit note was silently dropped.
    #[test]
    fn credit_note_lines_are_parsed_back() {
        let date = time::Date::from_calendar_date(2026, time::Month::March, 1).expect("valid date");
        let mut credit = Invoice::new(
            "CN-2",
            InvoiceTypeCode::CreditNote,
            date,
            party("Supplier"),
            party("Customer"),
        );
        credit.push_line(InvoiceLine::new(1, "Refund A", 2.into(), 25.into()).unwrap());
        credit.push_line(InvoiceLine::new(2, "Refund B", 1.into(), 10.into()).unwrap());
        let xml = Invoice::to_xml(&credit).expect("serialize");
        let restored = Invoice::from_xml(&xml).expect("parse");
        assert_eq!(restored.lines.len(), 2, "both credit note lines survive");
        assert_eq!(restored.lines[0].item_name, "Refund A");
        assert_eq!(
            restored.lines[1].line_extension_amount,
            Money::from_cents(1000)
        );
    }

    /// A document type code is data the sender chose; codes outside the two
    /// PEPPOL ones must round-trip rather than being refused.
    #[test]
    fn other_document_type_codes_round_trip() {
        for code in [384u16, 386, 389, 261, 262, 393, 80] {
            let type_code = InvoiceTypeCode::from_code(&code.to_string()).expect("valid 1001 code");
            assert_eq!(type_code.value(), code, "code {code} preserved");
            assert!(!type_code.is_credit_note(), "code {code} is an invoice");
        }
        assert_eq!(
            InvoiceTypeCode::from_code("380").expect("380"),
            InvoiceTypeCode::Invoice
        );
        assert_eq!(
            InvoiceTypeCode::from_code("381").expect("381"),
            InvoiceTypeCode::CreditNote
        );
        // Outside the UN/EDIFACT 1001 range, or not a number at all.
        assert!(InvoiceTypeCode::from_code("0").is_err());
        assert!(InvoiceTypeCode::from_code("10000").is_err());
        assert!(InvoiceTypeCode::from_code("380X").is_err());
    }

    #[test]
    fn credit_note_round_trips_through_the_parser() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
<CreditNote xmlns="urn:oasis:names:specification:ubl:schema:xsd:CreditNote-2" xmlns:cac="urn:oasis:names:specification:ubl:schema:xsd:CommonAggregateComponents-2" xmlns:cbc="urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2">
  <cbc:ID>CN-EXT-1</cbc:ID>
  <cbc:IssueDate>2026-02-14</cbc:IssueDate>
  <cbc:CreditNoteTypeCode>384</cbc:CreditNoteTypeCode>
  <cbc:DocumentCurrencyCode>EUR</cbc:DocumentCurrencyCode>
  <cac:AccountingSupplierParty><cac:Party><cac:PartyName><cbc:Name>Supplier</cbc:Name></cac:PartyName></cac:Party></cac:AccountingSupplierParty>
  <cac:AccountingCustomerParty><cac:Party><cac:PartyName><cbc:Name>Customer</cbc:Name></cac:PartyName></cac:Party></cac:AccountingCustomerParty>
  <cac:LegalMonetaryTotal>
    <cbc:PayableAmount currencyID="EUR">-99.00</cbc:PayableAmount>
  </cac:LegalMonetaryTotal>
  <cac:CreditNoteLine>
    <cbc:ID>1</cbc:ID>
    <cbc:CreditedQuantity>3</cbc:CreditedQuantity>
    <cbc:LineExtensionAmount currencyID="EUR">-99.00</cbc:LineExtensionAmount>
    <cac:Item><cbc:Name>Returned goods</cbc:Name></cac:Item>
    <cac:Price><cbc:PriceAmount currencyID="EUR">-33.00</cbc:PriceAmount></cac:Price>
  </cac:CreditNoteLine>
</CreditNote>"#;
        let invoice = Invoice::from_xml(xml).expect("parse credit note");
        assert_eq!(invoice.id, "CN-EXT-1");
        // The `CreditNote` root is what decides the document kind; a
        // self-billed sub-code must not turn it back into an invoice, or
        // re-serializing would emit invoice line elements.
        assert_eq!(invoice.type_code, InvoiceTypeCode::CreditNote);
        assert!(invoice.type_code.is_credit_note());
        assert_eq!(invoice.lines.len(), 1);
        assert_eq!(invoice.lines[0].quantity, Decimal::from(3));
        assert_eq!(invoice.lines[0].item_name, "Returned goods");

        // Round-tripping keeps the credit note a credit note.
        let reserialized = String::from_utf8(invoice.to_xml().expect("serialize")).expect("UTF-8");
        assert!(
            reserialized.contains("<cac:CreditNoteLine>"),
            "{reserialized}"
        );
        assert!(
            reserialized.contains("<cbc:CreditedQuantity "),
            "{reserialized}"
        );
    }

    /// An invoice-rooted document keeps whatever 1001 code it carried.
    #[test]
    fn an_invoice_keeps_its_own_type_code() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Invoice xmlns="urn:oasis:names:specification:ubl:schema:xsd:Invoice-2" xmlns:cac="urn:oasis:names:specification:ubl:schema:xsd:CommonAggregateComponents-2" xmlns:cbc="urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2">
  <cbc:ID>INV-PREPAY</cbc:ID>
  <cbc:IssueDate>2026-01-15</cbc:IssueDate>
  <cbc:InvoiceTypeCode>386</cbc:InvoiceTypeCode>
  <cbc:DocumentCurrencyCode>NZD</cbc:DocumentCurrencyCode>
  <cac:AccountingSupplierParty><cac:Party><cac:PartyName><cbc:Name>Supplier</cbc:Name></cac:PartyName></cac:Party></cac:AccountingSupplierParty>
  <cac:AccountingCustomerParty><cac:Party><cac:PartyName><cbc:Name>Customer</cbc:Name></cac:PartyName></cac:Party></cac:AccountingCustomerParty>
</Invoice>"#;
        let invoice = Invoice::from_xml(xml.as_bytes()).expect("parse");
        assert_eq!(invoice.type_code, InvoiceTypeCode::Other(386));
        assert!(!invoice.type_code.is_credit_note());
        // And it survives a round trip.
        let back = String::from_utf8(invoice.to_xml().expect("serialize")).expect("UTF-8");
        assert!(
            back.contains("<cbc:InvoiceTypeCode>386</cbc:InvoiceTypeCode>"),
            "{back}"
        );
        assert!(!back.contains("CreditNote"), "{back}");
    }

    /// A control character cannot be represented in XML. Writing it raw yields an
    /// invoice that PEPPOL receivers reject, so serialization must fail instead.
    #[test]
    fn illegal_control_characters_are_rejected_not_written_raw() {
        let date =
            time::Date::from_calendar_date(2026, time::Month::January, 15).expect("valid date");

        // In the supplier name.
        let invoice = Invoice::new(
            "INV-CTRL",
            InvoiceTypeCode::Invoice,
            date,
            Party::new("Bad\u{1}Supplier"),
            Party::new("Customer"),
        );
        let err = invoice.to_xml().expect_err("U+0001 in a party name");
        assert!(err.to_string().contains("U+0001"), "{err}");

        // In a line item name.
        let mut invoice = Invoice::new(
            "INV-CTRL2",
            InvoiceTypeCode::Invoice,
            date,
            Party::new("Supplier"),
            Party::new("Customer"),
        );
        invoice.push_line(InvoiceLine::new(1, "Bad\u{2}Item", 1.into(), 10.into()).unwrap());
        let err = invoice.to_xml().expect_err("U+0002 in a line name");
        assert!(err.to_string().contains("U+0002"), "{err}");
    }

    #[test]
    fn tab_newline_and_return_are_accepted() {
        let date =
            time::Date::from_calendar_date(2026, time::Month::January, 15).expect("valid date");
        let invoice = Invoice::new(
            "INV-WS",
            InvoiceTypeCode::Invoice,
            date,
            Party::new("a\tb\nc\rd"),
            Party::new("Customer"),
        );
        assert!(invoice.to_xml().is_ok(), "these controls are legal XML");
    }

    #[test]
    fn non_invoice_xml_is_rejected() {
        assert!(Invoice::from_xml(b"<Order/>").is_err());
        assert!(Invoice::from_xml(b"not xml").is_err());
    }
}

/// Factur-X/ZUGFeRD hybrid PDF+XML support: the UBL invoice XML is attached
/// to a PDF document as an associated file (PDF 2.0 §7.11.3), the mechanism
/// `ZUGFeRD` 2.x and `Factur-X` 1.0 use for e-invoice embedding.
///
/// Full `Factur-X` compliance additionally requires PDF/A-3 with the
/// Factur-X XMP extension schema; this module implements the attachment
/// mechanism and naming conventions, with PDF/A conformance left to the
/// caller.
#[cfg(feature = "facturx")]
pub mod facturx {
    use super::Invoice;
    use tpt_doc_core::DocError;
    use tpt_doc_pdf::Document;

    /// The conventional file name for the embedded invoice XML.
    pub const FILE_NAME: &str = "factur-x.xml";

    /// Attach this invoice's UBL XML to a PDF document.
    ///
    /// The XML produced by [`Invoice::to_xml`] is embedded under
    /// `factur-x.xml` with `/AFRelationship /Data` and registered in the
    /// catalog's `/AF` array and `/Names /EmbeddedFiles` tree.
    ///
    /// # Errors
    /// Returns [`DocError`] if XML serialization fails.
    pub fn attach_xml(invoice: &Invoice, document: &mut Document) -> Result<(), DocError> {
        let xml = invoice.to_xml()?;
        document.attach_file(FILE_NAME, "text/xml", xml);
        Ok(())
    }
}
