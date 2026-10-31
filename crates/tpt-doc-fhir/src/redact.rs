//! De-identification of FHIR JSON by element path.
//!
//! Works on the JSON itself rather than the typed models, so elements this
//! crate does not model (extensions, unmodelled resources, `contained`
//! resources) are still reached. A [`Rule`] names an element as
//! `Resource.element.child` (`"*"` for any resource type); arrays are
//! traversed transparently, so `Patient.name` covers every name and
//! `Patient.identifier.value` every identifier value.
//!
//! [`RedactionProfile::safe_harbor`] removes the direct identifiers HIPAA Safe
//! Harbor names that FHIR resources carry, cuts dates to the year, and replaces
//! ids with stable pseudonyms. A `Patient` that becomes `PATIENT-1` is
//! `PATIENT-1` everywhere it is referenced (`reference`, `fullUrl`), so the
//! bundle still hangs together. **It is a starting point, not a guarantee of
//! anonymity**: free-text elements (`note`, `valueString`, narrative `text`)
//! are removed by default for that reason, but review the profile against
//! your own data flows.

use std::collections::HashMap;

use serde_json::Value;
use tpt_doc_core::DocError;

/// What to do with a matched element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Delete the element.
    Remove,
    /// Replace string values with a fixed string.
    Replace(String),
    /// Replace string values with `"{prefix}{n}"`, numbering distinct values in
    /// order of first appearance.
    Pseudonym(String),
    /// Keep only the year of a date or dateTime (`1970-01-01` becomes `1970`,
    /// which is a valid FHIR partial date).
    YearOnly,
    /// Rewrite `Type/id` references (also `fullUrl`) whose `Type` is `Patient`
    /// to the same pseudonym [`Action::Pseudonym`] gave that patient's `id`.
    PatientReference(String),
}

/// One redaction rule: a path and an action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    path: Vec<String>,
    action: Action,
}

impl Rule {
    /// A rule for `path`, e.g. `"Patient.name"` or `"*.subject.display"`.
    #[must_use]
    pub fn new(path: &str, action: Action) -> Self {
        Self {
            path: path.split('.').map(str::to_owned).collect(),
            action,
        }
    }

    /// The path as written.
    #[must_use]
    pub fn path(&self) -> String {
        self.path.join(".")
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

    /// The rules in application order.
    #[must_use]
    pub fn rules(&self) -> &[Rule] {
        &self.rules
    }

    /// Direct identifiers per HIPAA Safe Harbor for the common clinical
    /// resources. See the module docs for the caveats.
    #[must_use]
    pub fn safe_harbor() -> Self {
        use Action::{PatientReference, Pseudonym, Remove, YearOnly};
        let rules = [
            // Patient: ids first, so references can follow the same numbering.
            ("Patient.id", Pseudonym("PATIENT-".into())),
            ("Patient.identifier.value", Pseudonym("ID-".into())),
            ("Patient.name", Remove),
            ("Patient.telecom", Remove),
            ("Patient.address", Remove),
            ("Patient.photo", Remove),
            ("Patient.contact", Remove),
            ("Patient.birthDate", YearOnly),
            ("Patient.deceasedDateTime", YearOnly),
            ("Patient.generalPractitioner", Remove),
            ("Patient.managingOrganization", Remove),
            // Dates on clinical resources.
            ("Observation.effectiveDateTime", YearOnly),
            ("Observation.issued", YearOnly),
            ("Encounter.period.start", YearOnly),
            ("Encounter.period.end", YearOnly),
            // Narrative and free text.
            ("*.text", Remove),
            ("*.note", Remove),
            ("Observation.valueString", Remove),
            // Names that ride along on references.
            ("*.subject.display", Remove),
            ("*.performer.display", Remove),
            ("*.participant.individual", Remove),
            ("*.recorder.display", Remove),
            ("*.author.display", Remove),
            // References and bundle URLs follow the patient pseudonym.
            ("*.subject.reference", PatientReference("PATIENT-".into())),
            ("Bundle.entry.fullUrl", PatientReference("PATIENT-".into())),
        ];
        Self {
            rules: rules
                .into_iter()
                .map(|(path, action)| Rule::new(path, action))
                .collect(),
        }
    }
}

/// An element that was changed, for an audit trail (never the value).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Change {
    /// The `resourceType` the rule matched.
    pub resource_type: String,
    /// The rule's path.
    pub path: String,
    /// How many elements it changed in this resource.
    pub count: usize,
}

/// The result of [`redact`].
#[derive(Debug, Clone)]
pub struct Redacted {
    /// The de-identified JSON.
    pub bytes: Vec<u8>,
    /// What was changed, per resource and rule.
    pub changes: Vec<Change>,
}

