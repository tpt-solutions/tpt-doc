//! De-identification of HL7 v2 messages.
//!
//! A [`RedactionProfile`] is a list of [`Rule`]s addressing a field (or one
//! component of it) by segment and position, with an [`Action`]. The default
//! profile ([`RedactionProfile::safe_harbor`]) covers the direct identifiers
//! HIPAA's Safe Harbor method names that HL7 v2 commonly carries: names,
//! addresses, phone and e-mail, identifiers, and the day and month of dates.
//! New Zealand's HIPC 2020 asks for the same data minimisation for secondary
//! use, so the profile is a starting point for either — **review it against
//! your own data flows; it does not guarantee anonymity** (free-text fields
//! such as `NTE-3` and `OBX-5` can contain anything and are cleared by
//! default for that reason).
//!
//! Everything not addressed by a rule is copied through byte for byte, so the
//! message stays valid and structurally identical.

use std::collections::HashMap;

use tpt_doc_core::DocError;

use crate::{Delimiters, Hl7Parser, Segment};

/// What to do with a matched field or component.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Replace with an empty value.
    Clear,
    /// Replace with a fixed string.
    Replace(String),
    /// Replace with `"{prefix}{n}"`, where `n` numbers the distinct original
    /// values in order of first appearance, so equal inputs stay equal and
    /// unequal inputs stay distinct within one message.
    Pseudonym(String),
    /// Keep only the year of an HL7 timestamp (`19700101` becomes `1970`).
    YearOnly,
}

/// One redaction rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    /// Segment tag, e.g. `"PID"`.
    pub segment: String,
    /// Field number, 1-based (not valid for `MSH`).
    pub field: usize,
    /// Component number, 1-based; `None` applies to the whole field.
    pub component: Option<usize>,
    /// The action to apply.
    pub action: Action,
}

impl Rule {
    /// A rule for a whole field.
    #[must_use]
    pub fn field(segment: &str, field: usize, action: Action) -> Self {
        Self {
            segment: segment.to_owned(),
            field,
            component: None,
            action,
        }
    }

    /// A rule for one component of a field.
    #[must_use]
    pub fn component(segment: &str, field: usize, component: usize, action: Action) -> Self {
        Self {
            segment: segment.to_owned(),
            field,
            component: Some(component),
            action,
        }
    }
}

/// An ordered set of [`Rule`]s.
#[derive(Debug, Clone, Default)]
pub struct RedactionProfile {
    rules: Vec<Rule>,
}

impl RedactionProfile {
    /// An empty profile (changes nothing).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a rule.
    #[must_use]
    pub fn with_rule(mut self, rule: Rule) -> Self {
        self.rules.push(rule);
        self
    }

    /// The rules, in application order.
    #[must_use]
    pub fn rules(&self) -> &[Rule] {
        &self.rules
    }

    /// Direct identifiers per HIPAA Safe Harbor, as carried by common HL7 v2.5
    /// ADT/ORU messages. See the module docs for the caveats.
    #[must_use]
    pub fn safe_harbor() -> Self {
        use Action::{Clear, Pseudonym, YearOnly};
        let mut p = Self::new()
            // PID: identifiers, name, dates, contact and demographics.
            .with_rule(Rule::component("PID", 3, 1, Pseudonym("PATIENT-".into())))
            .with_rule(Rule::field("PID", 2, Clear)) // patient id (legacy)
            .with_rule(Rule::field("PID", 4, Clear)) // alternate id
            .with_rule(Rule::field("PID", 5, Clear)) // name
            .with_rule(Rule::field("PID", 6, Clear)) // mother's maiden name
            .with_rule(Rule::field("PID", 7, YearOnly)) // date of birth
            .with_rule(Rule::field("PID", 9, Clear)) // alias
            .with_rule(Rule::field("PID", 11, Clear)) // address
            .with_rule(Rule::field("PID", 12, Clear)) // county
            .with_rule(Rule::field("PID", 13, Clear)) // home phone
            .with_rule(Rule::field("PID", 14, Clear)) // business phone
            .with_rule(Rule::field("PID", 18, Clear)) // account number
            .with_rule(Rule::field("PID", 19, Clear)) // SSN
            .with_rule(Rule::field("PID", 20, Clear)) // licence number
            .with_rule(Rule::field("PID", 29, YearOnly)) // death date
            // PV1: visit number, attending/referring/consulting staff, dates.
            .with_rule(Rule::field("PV1", 7, Clear))
            .with_rule(Rule::field("PV1", 8, Clear))
            .with_rule(Rule::field("PV1", 9, Clear))
            .with_rule(Rule::field("PV1", 17, Clear))
            .with_rule(Rule::component("PV1", 19, 1, Pseudonym("VISIT-".into())))
            .with_rule(Rule::field("PV1", 44, YearOnly))
            .with_rule(Rule::field("PV1", 45, YearOnly))
            // Next of kin, guarantor, insured: all personal data.
            .with_rule(Rule::field("NK1", 2, Clear))
            .with_rule(Rule::field("NK1", 4, Clear))
            .with_rule(Rule::field("NK1", 5, Clear))
            .with_rule(Rule::field("NK1", 6, Clear))
            .with_rule(Rule::field("GT1", 3, Clear))
            .with_rule(Rule::field("GT1", 5, Clear))
            .with_rule(Rule::field("GT1", 6, Clear))
            .with_rule(Rule::field("IN1", 16, Clear))
            .with_rule(Rule::field("IN1", 19, Clear))
            .with_rule(Rule::field("IN1", 36, Clear))
            // Free text can hold anything: comments and text results.
            .with_rule(Rule::field("NTE", 3, Clear))
            // Result dates and ordering/performing staff.
            .with_rule(Rule::field("OBR", 7, YearOnly))
            .with_rule(Rule::field("OBR", 14, YearOnly))
            .with_rule(Rule::field("OBR", 16, Clear))
            .with_rule(Rule::field("OBX", 14, YearOnly))
            .with_rule(Rule::field("OBX", 16, Clear));
        // OBX-5 is a free-text value unless it is numeric; handled in `redact`.
        p.rules.push(Rule::field("OBX", 5, Clear));
        p
    }
}

