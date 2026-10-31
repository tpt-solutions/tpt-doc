//! Structural diff for the document formats in this workspace.
//!
//! A text diff of two HL7 messages or two FHIR bundles is noise: a field moves
//! one column, a JSON key is reordered, a timestamp differs. This crate parses
//! both sides and reports **which element changed**, by a path that names it:
//!
//! | Format | Path | Example |
//! |---|---|---|
//! | HL7 v2 | `SEG[occurrence]-field[~repeat][.component]` | `PID[0]-5.2` |
//! | EDIFACT / X12 | `SEG[occurrence]-element` | `NAD[1]-3` |
//! | FHIR / JSON | JSON pointer | `/entry/0/resource/birthDate` |
//! | XLSX | cell reference (first sheet) | `B3` |
//! | DOCX | paragraph index | `/p/4` |
//!
//! Changes carry both values, so a report reads `PID[0]-5.1: Smith → Smyth`.
//! Use [`Options::ignore`] to skip volatile elements (`MSH-7`, `MSH-10`).
//!
//! # Examples
//!
//! ```
//! use tpt_doc_diff::{diff_hl7, Options};
//!
//! let a = b"MSH|^~\\&|A|B|C|D|20260101||ADT^A01|1|P|2.5.1\rPID|1||9||Smith^John";
//! let b = b"MSH|^~\\&|A|B|C|D|20260102||ADT^A01|2|P|2.5.1\rPID|1||9||Smyth^John";
//! let d = diff_hl7(a, b, &Options::default().ignoring("MSH"))?;
//! assert_eq!(d.changes.len(), 1);
//! assert_eq!(d.changes[0].path, "PID[0]-5.1");
//! # Ok::<(), tpt_doc_core::DocError>(())
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::collections::HashSet;
use std::fmt;

use serde_json::Value;
use tpt_doc_core::DocError;

/// How an element differs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    /// Only present on the right.
    Added,
    /// Only present on the left.
    Removed,
    /// Present on both sides with different values.
    Changed,
}

/// One difference between two documents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Change {
    /// Where the difference is (format-specific, see the crate docs).
    pub path: String,
    /// Added, removed or changed.
    pub kind: ChangeKind,
    /// The value on the left, when there is one.
    pub left: Option<String>,
    /// The value on the right, when there is one.
    pub right: Option<String>,
}

impl fmt::Display for Change {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (&self.kind, &self.left, &self.right) {
            (ChangeKind::Added, _, Some(r)) => write!(f, "+ {}: {r}", self.path),
            (ChangeKind::Removed, Some(l), _) => write!(f, "- {}: {l}", self.path),
            (_, Some(l), Some(r)) => write!(f, "~ {}: {l} \u{2192} {r}", self.path),
            _ => write!(f, "? {}", self.path),
        }
    }
}

/// The differences between two documents, in document order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Diff {
    /// Every difference found.
    pub changes: Vec<Change>,
}

impl Diff {
    /// `true` when the documents are structurally identical.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }
}

impl fmt::Display for Diff {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for change in &self.changes {
            writeln!(f, "{change}")?;
        }
        Ok(())
    }
}

/// Diff options.
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// Paths starting with any of these are skipped (`"MSH"` skips the whole
    /// header, `"MSH[0]-7"` only the timestamp, `"/id"` a JSON `id`).
    pub ignore: Vec<String>,
}

impl Options {
    /// Add a path prefix to skip.
    #[must_use]
    pub fn ignoring(mut self, prefix: &str) -> Self {
        self.ignore.push(prefix.to_owned());
        self
    }

    fn skips(&self, path: &str) -> bool {
        self.ignore.iter().any(|p| path.starts_with(p.as_str()))
    }
}

type Flat = Vec<(String, String)>;