/// De-identify a FHIR JSON resource or `Bundle`.
///
/// # Errors
/// Returns [`DocError`] if the input is not a JSON object with a
/// `resourceType`, or a rule has an empty path.
pub fn redact(json: &[u8], profile: &RedactionProfile) -> Result<Redacted, DocError> {
    if profile
        .rules
        .iter()
        .any(|r| r.path.iter().any(String::is_empty))
    {
        return Err(DocError::invalid_format(
            "redaction rule has an empty path segment",
        ));
    }
    let mut root: Value = serde_json::from_slice(json)
        .map_err(|e| DocError::invalid_format(format!("not valid JSON: {e}")))?;
    if root.get("resourceType").and_then(Value::as_str).is_none() {
        return Err(DocError::invalid_format(
            "JSON has no `resourceType`; refusing to pass unrecognised content through",
        ));
    }
    let mut state = State::default();
    redact_resource(&mut root, profile, &mut state);
    let bytes = serde_json::to_vec_pretty(&root)
        .map_err(|e| DocError::invalid_format(format!("JSON serialization failed: {e}")))?;
    Ok(Redacted {
        bytes,
        changes: state.changes,
    })
}

#[derive(Default)]
struct State {
    numbers: HashMap<(String, String), usize>,
    next: HashMap<String, usize>,
    changes: Vec<Change>,
}

impl State {
    fn pseudonym(&mut self, prefix: &str, value: &str) -> String {
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

/// Redact one resource, then recurse into `Bundle.entry[].resource` and
/// `contained[]`.
fn redact_resource(resource: &mut Value, profile: &RedactionProfile, state: &mut State) {
    let Some(kind) = resource
        .get("resourceType")
        .and_then(Value::as_str)
        .map(str::to_owned)
    else {
        return;
    };
    for rule in &profile.rules {
        let (head, rest) = rule.path.split_first().expect("path is non-empty");
        if head != "*" && *head != kind {
            continue;
        }
        let count = apply(resource, rest, &rule.action, state);
        if count > 0 {
            state.changes.push(Change {
                resource_type: kind.clone(),
                path: rule.path(),
                count,
            });
        }
    }
    // Nested resources are redacted by their own type's rules.
    if let Some(entries) = resource.get_mut("entry").and_then(Value::as_array_mut) {
        for entry in entries {
            if let Some(inner) = entry.get_mut("resource") {
                redact_resource(inner, profile, state);
            }
        }
    }
    if let Some(contained) = resource.get_mut("contained").and_then(Value::as_array_mut) {
        for inner in contained {
            redact_resource(inner, profile, state);
        }
    }
}

/// Apply `action` at `path` below `value`, returning how many elements changed.
fn apply(value: &mut Value, path: &[String], action: &Action, state: &mut State) -> usize {
    match value {
        Value::Array(items) => items
            .iter_mut()
            .map(|item| apply(item, path, action, state))
            .sum(),
        Value::Object(map) => {
            let Some((key, rest)) = path.split_first() else {
                return 0;
            };
            if rest.is_empty() {
                return match action {
                    Action::Remove => usize::from(map.remove(key).is_some()),
                    _ => map.get_mut(key).map_or(0, |v| leaf(v, action, state)),
                };
            }
            map.get_mut(key)
                .map_or(0, |child| apply(child, rest, action, state))
        }
        _ => 0,
    }
}

/// Apply a value-rewriting action to a string (or each string in an array).
fn leaf(value: &mut Value, action: &Action, state: &mut State) -> usize {
    match value {
        Value::Array(items) => items.iter_mut().map(|v| leaf(v, action, state)).sum(),
        Value::String(text) => {
            let new = match action {
                Action::Remove => return 0,
                Action::Replace(with) => with.clone(),
                Action::Pseudonym(prefix) => state.pseudonym(prefix, text),
                Action::YearOnly => text.chars().take(4).collect(),
                Action::PatientReference(prefix) => match patient_id(text) {
                    Some((head, id)) => {
                        let pseudo = state.pseudonym(prefix, id);
                        format!("{head}Patient/{pseudo}")
                    }
                    None => return 0,
                },
            };
            if *text == new {
                return 0;
            }
            *text = new;
            1
        }
        _ => 0,
    }
}

/// Split `[base/]Patient/id[/_history/..]` into (`base/`, `id`).
fn patient_id(reference: &str) -> Option<(&str, &str)> {
    let at = reference.find("Patient/")?;
    // `Patient/` must start a path segment, not end e.g. `NotPatient/`.
    if at > 0 && !reference[..at].ends_with('/') {
        return None;
    }
    let id = reference[at + "Patient/".len()..].split('/').next()?;
    (!id.is_empty()).then_some((&reference[..at], id))
}

#[cfg(test)]
mod tests {
    use super::*;

    const BUNDLE: &str = include_str!("../tests/fixtures/patient_bundle.json");

    fn run(json: &str) -> (Value, Vec<Change>) {
        let out = redact(json.as_bytes(), &RedactionProfile::safe_harbor()).expect("redact");
        (
            serde_json::from_slice(&out.bytes).expect("json"),
            out.changes,
        )
    }

    #[test]
    fn identifiers_are_removed_from_the_fixture_bundle() {
        let out = redact(BUNDLE.as_bytes(), &RedactionProfile::safe_harbor()).expect("redact");
        let text = String::from_utf8(out.bytes).expect("utf8");
        for leaked in ["Smith", "John", "Tama", "ABC1234", "nz-12345", "1970-01-01"] {
            assert!(!text.contains(leaked), "{leaked} leaked");
        }
        assert!(text.contains("\"birthDate\": \"1970\""), "{text}");
        // Clinical content stays.
        assert!(text.contains("29463-7") && text.contains("72.5"));
    }

    #[test]
    fn references_follow_the_patient_pseudonym() {
        let (value, _) = run(BUNDLE);
        let entries = value["entry"].as_array().expect("entries");
        let patient_id = entries[0]["resource"]["id"].as_str().expect("id");
        assert_eq!(patient_id, "PATIENT-1");
        let subject = entries[1]["resource"]["subject"]["reference"]
            .as_str()
            .expect("subject");
        assert_eq!(subject, "Patient/PATIENT-1");
    }

    #[test]
    fn distinct_patients_stay_distinct_and_equal_ones_equal() {
        let json = r#"{"resourceType":"Bundle","type":"collection","entry":[
          {"fullUrl":"http://x/Patient/a","resource":{"resourceType":"Patient","id":"a"}},
          {"fullUrl":"http://x/Patient/b","resource":{"resourceType":"Patient","id":"b"}},
          {"resource":{"resourceType":"Observation","status":"final","code":{"text":"x"},
            "subject":{"reference":"Patient/b","display":"Bee Person"}}}]}"#;
        let (v, _) = run(json);
        assert_eq!(v["entry"][0]["fullUrl"], "http://x/Patient/PATIENT-1");
        assert_eq!(v["entry"][1]["fullUrl"], "http://x/Patient/PATIENT-2");
        assert_eq!(
            v["entry"][2]["resource"]["subject"]["reference"],
            "Patient/PATIENT-2"
        );
        assert!(
            v["entry"][2]["resource"]["subject"]
                .get("display")
                .is_none()
        );
        // `code.text` is a clinical term, not narrative: it stays.
        assert_eq!(v["entry"][2]["resource"]["code"]["text"], "x");
    }