/// One field that was changed, for an audit trail. Original values are
/// deliberately not recorded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Change {
    /// Index of the segment in the message (0-based).
    pub segment_index: usize,
    /// Segment tag.
    pub segment: String,
    /// Field number (1-based).
    pub field: usize,
    /// Component, when the rule addressed one.
    pub component: Option<usize>,
}

/// The result of [`redact`].
#[derive(Debug, Clone)]
pub struct Redacted {
    /// The de-identified message, `\r`-terminated segments.
    pub bytes: Vec<u8>,
    /// Every field that held a value and was changed.
    pub changes: Vec<Change>,
}

/// Apply `profile` to an HL7 v2 message.
///
/// `OBX-5` is only cleared when `OBX-2` is not a numeric type (`NM`, `SN`):
/// laboratory values are the clinical payload, while text results can carry
/// identifying prose.
///
/// # Errors
/// Returns [`DocError`] if the message does not parse or a rule addresses
/// field 0 or an `MSH` field.
pub fn redact(message: &[u8], profile: &RedactionProfile) -> Result<Redacted, DocError> {
    if profile
        .rules
        .iter()
        .any(|r| r.field == 0 || r.component == Some(0))
    {
        return Err(DocError::invalid_format(
            "redaction rules are 1-based; field and component 0 do not exist",
        ));
    }
    if profile.rules.iter().any(|r| r.segment == "MSH") {
        return Err(DocError::invalid_format(
            "MSH cannot be redacted: it defines the delimiters",
        ));
    }
    let segments: Vec<Segment<'_>> = Hl7Parser::new(message).collect::<Result<_, _>>()?;
    if segments.first().map(Segment::tag) != Some("MSH") {
        return Err(DocError::invalid_format(
            "HL7 message must start with MSH; refusing to pass unrecognised text through",
        ));
    }
    let mut pseudonyms = Pseudonyms::default();
    let mut changes = Vec::new();
    let mut out = Vec::with_capacity(message.len());

    for (index, segment) in segments.iter().enumerate() {
        let d = *segment.delimiters();
        let mut fields: Vec<String> = segment
            .raw_fields()
            .iter()
            .map(|f| (*f).to_owned())
            .collect();
        for rule in profile.rules.iter().filter(|r| r.segment == segment.tag()) {
            if rule.field == 5 && is_numeric_result(segment) {
                continue;
            }
            let Some(slot) = fields.get_mut(rule.field - 1) else {
                continue;
            };
            if redact_field(slot, rule, d, &mut pseudonyms) {
                changes.push(Change {
                    segment_index: index,
                    segment: rule.segment.clone(),
                    field: rule.field,
                    component: rule.component,
                });
            }
        }
        write_segment(&mut out, segment, &fields);
    }
    Ok(Redacted {
        bytes: out,
        changes,
    })
}

/// `OBX-5` holding a numeric value (`OBX-2` is `NM` or `SN`) is the clinical
/// result rather than free text, so it is kept.
fn is_numeric_result(segment: &Segment<'_>) -> bool {
    segment.tag() == "OBX" && matches!(segment.field(2), Some("NM" | "SN"))
}