/// Compare two flattened documents by path.
fn compare(mut left: Flat, mut right: Flat, options: &Options) -> Diff {
    align(&mut left, &mut right);
    let right_map: std::collections::HashMap<&str, &str> = right
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();
    let left_keys: HashSet<&str> = left.iter().map(|(k, _)| k.as_str()).collect();
    let mut changes = Vec::new();
    for (path, value) in &left {
        if options.skips(path) {
            continue;
        }
        match right_map.get(path.as_str()) {
            None => changes.push(Change {
                path: path.clone(),
                kind: ChangeKind::Removed,
                left: Some(value.clone()),
                right: None,
            }),
            Some(other) if other != value => changes.push(Change {
                path: path.clone(),
                kind: ChangeKind::Changed,
                left: Some(value.clone()),
                right: Some((*other).to_owned()),
            }),
            Some(_) => {}
        }
    }
    for (path, value) in &right {
        if !left_keys.contains(path.as_str()) && !options.skips(path) {
            changes.push(Change {
                path: path.clone(),
                kind: ChangeKind::Added,
                left: None,
                right: Some(value.clone()),
            });
        }
    }
    Diff { changes }
}

/// A field that is `X` on one side and `X.1` / `X~1` on the other is the same
/// element once it gains or loses a component or repeat: rename so it compares.
fn align(a: &mut Flat, b: &mut Flat) {
    for suffix in ["~1", ".1"] {
        rename_towards(a, b, suffix);
        rename_towards(b, a, suffix);
    }
}

fn rename_towards(x: &mut Flat, y: &Flat, suffix: &str) {
    let y_keys: HashSet<&str> = y.iter().map(|(k, _)| k.as_str()).collect();
    for (key, _) in x.iter_mut() {
        if !y_keys.contains(key.as_str()) && y_keys.contains(format!("{key}{suffix}").as_str()) {
            key.push_str(suffix);
        }
    }
}

// ---------------------------------------------------------------------------
// Segment-based formats (HL7 v2, EDIFACT, X12)
// ---------------------------------------------------------------------------

/// A parsed segment: tag plus raw fields, for the segment-oriented formats.
struct Seg {
    tag: String,
    fields: Vec<String>,
    /// Splits one field into repeats and components (HL7 only).
    separators: Option<(char, char)>,
    /// Renders the whole segment (used when a segment is added or removed).
    raw: String,
}

/// Number each segment's occurrence among segments with the same tag.
fn keyed(segments: &[Seg]) -> Vec<(String, &Seg)> {
    let mut seen: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    segments
        .iter()
        .map(|s| {
            let n = seen.entry(s.tag.as_str()).or_insert(0);
            let key = format!("{}[{n}]", s.tag);
            *n += 1;
            (key, s)
        })
        .collect()
}

fn flatten_segment(key: &str, seg: &Seg, out: &mut Flat) {
    for (i, field) in seg.fields.iter().enumerate() {
        if field.is_empty() {
            continue;
        }
        let base = format!("{key}-{}", i + 1);
        match seg.separators {
            None => out.push((base, field.clone())),
            Some((repeat, component)) => {
                let repeats: Vec<&str> = field.split(repeat).collect();
                for (r, rep) in repeats.iter().enumerate() {
                    let rbase = if repeats.len() > 1 {
                        format!("{base}~{}", r + 1)
                    } else {
                        base.clone()
                    };
                    let comps: Vec<&str> = rep.split(component).collect();
                    for (c, comp) in comps.iter().enumerate() {
                        if comp.is_empty() {
                            continue;
                        }
                        let path = if comps.len() > 1 {
                            format!("{rbase}.{}", c + 1)
                        } else {
                            rbase.clone()
                        };
                        out.push((path, (*comp).to_owned()));
                    }
                }
            }
        }
    }
}

