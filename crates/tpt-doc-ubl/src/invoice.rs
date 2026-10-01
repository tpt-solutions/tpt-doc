use quick_xml::escape::escape;
use serde::{Deserialize, Serialize};
use tpt_doc_core::DocError;

use crate::types::{
    Address, InvoiceLine, IssueDate, MonetaryTotal, Party, TaxCategory, TaxSubtotal, round2,
};

/// UBL document type code (`cbc:InvoiceTypeCode` / `cbc:CreditNoteTypeCode`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InvoiceTypeCode {
    /// `380` — commercial invoice.
    Invoice,
    /// `381` — credit note.
    CreditNote,
}

impl InvoiceTypeCode {
    /// The UBL type code value.
    #[must_use]
    pub fn value(&self) -> u16 {
        match self {
            Self::Invoice => 380,
            Self::CreditNote => 381,
        }
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
    pub fn total(&self) -> f64 {
        self.monetary_total.payable
    }

    /// Recompute `LegalMonetaryTotal` from lines and tax subtotals:
    ///
    /// - `tax_exclusive` = Σ line net amounts (BR-CO-10)
    /// - `tax_inclusive` = `tax_exclusive` + Σ tax amounts (BR-CO-13)
    /// - `payable` = `tax_inclusive` (BR-CO-15)
    fn recalculate_totals(&mut self) {
        let tax_exclusive = round2(self.lines.iter().map(|l| l.line_extension_amount).sum());
        let tax_amount = round2(self.tax_subtotals.iter().map(|t| t.tax_amount).sum());
        let tax_inclusive = round2(tax_exclusive + tax_amount);
        self.monetary_total = MonetaryTotal {
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

        let root = match self.type_code {
            InvoiceTypeCode::Invoice => "Invoice",
            InvoiceTypeCode::CreditNote => "CreditNote",
        };
        let mut xml = String::with_capacity(2048);
        let _ = write!(xml, r#"<?xml version="1.0" encoding="UTF-8"?>"#);
        let _ = write!(
            xml,
            r#"<{root} xmlns="urn:oasis:names:specification:ubl:schema:xsd:{root}-2" xmlns:cac="urn:oasis:names:specification:ubl:schema:xsd:CommonAggregateComponents-2" xmlns:cbc="urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2">"#
        );
        cbc(&mut xml, "CustomizationID", &self.customization_id);
        cbc(&mut xml, "ProfileID", &self.profile_id);
        cbc(&mut xml, "ID", &self.id);
        let issue_date = self
            .issue_date
            .format(&DATE_FORMAT)
            .map_err(|e| DocError::invalid_format(format!("IssueDate formatting failed: {e}")))?;
        cbc(&mut xml, "IssueDate", &issue_date);
        cbc(
            &mut xml,
            "InvoiceTypeCode",
            &self.type_code.value().to_string(),
        );
        cbc(&mut xml, "DocumentCurrencyCode", &self.currency);

        party_block(
            &mut xml,
            "AccountingSupplierParty",
            &self.supplier,
            &self.currency,
        );
        party_block(
            &mut xml,
            "AccountingCustomerParty",
            &self.customer,
            &self.currency,
        );

        let tax_total = round2(self.tax_subtotals.iter().map(|t| t.tax_amount).sum());
        xml.push_str("<cac:TaxTotal>");
        cbc_amount(&mut xml, "TaxAmount", tax_total, &self.currency);
        for subtotal in &self.tax_subtotals {
            write_tax_subtotal(&mut xml, subtotal, &self.currency);
        }
        xml.push_str("</cac:TaxTotal>");

        xml.push_str("<cac:LegalMonetaryTotal>");
        cbc_amount(
            &mut xml,
            "TaxExclusiveAmount",
            self.monetary_total.tax_exclusive,
            &self.currency,
        );
        cbc_amount(
            &mut xml,
            "TaxInclusiveAmount",
            self.monetary_total.tax_inclusive,
            &self.currency,
        );
        cbc_amount(
            &mut xml,
            "PayableAmount",
            self.monetary_total.payable,
            &self.currency,
        );
        xml.push_str("</cac:LegalMonetaryTotal>");

        for line in &self.lines {
            xml.push_str("<cac:InvoiceLine>");
            cbc(&mut xml, "ID", &line.id.to_string());
            let quantity_text = format!("{}", line.quantity);
            let quantity = escape(&quantity_text);
            let _ = write!(
                xml,
                "<cbc:InvoicedQuantity>{quantity}</cbc:InvoicedQuantity>"
            );
            cbc_amount(
                &mut xml,
                "LineExtensionAmount",
                line.line_extension_amount,
                &self.currency,
            );
            xml.push_str("<cac:Item>");
            cbc(&mut xml, "Name", &line.item_name);
            xml.push_str("</cac:Item>");
            xml.push_str("<cac:Price>");
            cbc_amount(&mut xml, "PriceAmount", line.unit_price, &self.currency);
            xml.push_str("</cac:Price>");
            xml.push_str("</cac:InvoiceLine>");
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
                    if path.is_empty() {
                        match name.as_str() {
                            "Invoice" => fields.type_code = Some(InvoiceTypeCode::Invoice),
                            "CreditNote" => fields.type_code = Some(InvoiceTypeCode::CreditNote),
                            other => {
                                return Err(DocError::invalid_format(format!(
                                    "unexpected root element `{other}`"
                                )));
                            }
                        }
                    } else {
                        match name.as_str() {
                            "AccountingSupplierParty" => fields.party_target = 0,
                            "AccountingCustomerParty" => fields.party_target = 1,
                            _ => {}
                        }
                    }
                    path.push(name);
                    // A new element starts a fresh text buffer.
                    pending_text.clear();
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
                    pending_text.push_str(&resolve_ref(&reference)?);
                }
                Ok(Event::End(end)) => {
                    let name = end.local_name().as_ref().to_owned();
                    let leaf_path = path.clone();
                    path.pop();
                    fields.apply_element(&path, &name, &leaf_path, &pending_text)?;
                    pending_text.clear();
                    match name.as_str() {
                        "InvoiceLine" => {
                            let line = std::mem::take(&mut fields.line);
                            fields.lines.push(InvoiceLine {
                                id: line.id,
                                item_name: line.name,
                                quantity: line.quantity,
                                unit_price: line.price,
                                line_extension_amount: line.extension,
                            });
                        }
                        "TaxSubtotal" => {
                            if let Some(sub) = fields.subtotal.take()
                                && let Some(category) = sub.category
                            {
                                fields.tax_subtotals.push(TaxSubtotal {
                                    category,
                                    rate_percent: sub.percent,
                                    taxable_amount: sub.taxable,
                                    tax_amount: sub.tax,
                                });
                            }
                        }
                        _ => {}
                    }
                }
                Ok(Event::Eof) => break,
                Ok(_) => {}
                Err(e) => return Err(DocError::invalid_format(format!("XML error: {e}"))),
            }
        }
        fields.into_invoice()
    }
}

/// `cbc:IssueDate` carries a bare ISO calendar date (`YYYY-MM-DD`).
const DATE_FORMAT: &[time::format_description::BorrowedFormatItem] =
    time::macros::format_description!("[year]-[month]-[day]");

/// Append `<cbc:{name}>{escaped}</cbc:{name}>`.
fn cbc(xml: &mut String, name: &str, value: &str) {
    use std::fmt::Write as _;
    let escaped = escape(value);
    let _ = write!(xml, "<cbc:{name}>{escaped}</cbc:{name}>");
}

/// Append `<cbc:{name} currencyID="{ccy}">{amount:.2}</cbc:{name}>`.
fn cbc_amount(xml: &mut String, name: &str, amount: f64, currency: &str) {
    use std::fmt::Write as _;
    let _ = write!(
        xml,
        "<cbc:{name} currencyID=\"{}\">{:.2}</cbc:{name}>",
        escape(currency),
        round2(amount)
    );
}

/// Emit one `<cac:TaxSubtotal>` block with its tax category.
fn write_tax_subtotal(xml: &mut String, subtotal: &TaxSubtotal, currency: &str) {
    xml.push_str("<cac:TaxSubtotal>");
    cbc_amount(xml, "TaxableAmount", subtotal.taxable_amount, currency);
    cbc_amount(xml, "TaxAmount", subtotal.tax_amount, currency);
    xml.push_str("<cac:TaxCategory>");
    cbc(xml, "ID", subtotal.category.code());
    let percent = format!("{}", subtotal.rate_percent);
    cbc(xml, "Percent", &percent);
    xml.push_str("</cac:TaxCategory>");
    xml.push_str("</cac:TaxSubtotal>");
}

/// Emit an `Accounting{Supplier,Customer}Party` block.
fn party_block(xml: &mut String, wrapper: &str, party: &Party, currency: &str) {
    use std::fmt::Write as _;
    let _ = write!(xml, "<cac:{wrapper}><cac:Party>");
    if let Some(endpoint) = &party.endpoint_id {
        cbc(xml, "EndpointID", endpoint);
    }
    xml.push_str("<cac:PartyName>");
    cbc(xml, "Name", &party.name);
    xml.push_str("</cac:PartyName>");
    if let Some(address) = &party.address {
        xml.push_str("<cac:PostalAddress>");
        if let Some(city) = &address.city {
            cbc(xml, "CityName", city);
        }
        if let Some(postal) = &address.postal_code {
            cbc(xml, "PostalZone", postal);
        }
        if let Some(country) = &address.country_code {
            xml.push_str("<cac:Country>");
            cbc(xml, "IdentificationCode", country);
            xml.push_str("</cac:Country>");
        }
        xml.push_str("</cac:PostalAddress>");
    }
    if let Some(tax_id) = &party.tax_id {
        xml.push_str("<cac:PartyTaxScheme>");
        cbc(xml, "CompanyID", tax_id);
        let _ = currency;
        xml.push_str("</cac:PartyTaxScheme>");
    }
    let _ = write!(xml, "</cac:Party></cac:{wrapper}>");
}

/// Scratch state accumulated while parsing an UBL invoice document.
#[derive(Debug, Default)]
struct InvoiceFields {
    customization_id: String,
    profile_id: String,
    id: String,
    type_code: Option<InvoiceTypeCode>,
    issue_date: Option<IssueDate>,
    currency: String,
    party_target: usize, // 0 = supplier, 1 = customer
    parties: [PartyFields; 2],
    line: LineFields,
    lines: Vec<InvoiceLine>,
    tax_subtotals: Vec<TaxSubtotal>,
    subtotal: Option<SubtotalFields>,
    monetary: MonetaryTotal,
}

#[derive(Debug, Default)]
struct PartyFields {
    name: String,
    city: String,
    postal_code: String,
    country: String,
    tax_id: String,
    endpoint: String,
}

#[derive(Debug, Default)]
struct LineFields {
    id: u32,
    quantity: f64,
    extension: f64,
    name: String,
    price: f64,
}

#[derive(Debug, Default)]
struct SubtotalFields {
    taxable: f64,
    tax: f64,
    category: Option<TaxCategory>,
    percent: f64,
}

impl InvoiceFields {
    /// Apply a completed leaf element's text. `parents` excludes the leaf
    /// and the root; `full_path` retains the leaf for context checks.
    fn apply_element(
        &mut self,
        parents: &[String],
        name: &str,
        full_path: &[String],
        value: &str,
    ) -> Result<(), DocError> {
        let path = full_path;
        let in_line = path.iter().any(|p| p == "InvoiceLine");
        let in_party = path.iter().any(|p| p == "Party");
        let in_subtotal = path.iter().any(|p| p == "TaxSubtotal");
        let amount = value.trim().parse::<f64>().ok();

        if in_party {
            self.apply_party_field(name, value);
            return Ok(());
        }
        if in_subtotal {
            self.apply_subtotal_field(name, value, amount);
            return Ok(());
        }
        if path.iter().any(|p| p == "LegalMonetaryTotal") {
            match name {
                "TaxExclusiveAmount" => self.monetary.tax_exclusive = amount.unwrap_or(0.0),
                "TaxInclusiveAmount" => self.monetary.tax_inclusive = amount.unwrap_or(0.0),
                "PayableAmount" => self.monetary.payable = amount.unwrap_or(0.0),
                _ => {}
            }
            return Ok(());
        }

        match name {
            "CustomizationID" => self.customization_id = value.to_string(),
            "ProfileID" => self.profile_id = value.to_string(),
            "ID" if in_line => {
                #[allow(clippy::cast_possible_truncation)] // line counts are tiny
                let fallback = self.lines.len() as u32 + 1;
                self.line.id = value.trim().parse().unwrap_or(fallback);
            }
            "ID" if parents.len() <= 1 => self.id = value.to_string(),
            "IssueDate" => {
                self.issue_date =
                    Some(IssueDate::parse(value.trim(), DATE_FORMAT).map_err(|e| {
                        DocError::invalid_format(format!("bad IssueDate `{value}`: {e}"))
                    })?);
            }
            "InvoiceTypeCode" | "CreditNoteTypeCode" => {
                self.type_code = match value.trim() {
                    "380" => Some(InvoiceTypeCode::Invoice),
                    "381" => Some(InvoiceTypeCode::CreditNote),
                    other => {
                        return Err(DocError::invalid_format(format!(
                            "unsupported type code `{other}`"
                        )));
                    }
                };
            }
            "DocumentCurrencyCode" => self.currency = value.to_string(),
            "Name" if in_line => self.line.name = value.to_string(),
            "InvoicedQuantity" | "CreditedQuantity" if in_line => {
                self.line.quantity = amount.unwrap_or(0.0);
            }
            "LineExtensionAmount" if in_line => {
                self.line.extension = amount.unwrap_or(0.0);
            }
            "PriceAmount" if in_line => {
                self.line.price = amount.unwrap_or(0.0);
            }
            _ => {}
        }
        Ok(())
    }