/// Serialise one segment with its (possibly rewritten) fields.
fn write_segment(out: &mut Vec<u8>, segment: &Segment<'_>, fields: &[String]) {
    let sep = char::from(segment.delimiters().field).to_string();
    out.extend_from_slice(segment.tag().as_bytes());
    if segment.tag() == "MSH" {
        // `fields[0]` is the field separator itself.
        out.extend_from_slice(fields[0].as_bytes());
        out.extend_from_slice(fields[1..].join(&sep).as_bytes());
    } else {
        out.extend_from_slice(sep.as_bytes());
        out.extend_from_slice(fields.join(&sep).as_bytes());
    }
    out.push(b'\r');
}

/// Numbering of distinct values per pseudonym prefix.
#[derive(Default)]
struct Pseudonyms {
    numbers: HashMap<(String, String), usize>,
    next: HashMap<String, usize>,
}

impl Pseudonyms {
    fn assign(&mut self, prefix: &str, value: &str) -> String {
        let key = (prefix.to_owned(), value.to_owned());
        if let Some(n) = self.numbers.get(&key) {
            return format!("{prefix}{n}");
        }
        let counter = self.next.entry(prefix.to_owned()).or_insert(0);
        *counter += 1;
        self.numbers.insert(key, *counter);
        format!("{prefix}{counter}")
    }
}

fn apply_action(action: &Action, value: &str, pseudonyms: &mut Pseudonyms) -> String {
    match action {
        Action::Clear => String::new(),
        Action::Replace(text) => text.clone(),
        Action::Pseudonym(prefix) => pseudonyms.assign(prefix, value),
        Action::YearOnly => value.chars().take(4).collect(),
    }
}

/// Apply `rule` to a raw field value across its repetitions. Returns whether
/// anything non-empty was rewritten.
fn redact_field(
    slot: &mut String,
    rule: &Rule,
    d: Delimiters,
    pseudonyms: &mut Pseudonyms,
) -> bool {
    let repeat = char::from(d.repeat).to_string();
    let component = char::from(d.component).to_string();
    let mut touched = false;
    let rebuilt: Vec<String> = slot
        .split(repeat.as_str())
        .map(|rep| match rule.component {
            None if rep.is_empty() => String::new(),
            None => {
                touched = true;
                apply_action(&rule.action, rep, pseudonyms)
            }
            Some(m) => {
                let mut parts: Vec<String> =
                    rep.split(component.as_str()).map(str::to_owned).collect();
                if let Some(part) = parts.get_mut(m - 1)
                    && !part.is_empty()
                {
                    *part = apply_action(&rule.action, part, pseudonyms);
                    touched = true;
                }
                parts.join(&component)
            }
        })
        .collect();
    if touched {
        *slot = rebuilt.join(&repeat);
    }
    touched
}

#[cfg(test)]
mod tests {
    use super::*;

    const ADT: &[u8] = b"MSH|^~\\&|HIS|ACME|RIS|ACME|20261001120000||ADT^A01|MSG1|P|2.5.1\r\
EVN|A01|20261001120000\r\
PID|1||12345^^^NZHPI^MR~99999^^^NHI^NH||Smith^John^Q||19700101|M|||1 Lambton Quay^^Wellington^^6011^NZ||0211234567|||||ACC-77|123-45-6789\r\
PV1|1|I|WARD4^101^A|||||5551^Jones^Amy|||||||||||V0001\r\
NK1|1|Smith^Jane|SPO|2 Other St|0219999999\r";

    fn text(r: &Redacted) -> String {
        String::from_utf8(r.bytes.clone()).expect("utf8")
    }

    #[test]
    fn safe_harbor_removes_direct_identifiers() {
        let r = redact(ADT, &RedactionProfile::safe_harbor()).expect("redact");
        let out = text(&r);
        for leaked in [
            "Smith",
            "John",
            "Lambton",
            "0211234567",
            "123-45-6789",
            "ACC-77",
            "Jones",
            "5551",
            "Jane",
            "0219999999",
            "12345",
            "99999",
            "V0001",
        ] {
            assert!(!out.contains(leaked), "{leaked} leaked in {out:?}");
        }
        // Year survives, month and day do not.
        assert!(out.contains("|1970|"), "{out:?}");
        assert!(!out.contains("19700101"));
        // Sex and the clinical envelope are not identifiers and stay.
        assert!(out.contains("|M|"));
        assert!(out.starts_with("MSH|^~\\&|HIS|ACME|RIS|ACME|"));
        assert!(out.contains("ADT^A01"));
    }

    #[test]
    fn output_is_still_a_valid_message_with_the_same_shape() {
        let before: Vec<String> = Hl7Parser::new(ADT)
            .map(|s| s.expect("parse").tag().to_owned())
            .collect();
        let r = redact(ADT, &RedactionProfile::safe_harbor()).expect("redact");
        let after: Vec<String> = Hl7Parser::new(&r.bytes)
            .map(|s| s.expect("parse").tag().to_owned())
            .collect();
        assert_eq!(before, after);
        let fields_before = Hl7Parser::new(ADT).map(|s| s.expect("p").raw_fields().len());
        let fields_after = Hl7Parser::new(&r.bytes).map(|s| s.expect("p").raw_fields().len());
        assert!(
            fields_before.eq(fields_after),
            "field counts must not shift"
        );
    }

