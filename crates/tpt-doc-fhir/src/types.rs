use serde::{Deserialize, Serialize};

/// A human name with family and given name components.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HumanName {
    /// Family name (surname).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub family: Option<String>,
    /// Given names (first name, middle names).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub given: Vec<String>,
}

impl HumanName {
    /// Construct a name with a single family and given name.
    pub fn new(family: impl Into<String>, given: impl Into<String>) -> Self {
        Self {
            family: Some(family.into()),
            given: vec![given.into()],
        }
    }
}

/// A FHIR Identifier: a system + value pair.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Identifier {
    /// URI that defines the namespace of the identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    /// The identifier value within the namespace.
    pub value: String,
}

/// A codeable concept: a code from a terminology system, plus optional display text.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CodeableConcept {
    /// Coded entries from a terminology system.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub coding: Vec<Coding>,
    /// Short human-readable display text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

impl CodeableConcept {
    /// Construct a concept with display text only.
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            coding: Vec::new(),
            text: Some(text.into()),
        }
    }

    /// Construct a concept from a single coding.
    #[must_use]
    pub fn from_coding(coding: Coding) -> Self {
        Self {
            coding: vec![coding],
            text: None,
        }
    }
}

/// A reference to a code defined by a terminology system.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Coding {
    /// URI of the terminology system (e.g. `"http://loinc.org"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    /// The code value in the system.
    pub code: String,
    /// Human-readable representation of the code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
}

impl Coding {
    /// Construct a coding with system and code.
    #[must_use]
    pub fn new(system: impl Into<String>, code: impl Into<String>) -> Self {
        Self {
            system: Some(system.into()),
            code: code.into(),
            display: None,
        }
    }

    /// Attach a human-readable display string.
    #[must_use]
    pub fn display(mut self, display: impl Into<String>) -> Self {
        self.display = Some(display.into());
        self
    }
}

/// A reference from one resource to another.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Reference {
    /// Literal reference, e.g. `"Patient/nz-12345"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    /// Human-readable text summary of the target.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
}

impl Reference {
    /// Construct a literal reference to `resource/id`.
    #[must_use]
    pub fn to(target: impl Into<String>) -> Self {
        Self {
            reference: Some(target.into()),
            display: None,
        }
    }
}

/// A time period defined by a start and (optional) end date/time.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Period {
    /// Starting time (ISO 8601 date or date-time).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<String>,
    /// End time, inclusive.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<String>,
}

impl Period {
    /// Construct a period from optional start and end strings.
    #[must_use]
    pub fn new(start: impl Into<String>, end: impl Into<String>) -> Self {
        Self {
            start: Some(start.into()),
            end: Some(end.into()),
        }
    }
}

/// A measured or stated amount (`Quantity` datatype).
///
/// `value` is the number. `value_text` is the decimal exactly as it was read
/// (or as the caller wrote it), so that FHIR's decimal precision survives: `5.40`
/// and `5.4` are the same number but say different things about precision. JSON
/// output writes `value_text` when it is present, and XML output writes it as
/// the `value` attribute.
#[derive(Debug, Clone)]
pub struct Quantity {
    /// The numerical value.
    pub value: f64,
    /// The decimal as written, when it came from a document.
    pub value_text: Option<String>,
    /// Human-readable unit (e.g. `"mg"`).
    pub unit: Option<String>,
    /// URI of the unit code system (e.g. UCUM).
    pub system: Option<String>,
    /// The unit as a code in `system`.
    pub code: Option<String>,
}

/// Two quantities are equal when they have the same number, unit and code.
/// Their decimal text is compared only when both carry it: a quantity built in
/// code has no text, and it equals the same quantity read back.
impl PartialEq for Quantity {
    fn eq(&self, other: &Self) -> bool {
        let text_agrees = match (&self.value_text, &other.value_text) {
            (Some(a), Some(b)) => a == b,
            _ => true,
        };
        self.value == other.value
            && text_agrees
            && self.unit == other.unit
            && self.system == other.system
            && self.code == other.code
    }
}

/// The JSON form: `value` is a JSON number, written with its original digits
/// when they are known.
#[derive(Serialize, Deserialize)]
struct QuantityWire {
    value: Box<serde_json::value::RawValue>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    code: Option<String>,
}