    /// Route a party-scoped field to the supplier or customer accumulator.
    fn apply_party_field(&mut self, name: &str, value: &str) {
        let target = self.party_target;
        let field = match name {
            "Name" => Some(("name", value.to_string())),
            "EndpointID" => Some(("endpoint", value.to_string())),
            "CompanyID" => Some(("tax_id", value.to_string())),
            "CityName" => Some(("city", value.to_string())),
            "PostalZone" => Some(("postal_code", value.to_string())),
            "IdentificationCode" => Some(("country", value.to_string())),
            _ => None,
        };
        let Some((field, value)) = field else {
            return;
        };
        let party = &mut self.parties[target];
        match field {
            "name" => party.name = value,
            "endpoint" => party.endpoint = value,
            "tax_id" => party.tax_id = value,
            "city" => party.city = value,
            "postal_code" => party.postal_code = value,
            "country" => party.country = value,
            _ => {}
        }
    }

    /// Apply a `TaxSubtotal`-scoped field; `value` is the raw element text.
    fn apply_subtotal_field(&mut self, name: &str, value: &str, amount: Option<f64>) {
        let subtotal = self.subtotal.get_or_insert_with(SubtotalFields::default);
        match name {
            "TaxableAmount" => subtotal.taxable = amount.unwrap_or(0.0),
            "TaxAmount" => subtotal.tax = amount.unwrap_or(0.0),
            "Percent" => subtotal.percent = amount.unwrap_or(0.0),
            "ID" => subtotal.category = TaxCategory::from_code(value.trim()),
            _ => {}
        }
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
                street: None,
                city: Some(fields.city.clone()).filter(|c| !c.is_empty()),
                postal_code: Some(fields.postal_code.clone()).filter(|c| !c.is_empty()),
                country_code: Some(fields.country.clone()).filter(|c| !c.is_empty()),
            };
            let address = if address.city.is_none()
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
        };
        Ok(invoice)
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
    // Monetary arithmetic is rounded to 2 decimal places; exact f64
    // comparison is intentional.
    #![allow(clippy::float_cmp)]

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
            endpoint_id: Some("0208:942703567".to_owned()),
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
        inv.push_line(InvoiceLine::new(1, "Consulting", 2.0, 150.0));
        inv.push_line(InvoiceLine::new(2, "Hosting", 12.0, 10.0));
        inv.push_tax_subtotal(TaxSubtotal {
            category: TaxCategory::Standard,
            rate_percent: 15.0,
            taxable_amount: 420.0,
            tax_amount: 63.0,
        });
        inv
    }

    #[test]
    fn totals_recompute_from_lines_and_tax() {
        let inv = sample_invoice();
        assert_eq!(inv.monetary_total.tax_exclusive, 420.0);
        assert_eq!(inv.monetary_total.tax_inclusive, 483.0);
        assert_eq!(inv.total(), 483.0);
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
        assert_eq!(inv.total(), 0.0);
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
        let credit = Invoice::new(
            "CN-1",
            InvoiceTypeCode::CreditNote,
            date,
            party("Supplier"),
            party("Customer"),
        );
        let text = String::from_utf8(Invoice::to_xml(&credit).expect("serialize")).expect("UTF-8");
        assert!(text.contains("xsd:CreditNote-2"));
        assert!(text.contains("<cbc:InvoiceTypeCode>381</cbc:InvoiceTypeCode>"));
        let restored = Invoice::from_xml(text.as_bytes()).expect("parse");
        assert_eq!(restored, credit);
    }

    #[test]
    fn non_invoice_xml_is_rejected() {
        assert!(Invoice::from_xml(b"<Order/>").is_err());
        assert!(Invoice::from_xml(b"not xml").is_err());
    }
}