    #[test]
    fn pseudonyms_are_stable_and_distinct_within_a_message() {
        let r = redact(ADT, &RedactionProfile::safe_harbor()).expect("redact");
        let out = text(&r);
        // Two PID-3 repeats with different ids get different pseudonyms; the
        // authority and type components are kept.
        assert!(
            out.contains("PATIENT-1^^^NZHPI^MR~PATIENT-2^^^NHI^NH"),
            "{out:?}"
        );
        assert!(out.contains("VISIT-1"), "{out:?}");

        let twice = b"MSH|^~\\&|A|B|C|D|||ADT^A01|M|P|2.5.1\rPID|1||AAA||X\rPID|1||AAA||X\r";
        let r = redact(twice, &RedactionProfile::safe_harbor()).expect("redact");
        assert_eq!(text(&r).matches("PATIENT-1").count(), 2);
    }

    #[test]
    fn changes_record_positions_but_not_values() {
        let r = redact(ADT, &RedactionProfile::safe_harbor()).expect("redact");
        assert!(r.changes.iter().any(|c| c.segment == "PID" && c.field == 5));
        let dump = format!("{:?}", r.changes);
        assert!(!dump.contains("Smith"));
        // Empty fields are not reported as changes.
        assert!(
            !r.changes
                .iter()
                .any(|c| c.segment == "PID" && c.field == 29)
        );
    }

    #[test]
    fn numeric_results_survive_but_text_results_are_cleared() {
        let msg = b"MSH|^~\\&|L|B|C|D|||ORU^R01|M|P|2.5.1\rPID|1||9||X\r\
OBX|1|NM|GLU^Glucose||5.2|mmol/L|||||F\rOBX|2|ST|NOTE^Comment||Patient lives at 5 High St|||||F\r\
NTE|1||Called the daughter\r";
        let out = text(&redact(msg, &RedactionProfile::safe_harbor()).expect("redact"));
        assert!(out.contains("|5.2|mmol/L|"), "{out:?}");
        assert!(!out.contains("High St"));
        assert!(!out.contains("daughter"));
    }

    #[test]
    fn custom_profiles_and_rule_validation() {
        let profile = RedactionProfile::new().with_rule(Rule::component(
            "PID",
            5,
            1,
            Action::Replace("REDACTED".into()),
        ));
        let out = text(&redact(ADT, &profile).expect("redact"));
        assert!(out.contains("|REDACTED^John^Q|"), "{out:?}");

        assert!(
            redact(
                ADT,
                &RedactionProfile::new().with_rule(Rule::field("MSH", 3, Action::Clear))
            )
            .is_err()
        );
        assert!(
            redact(
                ADT,
                &RedactionProfile::new().with_rule(Rule::field("PID", 0, Action::Clear))
            )
            .is_err()
        );
        assert!(redact(b"not hl7", &RedactionProfile::safe_harbor()).is_err());
    }

    #[test]
    fn non_default_delimiters_are_respected() {
        let msg = b"MSH|#~\\&|A|B|C|D|||ADT#A01|M|P|2.5.1\rPID|1||77#x||Doe#Jo||19800202\r";
        let out = text(&redact(msg, &RedactionProfile::safe_harbor()).expect("redact"));
        assert!(out.contains("PATIENT-1#x"), "{out:?}");
        assert!(!out.contains("Doe"));
        assert!(out.starts_with("MSH|#~\\&|"));
    }
}

#[cfg(test)]
mod proptests {
    use proptest::prelude::*;

    use super::*;

    proptest! {
        /// Arbitrary bytes never panic, and on success no original PID-5 name
        /// survives into the output.
        #[test]
        fn redact_never_panics_on_arbitrary_input(bytes in proptest::collection::vec(any::<u8>(), 0..512)) {
            let _ = redact(&bytes, &RedactionProfile::safe_harbor());
        }

        #[test]
        fn names_never_survive(family in "[A-Z][a-z]{3,10}", given in "[A-Z][a-z]{3,10}") {
            let msg = format!("MSH|^~\\&|A|B|C|D|||ADT^A01|M|P|2.5.1\rPID|1||1||{family}^{given}\r");
            let out = redact(msg.as_bytes(), &RedactionProfile::safe_harbor()).expect("redact");
            let text = String::from_utf8(out.bytes).expect("utf8");
            prop_assert!(!text.contains(&family) && !text.contains(&given));
        }
    }
}