fn diff_segments(left: &[Seg], right: &[Seg], options: &Options) -> Diff {
    let (lk, rk) = (keyed(left), keyed(right));
    let right_keys: HashSet<&str> = rk.iter().map(|(k, _)| k.as_str()).collect();
    let left_keys: HashSet<&str> = lk.iter().map(|(k, _)| k.as_str()).collect();
    let mut structural = Vec::new();
    let (mut lf, mut rf) = (Flat::new(), Flat::new());
    for (key, seg) in &lk {
        if right_keys.contains(key.as_str()) {
            flatten_segment(key, seg, &mut lf);
        } else if !options.skips(key) {
            structural.push(Change {
                path: key.clone(),
                kind: ChangeKind::Removed,
                left: Some(seg.raw.clone()),
                right: None,
            });
        }
    }
    for (key, seg) in &rk {
        if left_keys.contains(key.as_str()) {
            flatten_segment(key, seg, &mut rf);
        } else if !options.skips(key) {
            structural.push(Change {
                path: key.clone(),
                kind: ChangeKind::Added,
                left: None,
                right: Some(seg.raw.clone()),
            });
        }
    }
    let mut diff = compare(lf, rf, options);
    diff.changes.extend(structural);
    diff
}

/// Diff two HL7 v2 messages.
///
/// # Errors
/// Returns [`DocError`] if either message does not parse.
pub fn diff_hl7(left: &[u8], right: &[u8], options: &Options) -> Result<Diff, DocError> {
    fn segments(bytes: &[u8]) -> Result<Vec<Seg>, DocError> {
        tpt_doc_hl7v2::Hl7Parser::new(bytes)
            .map(|s| {
                let s = s?;
                let d = *s.delimiters();
                let sep = char::from(d.field);
                let tag = s.tag().to_owned();
                // For MSH, field 1 is the separator itself and field 2 the
                // encoding characters; neither is data worth diffing.
                let skip = usize::from(tag == "MSH") * 2;
                let fields: Vec<String> = s
                    .raw_fields()
                    .iter()
                    .enumerate()
                    .map(|(i, f)| {
                        if i < skip {
                            String::new()
                        } else {
                            (*f).to_owned()
                        }
                    })
                    .collect();
                let raw = format!("{tag}{sep}{}", s.raw_fields().join(&sep.to_string()));
                Ok(Seg {
                    tag,
                    fields,
                    separators: Some((char::from(d.repeat), char::from(d.component))),
                    raw,
                })
            })
            .collect()
    }
    Ok(diff_segments(&segments(left)?, &segments(right)?, options))
}

/// Diff two EDIFACT interchanges.
///
/// # Errors
/// Returns [`DocError`] if either interchange does not parse.
pub fn diff_edifact(left: &[u8], right: &[u8], options: &Options) -> Result<Diff, DocError> {
    fn segments(bytes: &[u8]) -> Result<Vec<Seg>, DocError> {
        tpt_doc_edi::EdifactParser::new(bytes)
            .map(|s| {
                let s = s?;
                let fields: Vec<String> = s.elements().iter().map(|e| (*e).to_owned()).collect();
                Ok(Seg {
                    tag: s.tag().to_owned(),
                    raw: format!("{}+{}", s.tag(), fields.join("+")),
                    fields,
                    separators: None,
                })
            })
            .collect()
    }
    Ok(diff_segments(&segments(left)?, &segments(right)?, options))
}

/// Diff two X12 interchanges.
///
/// # Errors
/// Returns [`DocError`] if either interchange does not parse.
pub fn diff_x12(left: &[u8], right: &[u8], options: &Options) -> Result<Diff, DocError> {
    fn segments(bytes: &[u8]) -> Result<Vec<Seg>, DocError> {
        tpt_doc_edi::x12::X12Parser::new(bytes)?
            .map(|s| {
                let s = s?;
                let fields: Vec<String> = s.elements().iter().map(|e| (*e).to_owned()).collect();
                Ok(Seg {
                    tag: s.tag().to_owned(),
                    raw: format!("{}*{}", s.tag(), fields.join("*")),
                    fields,
                    separators: None,
                })
            })
            .collect()
    }
    Ok(diff_segments(&segments(left)?, &segments(right)?, options))
}

