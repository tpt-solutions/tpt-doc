//! Shared helpers for FHIR XML serialization.
//!
//! FHIR XML uses the default namespace `http://hl7.org/fhir`; primitive
//! values are carried in a `value` attribute on an element named after the
//! property (e.g. `<birthDate value="1970-01-01"/>`).

use std::fmt::Write as _;

use quick_xml::XmlVersion;
use quick_xml::events::BytesStart;
use tpt_doc_core::DocError;
/// The FHIR XML namespace URI.
pub(crate) const FHIR_NS: &str = "http://hl7.org/fhir";

/// The XML version assumed when normalizing attribute values.
pub(crate) const XML_VERSION: XmlVersion = XmlVersion::Implicit1_0;

/// Append `<{name} value="…"/>` with the text escaped.
///
/// # Errors
/// Returns [`DocError::InvalidFormat`] if `value` contains a character XML
/// 1.0 forbids; FHIR XML must be well-formed, and a raw control character
/// makes it not.
pub(crate) fn value_element(xml: &mut String, name: &str, value: &str) -> Result<(), DocError> {
    let escaped = tpt_doc_core::escape_xml_text(value)?;
    let _ = write!(xml, r#"<{name} value="{escaped}"/>"#);
    Ok(())
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
/// The local (namespace-free) name of a start or empty tag, as an owned string.
pub(crate) fn local_name(start: &BytesStart<'_>) -> String {
    let local = start.local_name();
    let name: &str = local.as_ref();
    name.to_owned()
}

/// Reads one resource document by element position.
///
/// The driver ([`read_resource`]) keeps the stack of open element names and
/// enforces the root element, the depth limit, and well-formedness. A draft
/// receives `open` for every element with the path from the root down to and
/// including it, and `close` when that element ends.
pub(crate) trait DraftReader {
    /// An element has started (or is empty) at `path`.
    fn open(
        &mut self,
        path: &[String],
        start: &BytesStart<'_>,
    ) -> Result<(), crate::validate::FhirValidationError>;

    /// The element at `path` has ended.
    fn close(&mut self, path: &[String]);
}

/// Read a resource document into `draft`, requiring the root element `root`.
///
/// # Errors
/// Returns an error if the root is a different resource, the document is not
/// well formed, it nests deeper than `limits.max_xml_depth`, or an element is
/// never closed.
pub(crate) fn read_resource<D: DraftReader>(
    bytes: &[u8],
    limits: &tpt_doc_core::Limits,
    root: &str,
    draft: &mut D,
) -> Result<(), crate::validate::FhirValidationError> {
    use crate::validate::FhirValidationError;
    use quick_xml::events::Event;

    let fail = |msg: String| FhirValidationError::new(format!("/f:{root}"), msg);
    let mut reader = quick_xml::Reader::from_reader(bytes);
    let mut stack: Vec<String> = Vec::new();
    let mut saw_root = false;

    loop {
        match reader.read_event() {
            Ok(Event::Start(start)) => {
                push_element(&mut stack, &start, root, limits, &mut saw_root, &fail)?;
                draft.open(&stack, &start)?;
            }
            Ok(Event::Empty(start)) => {
                push_element(&mut stack, &start, root, limits, &mut saw_root, &fail)?;
                draft.open(&stack, &start)?;
                draft.close(&stack);
                stack.pop();
            }
            Ok(Event::End(_)) => {
                // quick-xml has already checked that the end tag matches the
                // element on top of the stack.
                draft.close(&stack);
                stack.pop();
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(e) => return Err(fail(format!("XML error: {e}"))),
        }
    }

    if !saw_root {
        return Err(fail(format!("no {root} element found")));
    }
    if let Some(open) = stack.last() {
        return Err(fail(format!("element <{open}> is never closed")));
    }
    Ok(())
}

fn push_element(
    stack: &mut Vec<String>,
    start: &BytesStart<'_>,
    root: &str,
    limits: &tpt_doc_core::Limits,
    saw_root: &mut bool,
    fail: &impl Fn(String) -> crate::validate::FhirValidationError,
) -> Result<(), crate::validate::FhirValidationError> {
    stack.push(local_name(start));
    limits
        .check_xml_depth(stack.len())
        .map_err(|e| fail(e.to_string()))?;
    if stack.len() == 1 {
        if stack[0] != root {
            return Err(fail(format!(
                "expected a {root} resource, found <{}>",
                stack[0]
            )));
        }
        *saw_root = true;
    }
    Ok(())
}

/// Borrow a path of owned element names as `&str`s for matching.
pub(crate) fn path_refs(path: &[String]) -> Vec<&str> {
    path.iter().map(String::as_str).collect()
}
