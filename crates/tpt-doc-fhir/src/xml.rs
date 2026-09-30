//! Shared helpers for FHIR XML serialization.
//!
//! FHIR XML uses the default namespace `http://hl7.org/fhir`; primitive
//! values are carried in a `value` attribute on an element named after the
//! property (e.g. `<birthDate value="1970-01-01"/>`).

use std::fmt::Write as _;

use quick_xml::XmlVersion;
use quick_xml::events::BytesStart;
/// The FHIR XML namespace URI.
pub(crate) const FHIR_NS: &str = "http://hl7.org/fhir";

/// The XML version assumed when normalizing attribute values.
pub(crate) const XML_VERSION: XmlVersion = XmlVersion::Implicit1_0;

/// Append `<{name} value="…"/>` with the text escaped.
pub(crate) fn value_element(xml: &mut String, name: &str, value: &str) {
    let escaped = quick_xml::escape::escape(value);
    let _ = write!(xml, r#"<{name} value="{escaped}"/>"#);
}

/// Append a child element open tag: `<{name}>`.
pub(crate) fn open_element(xml: &mut String, name: &str) {
    let _ = write!(xml, "<{name}>");
}

/// Append a child element close tag: `</{name}>`.
pub(crate) fn close_element(xml: &mut String, name: &str) {
    let _ = write!(xml, "</{name}>");
}

/// Read a `value` attribute (or any named attribute) off a start/empty tag.
pub(crate) fn attr_value(start: &BytesStart<'_>, name: &str) -> Option<String> {
    start
        .attributes()
        .filter_map(std::result::Result::ok)
        .find(|a| a.key.as_ref() == name)
        .and_then(|a| {
            a.normalized_value(XML_VERSION)
                .ok()
                .map(std::borrow::Cow::into_owned)
        })
}

/// Normalize an incoming local element name: strip any namespace prefix and
/// require it to match `expected`.
pub(crate) fn is_element(start: &BytesStart<'_>, expected: &str) -> bool {
    start.local_name().as_ref() == expected
}