impl Serialize for Quantity {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let text = match &self.value_text {
            Some(text) => text.clone(),
            None => serde_json::to_string(&self.value).map_err(serde::ser::Error::custom)?,
        };
        let value =
            serde_json::value::RawValue::from_string(text).map_err(serde::ser::Error::custom)?;
        QuantityWire {
            value,
            unit: self.unit.clone(),
            system: self.system.clone(),
            code: self.code.clone(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Quantity {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let wire = QuantityWire::deserialize(deserializer)?;
        let text = wire.value.get().to_owned();
        let value: f64 = serde_json::from_str(&text).map_err(serde::de::Error::custom)?;
        Ok(Self {
            value,
            value_text: Some(text),
            unit: wire.unit,
            system: wire.system,
            code: wire.code,
        })
    }
}

impl Quantity {
    /// Construct a quantity with a value and human-readable unit.
    #[must_use]
    pub fn new(value: f64, unit: impl Into<String>) -> Self {
        Self {
            value,
            value_text: None,
            unit: Some(unit.into()),
            system: None,
            code: None,
        }
    }
}

/// Check that a parsed JSON resource declares the type it is read as.
pub(crate) fn check_resource_type(
    actual: &str,
    expected: &str,
) -> Result<(), crate::validate::FhirValidationError> {
    if actual == expected {
        return Ok(());
    }
    Err(crate::validate::FhirValidationError::new(
        "/resourceType",
        format!("expected `resourceType: \"{expected}\"`, found `{actual}`"),
    ))
}

/// Check an FHIR `date`: `YYYY`, `YYYY-MM` or `YYYY-MM-DD`, with a real month
/// and day (leap years included).
///
/// # Errors
/// Returns an error naming `field` if the value is not such a date.
pub(crate) fn check_date(
    field: &str,
    value: &str,
) -> Result<(), crate::validate::FhirValidationError> {
    if is_date(value) {
        Ok(())
    } else {
        Err(date_error(
            field,
            value,
            "a date (YYYY, YYYY-MM or YYYY-MM-DD)",
        ))
    }
}

/// Check an FHIR `dateTime`: a date, optionally followed by `T`, a time
/// (`hh:mm`, `hh:mm:ss` or with fractional seconds) and a timezone (`Z` or
/// `±hh:mm`).
///
/// # Errors
/// Returns an error naming `field` if the value is not such a date-time.
pub(crate) fn check_date_time(
    field: &str,
    value: &str,
) -> Result<(), crate::validate::FhirValidationError> {
    if is_date_time(value) {
        Ok(())
    } else {
        Err(date_error(field, value, "a dateTime"))
    }
}

fn date_error(field: &str, value: &str, expected: &str) -> crate::validate::FhirValidationError {
    crate::validate::FhirValidationError::new(field, format!("`{value}` is not {expected}"))
}

fn is_date(value: &str) -> bool {
    let parts: Vec<&str> = value.split('-').collect();
    let digits = |s: &str, n: usize| s.len() == n && s.bytes().all(|b| b.is_ascii_digit());
    match parts.as_slice() {
        [year] => digits(year, 4),
        [year, month] => {
            digits(year, 4)
                && digits(month, 2)
                && (1..=12).contains(&month.parse::<u32>().unwrap_or(0))
        }
        [year, month, day] => {
            if !(digits(year, 4) && digits(month, 2) && digits(day, 2)) {
                return false;
            }
            let (Ok(y), Ok(m), Ok(d)) = (
                year.parse::<u32>(),
                month.parse::<u32>(),
                day.parse::<u32>(),
            ) else {
                return false;
            };
            (1..=12).contains(&m) && d >= 1 && d <= days_in_month(y, m)
        }
        _ => false,
    }
}

fn is_date_time(value: &str) -> bool {
    let (date, rest) = match value.split_once('T') {
        Some((date, rest)) => (date, Some(rest)),
        None => (value, None),
    };
    if !is_date(date) {
        return false;
    }
    let Some(rest) = rest else {
        return true;
    };
    // Split the timezone off the time: `Z`, or a `+`/`-` offset.
    let (time, zone) = match rest.find(['Z', '+', '-']) {
        Some(at) => (&rest[..at], &rest[at..]),
        None => (rest, ""),
    };
    let zone_ok = match zone {
        "" | "Z" => true,
        offset => {
            let body = &offset[1..];
            body.len() == 5
                && body.as_bytes()[2] == b':'
                && body[..2].parse::<u32>().is_ok_and(|h| h <= 14)
                && body[3..].parse::<u32>().is_ok_and(|m| m <= 59)
        }
    };
    let (clock, fraction) = match time.split_once('.') {
        Some((clock, fraction)) => (clock, Some(fraction)),
        None => (time, None),
    };
    let fraction_ok =
        fraction.is_none_or(|f| !f.is_empty() && f.bytes().all(|b| b.is_ascii_digit()));
    let fields: Vec<&str> = clock.split(':').collect();
    let clock_ok = match fields.as_slice() {
        [h, m] | [h, m, _] => {
            let hour = h.parse::<u32>().ok();
            let minute = m.parse::<u32>().ok();
            h.len() == 2
                && m.len() == 2
                && hour.is_some_and(|x| x <= 23)
                && minute.is_some_and(|x| x <= 59)
                && fields
                    .get(2)
                    .is_none_or(|s| s.len() == 2 && s.parse::<u32>().is_ok_and(|x| x <= 60))
        }
        _ => false,
    };
    zone_ok && fraction_ok && clock_ok
}

fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year.is_multiple_of(400) || (year.is_multiple_of(4) && !year.is_multiple_of(100)) => {
            29
        }
        _ => 28,
    }
}