    #[test]
    fn free_text_and_narrative_are_removed() {
        let json = r#"{"resourceType":"Observation","status":"final","code":{"text":"Weight"},
          "text":{"status":"generated","div":"<div>John Smith 72kg</div>"},
          "note":[{"text":"daughter called"}],"valueString":"lives at 5 High St"}"#;
        let (v, changes) = run(json);
        for key in ["text", "note", "valueString"] {
            assert!(v.get(key).is_none(), "{key} survived");
        }
        assert_eq!(v["code"]["text"], "Weight");
        assert!(changes.iter().any(|c| c.path == "*.text"));
    }

    #[test]
    fn contained_and_unmodelled_resources_are_reached() {
        let json = r##"{"resourceType":"Observation","status":"final","code":{"text":"c"},
          "contained":[{"resourceType":"Patient","id":"p1","name":[{"family":"Hidden"}]}],
          "subject":{"reference":"#p1"}}"##;
        let (v, _) = run(json);
        assert!(!v.to_string().contains("Hidden"));
        // A resource type this crate has no model for is still redacted.
        let json = r#"{"resourceType":"Patient","id":"q","name":[{"family":"Nope"}],"extension":[{"url":"u","valueString":"keep"}]}"#;
        let (v, _) = run(json);
        assert!(v.get("name").is_none());
        assert_eq!(v["extension"][0]["valueString"], "keep");
    }

    #[test]
    fn audit_has_paths_and_counts_but_no_values() {
        let out = redact(BUNDLE.as_bytes(), &RedactionProfile::safe_harbor()).expect("redact");
        let name = out
            .changes
            .iter()
            .find(|c| c.path == "Patient.name")
            .expect("name change");
        assert_eq!((name.resource_type.as_str(), name.count), ("Patient", 1));
        assert!(!format!("{:?}", out.changes).contains("Smith"));
    }

    #[test]
    fn custom_rules_and_input_validation() {
        let profile = RedactionProfile::new().with_rule(Rule::new(
            "Patient.gender",
            Action::Replace("unknown".into()),
        ));
        let out = redact(
            br#"{"resourceType":"Patient","id":"x","gender":"male"}"#,
            &profile,
        )
        .expect("redact");
        assert!(
            String::from_utf8(out.bytes)
                .expect("utf8")
                .contains("\"unknown\"")
        );

        assert!(redact(b"[]", &profile).is_err());
        assert!(redact(b"{\"a\":1}", &profile).is_err());
        assert!(redact(b"not json", &profile).is_err());
        let bad = RedactionProfile::new().with_rule(Rule::new("Patient..name", Action::Remove));
        assert!(redact(br#"{"resourceType":"Patient"}"#, &bad).is_err());
    }

    #[test]
    fn patient_id_parsing() {
        assert_eq!(patient_id("Patient/abc"), Some(("", "abc")));
        assert_eq!(
            patient_id("http://x/Patient/abc/_history/2"),
            Some(("http://x/", "abc"))
        );
        assert_eq!(patient_id("NotPatient/abc"), None);
        assert_eq!(patient_id("Observation/abc"), None);
        assert_eq!(patient_id("Patient/"), None);
    }
}