// ---------------------------------------------------------------------------
// JSON (FHIR)
// ---------------------------------------------------------------------------

fn show(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

fn json_walk(path: &str, left: &Value, right: &Value, options: &Options, out: &mut Vec<Change>) {
    if options.skips(path) || left == right {
        return;
    }
    match (left, right) {
        (Value::Object(l), Value::Object(r)) => {
            for (key, lv) in l {
                let child = format!("{path}/{}", escape_pointer(key));
                match r.get(key) {
                    Some(rv) => json_walk(&child, lv, rv, options, out),
                    None if !options.skips(&child) => out.push(Change {
                        path: child,
                        kind: ChangeKind::Removed,
                        left: Some(show(lv)),
                        right: None,
                    }),
                    None => {}
                }
            }
            for (key, rv) in r {
                let child = format!("{path}/{}", escape_pointer(key));
                if !l.contains_key(key) && !options.skips(&child) {
                    out.push(Change {
                        path: child,
                        kind: ChangeKind::Added,
                        left: None,
                        right: Some(show(rv)),
                    });
                }
            }
        }
        (Value::Array(l), Value::Array(r)) => {
            for i in 0..l.len().max(r.len()) {
                let child = format!("{path}/{i}");
                match (l.get(i), r.get(i)) {
                    (Some(lv), Some(rv)) => json_walk(&child, lv, rv, options, out),
                    (Some(lv), None) if !options.skips(&child) => out.push(Change {
                        path: child,
                        kind: ChangeKind::Removed,
                        left: Some(show(lv)),
                        right: None,
                    }),
                    (None, Some(rv)) if !options.skips(&child) => out.push(Change {
                        path: child,
                        kind: ChangeKind::Added,
                        left: None,
                        right: Some(show(rv)),
                    }),
                    _ => {}
                }
            }
        }
        _ => out.push(Change {
            path: if path.is_empty() {
                "/".to_owned()
            } else {
                path.to_owned()
            },
            kind: ChangeKind::Changed,
            left: Some(show(left)),
            right: Some(show(right)),
        }),
    }
}

/// RFC 6901 escaping of a pointer segment.
fn escape_pointer(key: &str) -> String {
    key.replace('~', "~0").replace('/', "~1")
}

/// Diff two JSON documents (FHIR resources and bundles included) by JSON
/// pointer. Object key order is irrelevant; arrays compare by index.
///
/// # Errors
/// Returns [`DocError`] if either side is not valid JSON.
pub fn diff_json(left: &[u8], right: &[u8], options: &Options) -> Result<Diff, DocError> {
    let parse = |bytes: &[u8]| {
        serde_json::from_slice::<Value>(bytes)
            .map_err(|e| DocError::invalid_format(format!("not valid JSON: {e}")))
    };
    let mut changes = Vec::new();
    json_walk("", &parse(left)?, &parse(right)?, options, &mut changes);
    Ok(Diff { changes })
}

// ---------------------------------------------------------------------------
// XLSX and DOCX
// ---------------------------------------------------------------------------

/// Diff the first worksheet of two workbooks, cell by cell.
///
/// # Errors
/// Returns [`DocError`] if either workbook does not parse.
pub fn diff_xlsx(left: &[u8], right: &[u8], options: &Options) -> Result<Diff, DocError> {
    use tpt_doc_spreadsheet::xlsx::{Cell, XlsxReader, column_name};
    fn cells(bytes: &[u8]) -> Result<Flat, DocError> {
        let mut reader = XlsxReader::new(bytes)?;
        let mut out = Vec::new();
        for row in reader.rows() {
            let row = row?;
            for (col, cell) in row.cells().iter().enumerate() {
                let text = match cell {
                    Cell::String(s) | Cell::Error(s) => s.clone(),
                    Cell::Number(n) => n.to_string(),
                    Cell::Boolean(b) => b.to_string(),
                    Cell::Blank => continue,
                };
                out.push((format!("{}{}", column_name(col), row.index), text));
            }
        }
        Ok(out)
    }
    Ok(compare(cells(left)?, cells(right)?, options))
}

/// Diff the paragraphs of two Word documents by index.
///
/// # Errors
/// Returns [`DocError`] if either document does not parse.
pub fn diff_docx(left: &[u8], right: &[u8], options: &Options) -> Result<Diff, DocError> {
    use tpt_doc_word::DocxReader;
    fn paragraphs(bytes: &[u8]) -> Result<Flat, DocError> {
        Ok(DocxReader::extract_text(bytes)?
            .lines()
            .enumerate()
            .map(|(i, line)| (format!("/p/{i}"), line.to_owned()))
            .collect())
    }
    Ok(compare(paragraphs(left)?, paragraphs(right)?, options))
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: &[u8] = b"MSH|^~\\&|HIS|ACME|RIS|ACME|20261001120000||ADT^A01|MSG1|P|2.5.1\r\
EVN|A01|20261001120000\r\
PID|1||12345^^^NZHPI^MR~777^^^NHI^NH||Smith^John^Q||19700101|M\r\
PV1|1|I|WARD4^101^A\r";

    fn paths(d: &Diff) -> Vec<(&str, ChangeKind)> {
        d.changes
            .iter()
            .map(|c| (c.path.as_str(), c.kind))
            .collect()
    }

    #[test]
    fn identical_documents_have_no_changes() {
        assert!(diff_hl7(A, A, &Options::default()).unwrap().is_empty());
    }

    #[test]
    fn hl7_names_the_changed_component() {
        let b = String::from_utf8_lossy(A).replace("Smith^John", "Smyth^John");
        let d = diff_hl7(A, b.as_bytes(), &Options::default()).unwrap();
        assert_eq!(paths(&d), [("PID[0]-5.1", ChangeKind::Changed)]);
        assert_eq!(d.changes[0].left.as_deref(), Some("Smith"));
        assert_eq!(d.changes[0].right.as_deref(), Some("Smyth"));
        assert_eq!(d.to_string(), "~ PID[0]-5.1: Smith \u{2192} Smyth\n");
    }

    #[test]
    fn hl7_repeats_are_addressed_individually() {
        let b = String::from_utf8_lossy(A).replace("777^^^NHI", "888^^^NHI");
        let d = diff_hl7(A, b.as_bytes(), &Options::default()).unwrap();
        assert_eq!(paths(&d), [("PID[0]-3~2.1", ChangeKind::Changed)]);
    }

    #[test]
    fn hl7_added_and_removed_segments_and_fields() {
        let b = String::from_utf8_lossy(A)
            .replace("PV1|1|I|WARD4^101^A\r", "NK1|1|Smith^Jane\r")
            .replace("|M\r", "|F|||||||\r");
        let d = diff_hl7(A, b.as_bytes(), &Options::default()).unwrap();
        let p = paths(&d);
        assert!(p.contains(&("PID[0]-8", ChangeKind::Changed)), "{p:?}");
        assert!(p.contains(&("PV1[0]", ChangeKind::Removed)), "{p:?}");
        assert!(p.contains(&("NK1[0]", ChangeKind::Added)), "{p:?}");
        let removed = d.changes.iter().find(|c| c.path == "PV1[0]").unwrap();
        assert_eq!(removed.left.as_deref(), Some("PV1|1|I|WARD4^101^A"));
    }

    #[test]
    fn hl7_a_field_that_gains_components_still_lines_up() {
        let a = b"MSH|^~\\&|A|B|C|D|||ADT^A01|1|P|2.5.1\rPID|1||1||Smith\r";
        let b = b"MSH|^~\\&|A|B|C|D|||ADT^A01|1|P|2.5.1\rPID|1||1||Smith^John\r";
        let d = diff_hl7(a, b, &Options::default()).unwrap();
        assert_eq!(paths(&d), [("PID[0]-5.2", ChangeKind::Added)]);
    }

    #[test]
    fn ignore_prefixes_skip_volatile_elements() {
        let b = String::from_utf8_lossy(A)
            .replace("20261001120000||ADT", "20261002130000||ADT")
            .replace("MSG1", "MSG2");
        let all = diff_hl7(A, b.as_bytes(), &Options::default()).unwrap();
        assert_eq!(all.changes.len(), 2, "{all}");
        let one = diff_hl7(A, b.as_bytes(), &Options::default().ignoring("MSH[0]-7")).unwrap();
        assert_eq!(paths(&one), [("MSH[0]-10", ChangeKind::Changed)]);
        assert!(
            diff_hl7(A, b.as_bytes(), &Options::default().ignoring("MSH"))
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn json_diff_uses_pointers_and_ignores_key_order() {
        let a =
            br#"{"resourceType":"Patient","id":"1","name":[{"family":"Smith"}],"gender":"male"}"#;
        let b = br#"{"gender":"male","name":[{"family":"Smyth"},{"family":"Extra"}],"resourceType":"Patient","id":"1","active":true}"#;
        let d = diff_json(a, b, &Options::default()).unwrap();
        let p = paths(&d);
        assert!(
            p.contains(&("/name/0/family", ChangeKind::Changed)),
            "{p:?}"
        );
        assert!(p.contains(&("/name/1", ChangeKind::Added)), "{p:?}");
        assert!(p.contains(&("/active", ChangeKind::Added)), "{p:?}");
        assert_eq!(d.changes.len(), 3);
        assert!(diff_json(a, a, &Options::default()).unwrap().is_empty());
        let ignored = diff_json(a, b, &Options::default().ignoring("/name")).unwrap();
        assert_eq!(paths(&ignored), [("/active", ChangeKind::Added)]);
    }

    #[test]
    fn json_pointer_escapes_special_keys() {
        let d = diff_json(
            br#"{"a/b":1,"c~d":1}"#,
            br#"{"a/b":2,"c~d":2}"#,
            &Options::default(),
        )
        .unwrap();
        let p: Vec<_> = d.changes.iter().map(|c| c.path.as_str()).collect();
        assert!(p.contains(&"/a~1b") && p.contains(&"/c~0d"), "{p:?}");
    }

    #[test]
    fn edifact_and_x12_diff_by_element() {
        let a = b"UNB+UNOA:1+S+R+260101:0900+1'UNH+1+ORDERS:D:96A:UN'BGM+220+PO1'UNT+3+1'UNZ+1+1'";
        let b = b"UNB+UNOA:1+S+R+260101:0900+1'UNH+1+ORDERS:D:96A:UN'BGM+220+PO2'UNT+3+1'UNZ+1+1'";
        let d = diff_edifact(a, b, &Options::default()).unwrap();
        assert_eq!(paths(&d), [("BGM[0]-2", ChangeKind::Changed)]);

        let x = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../tpt-doc-edi/tests/fixtures/sample_835.x12"
        ))
        .unwrap();
        assert!(diff_x12(&x, &x, &Options::default()).unwrap().is_empty());
        let y = String::from_utf8_lossy(&x).replace("CLP*", "CLP*Z");
        let d = diff_x12(&x, y.as_bytes(), &Options::default()).unwrap();
        assert_eq!(d.changes.len(), 1);
        assert!(d.changes[0].path.starts_with("CLP[0]-1"), "{d}");
    }

    #[test]
    fn malformed_input_is_an_error() {
        assert!(diff_json(b"{", b"{}", &Options::default()).is_err());
        assert!(diff_xlsx(b"nope", b"nope", &Options::default()).is_err());
        assert!(diff_docx(b"nope", b"nope", &Options::default()).is_err());
    }
}
