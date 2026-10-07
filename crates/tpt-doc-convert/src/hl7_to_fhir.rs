//! HL7 v2 (ADT / ORU) to FHIR R5 conversion.
//!
//! | HL7 v2 | FHIR |
//! |---|---|
//! | `PID` | `Patient` (identifier, name, gender, birthDate) |
//! | `PV1` (ADT) | `Encounter` (class, status, period) |
//! | `OBX` (ORU) | `Observation` (code, status, valueQuantity, effective) |
//!
//! Anything that cannot be represented faithfully is reported in
//! [`Conversion::warnings`] rather than silently guessed or dropped.

use tpt_doc_core::{DocError, ValidationReport};
use tpt_doc_fhir::{
    Bundle, CodeableConcept, Coding, Encounter, EncounterStatus, Gender, HumanName, Identifier,
    Observation, ObservationStatus, Patient, Period, Quantity, Reference, Resource,
    validate_encounter, validate_observation,
};
use tpt_doc_hl7v2::{Hl7Parser, Segment};

const V3_ACT_CODE: &str = "http://terminology.hl7.org/CodeSystem/v3-ActCode";

/// The outcome of a conversion.
#[derive(Debug, Clone)]
pub struct Conversion {
    /// The resulting `collection` bundle (Patient first, then Encounter or
    /// Observations in message order).
    pub bundle: Bundle,
    /// Data that could not be mapped faithfully, in message order.
    pub warnings: Vec<String>,
    /// FHIR invariant findings on the generated resources.
    pub report: ValidationReport,
}

/// Convert an HL7 v2 `ADT^*` or `ORU^R01` message into a FHIR bundle.
///
/// # Errors
/// Returns [`DocError`] if the message does not parse, has no `MSH`, or is of
/// an unsupported type (only `ADT` and `ORU` are converted).
pub fn hl7v2_to_fhir(message: &[u8]) -> Result<Conversion, DocError> {
    let segments: Vec<Segment<'_>> = Hl7Parser::new(message).collect::<Result<_, _>>()?;
    let msh = segments
        .first()
        .filter(|s| s.tag() == "MSH")
        .ok_or_else(|| DocError::invalid_format("HL7 message must start with MSH"))?;
    let msg_type = msh.component(9, 1).unwrap_or_default().to_owned();
    let trigger = msh.component(9, 2).unwrap_or_default().to_owned();
    let control = msh.field(10).unwrap_or_default().to_owned();

    let mut ctx = Ctx {
        control: sanitize_id(&control, "msg"),
        warnings: Vec::new(),
    };
    let mut entries = Vec::new();

    let patient = segments
        .iter()
        .find(|s| s.tag() == "PID")
        .map(|pid| ctx.patient(pid));
    let patient_ref = patient
        .as_ref()
        .map(|p| Reference::to(format!("Patient/{}", p.id)));
    if let Some(p) = patient {
        entries.push(Resource::Patient(p));
    } else {
        ctx.warn("message has no PID segment; resources have no subject");
    }

    match msg_type.as_str() {
        "ADT" => {
            let evn_time = segments
                .iter()
                .find(|s| s.tag() == "EVN")
                .and_then(|s| s.field(2));
            if let Some(pv1) = segments.iter().find(|s| s.tag() == "PV1") {
                if let Some(enc) = ctx.encounter(pv1, &trigger, evn_time, patient_ref.as_ref()) {
                    entries.push(Resource::Encounter(enc));
                }
            } else {
                ctx.warn("ADT message has no PV1 segment; no Encounter produced");
            }
        }
        "ORU" => {
            let obr_time = segments
                .iter()
                .find(|s| s.tag() == "OBR")
                .and_then(|s| s.field(7));
            for obx in segments.iter().filter(|s| s.tag() == "OBX") {
                if let Some(o) = ctx.observation(obx, obr_time, patient_ref.as_ref()) {
                    entries.push(Resource::Observation(o));
                }
            }
        }
        other => {
            return Err(DocError::invalid_format(format!(
                "unsupported HL7 message type `{other}` (only ADT and ORU are converted)"
            )));
        }
    }

    let mut report = ValidationReport::default();
    for entry in &entries {
        match entry {
            Resource::Observation(o) => report.merge(validate_observation(o)),
            Resource::Encounter(e) => report.merge(validate_encounter(e)),
            Resource::Patient(_) | Resource::Organization(_) => {}
        }
    }
    Ok(Conversion {
        bundle: Bundle::collection(entries),
        warnings: ctx.warnings,
        report,
    })
}

struct Ctx {
    control: String,
    warnings: Vec<String>,
}

impl Ctx {
    fn warn(&mut self, message: impl Into<String>) {
        self.warnings.push(message.into());
    }

    fn patient(&mut self, pid: &Segment<'_>) -> Patient {
        let mut identifiers = Vec::new();
        for raw in repeats(pid, 3) {
            let id = comp(raw, pid, 1);
            if id.is_empty() {
                continue;
            }
            let authority = comp(raw, pid, 4);
            identifiers.push(Identifier {
                system: (!authority.is_empty()).then(|| format!("urn:hl7v2:{authority}")),
                value: id,
            });
        }
        let id = identifiers
            .first()
            .map(|i| sanitize_id(&i.value, "patient"))
            .unwrap_or_else(|| {
                self.warn("PID-3 has no identifier; generated a placeholder Patient id");
                format!("{}-patient", self.control)
            });

        let mut builder = Patient::builder().id(id);
        for identifier in identifiers {
            builder = builder.identifier(identifier);
        }
        for raw in repeats(pid, 5) {
            let family = comp(raw, pid, 1);
            let given = comp(raw, pid, 2);
            if !family.is_empty() || !given.is_empty() {
                let mut name = HumanName::new(family, given);
                if name.family.as_deref() == Some("") {
                    name.family = None;
                }
                if name.given.iter().all(String::is_empty) {
                    name.given.clear();
                }
                builder = builder.name(name);
            }
        }
        if let Some(raw) = pid.field(7).filter(|v| !v.is_empty()) {
            match fhir_date(raw) {
                Some(date) => builder = builder.birth_date(date),
                None => self.warn(format!("PID-7 `{raw}` is not a valid HL7 date; dropped")),
            }
        }
        match pid.field(8).map(str::trim).unwrap_or_default() {
            "" => {}
            "M" => builder = builder.gender(Gender::Male),
            "F" => builder = builder.gender(Gender::Female),
            "O" => builder = builder.gender(Gender::Other),
            "U" => builder = builder.gender(Gender::Unknown),
            other => {
                self.warn(format!(
                    "PID-8 `{other}` has no FHIR gender mapping; dropped"
                ));
            }
        }
        builder.build()
    }

    fn encounter(
        &mut self,
        pv1: &Segment<'_>,
        trigger: &str,
        evn_time: Option<&str>,
        subject: Option<&Reference>,
    ) -> Option<Encounter> {
        let class_code = match pv1.field(2).map(str::trim).unwrap_or_default() {
            "I" => ("IMP", "inpatient encounter"),
            "O" => ("AMB", "ambulatory"),
            "E" => ("EMER", "emergency"),
            "P" => ("PRENC", "pre-admission"),
            other => {
                self.warn(format!(
                    "PV1-2 patient class `{other}` has no FHIR mapping; no Encounter produced"
                ));
                return None;
            }
        };
        let status = match trigger {
            "A03" => EncounterStatus::Finished,
            "A11" | "A13" => EncounterStatus::Cancelled,
            "A05" | "A14" | "A15" => EncounterStatus::Planned,
            _ => EncounterStatus::InProgress,
        };
        let visit = comp(pv1.field(19).unwrap_or_default(), pv1, 1);
        let id = if visit.is_empty() {
            format!("{}-enc", self.control)
        } else {
            sanitize_id(&visit, "enc")
        };

        let mut builder = Encounter::builder()
            .status(status)
            .class(Coding::new(V3_ACT_CODE, class_code.0).display(class_code.1))
            .id(id);
        if let Some(subject) = subject {
            builder = builder.subject(subject.clone());
        }
        let start = pv1
            .field(44)
            .filter(|v| !v.is_empty())
            .or(evn_time.filter(|v| !v.is_empty()));
        let end = pv1.field(45).filter(|v| !v.is_empty());
        let start = start.and_then(|v| self.datetime(v, "PV1-44/EVN-2"));
        let end = end.and_then(|v| self.datetime(v, "PV1-45"));
        if start.is_some() || end.is_some() {
            builder = builder.period(Period { start, end });
        }
        Some(builder.build())
    }

    fn observation(
        &mut self,
        obx: &Segment<'_>,
        obr_time: Option<&str>,
        subject: Option<&Reference>,
    ) -> Option<Observation> {
        let set_id = obx.field(1).unwrap_or_default();
        let identifier = comp(obx.field(3).unwrap_or_default(), obx, 1);
        if identifier.is_empty() {
            self.warn(format!(
                "OBX-{set_id}: OBX-3 has no observation code; skipped"
            ));
            return None;
        }
        let text = comp(obx.field(3).unwrap_or_default(), obx, 2);
        let system = comp(obx.field(3).unwrap_or_default(), obx, 3);
        let mut coding = Coding::new(
            if system.is_empty() {
                "urn:hl7v2:local".to_owned()
            } else {
                format!("urn:hl7v2:{system}")
            },
            identifier,
        );
        if !text.is_empty() {
            coding = coding.display(text);
        }

        let status = match obx.field(11).map(str::trim).unwrap_or_default() {
            "F" => ObservationStatus::Final,
            "P" | "R" | "I" | "S" => ObservationStatus::Preliminary,
            "O" => ObservationStatus::Registered,
            "C" => ObservationStatus::Corrected,
            "X" => ObservationStatus::Cancelled,
            "D" | "W" => ObservationStatus::EnteredInError,
            other => {
                self.warn(format!(
                    "OBX-{set_id}: result status `{other}` is unknown; mapped to `unknown`"
                ));
                ObservationStatus::Unknown
            }
        };
        let id = format!("{}-{}", self.control, sanitize_id(set_id, "obx"));
        let mut builder = Observation::builder()
            .status(status)
            .code(CodeableConcept::from_coding(coding))
            .id(id);
        if let Some(subject) = subject {
            builder = builder.subject(subject.clone());
        }

        let value = obx.field(5).unwrap_or_default();
        match obx.field(2).map(str::trim).unwrap_or_default() {
            "NM" if !value.is_empty() => match value.trim().parse::<f64>() {
                Ok(number) if number.is_finite() => {
                    let unit = comp(obx.field(6).unwrap_or_default(), obx, 1);
                    builder = builder.value_quantity(Quantity::new(number, unit));
                }
                _ => self.warn(format!(
                    "OBX-{set_id}: numeric value `{value}` is not a number; dropped"
                )),
            },
            "" | "NM" => {}
            other => {
                if !value.is_empty() {
                    self.warn(format!(
                        "OBX-{set_id}: value type `{other}` is not representable \
                         (only NM maps to valueQuantity); value dropped"
                    ));
                }
            }
        }

        let effective = obx
            .field(14)
            .filter(|v| !v.is_empty())
            .or(obr_time.filter(|v| !v.is_empty()));
        if let Some(raw) = effective {
            if let Some(dt) = self.datetime(raw, "OBX-14/OBR-7") {
                builder = builder.effective_date_time(dt);
            }
        }
        Some(builder.build())
    }

    /// Convert an HL7 timestamp to a FHIR dateTime, warning when information
    /// has to be dropped.
    fn datetime(&mut self, raw: &str, field: &str) -> Option<String> {
        match fhir_datetime(raw) {
            Ok((value, lossy)) => {
                if lossy {
                    self.warn(format!(
                        "{field} `{raw}` has a time but no UTC offset; FHIR requires one, \
                         so only the date was kept"
                    ));
                }
                Some(value)
            }
            Err(()) => {
                self.warn(format!(
                    "{field} `{raw}` is not a valid HL7 timestamp; dropped"
                ));
                None
            }
        }
    }
}

/// Repetitions of field `n`, skipping empties.
fn repeats<'a>(segment: &Segment<'a>, n: usize) -> Vec<&'a str> {
    let sep = char::from(segment.delimiters().repeat);
    segment
        .field(n)
        .map(|f| f.split(sep).filter(|r| !r.is_empty()).collect())
        .unwrap_or_default()
}

/// Component `m` (1-based) of a raw field value, unescaped, without
/// subcomponents.
fn comp(raw: &str, segment: &Segment<'_>, m: usize) -> String {
    let d = segment.delimiters();
    let part = raw.split(char::from(d.component)).nth(m - 1).unwrap_or("");
    let part = part.split(char::from(d.subcomponent)).next().unwrap_or("");
    unescape(part, segment)
}

/// Resolve the standard HL7 escape sequences (`\F\ \S\ \R\ \T\ \E\`).
fn unescape(text: &str, segment: &Segment<'_>) -> String {
    let d = segment.delimiters();
    let esc = char::from(d.escape);
    if !text.contains(esc) {
        return text.to_owned();
    }
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find(esc) {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        match after.find(esc) {
            Some(end) => {
                let replacement = match &after[..end] {
                    "F" => Some(char::from(d.field)),
                    "S" => Some(char::from(d.component)),
                    "R" => Some(char::from(d.repeat)),
                    "T" => Some(char::from(d.subcomponent)),
                    "E" => Some(esc),
                    _ => None,
                };
                match replacement {
                    Some(c) => out.push(c),
                    // Unknown sequence (e.g. `\.br\`): keep it verbatim.
                    None => out.push_str(&rest[start..start + end + 2]),
                }
                rest = &after[end + 1..];
            }
            None => {
                out.push_str(&rest[start..]);
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out
}

/// FHIR ids are `[A-Za-z0-9\-.]{1,64}`.
fn sanitize_id(raw: &str, fallback: &str) -> String {
    let id: String = raw
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '.' {
                c
            } else {
                '-'
            }
        })
        .take(64)
        .collect();
    if id.is_empty() || id.chars().all(|c| c == '-') {
        fallback.to_owned()
    } else {
        id
    }
}

/// `YYYY[MM[DD]]` prefix of an HL7 timestamp as a FHIR date.
fn fhir_date(raw: &str) -> Option<String> {
    let digits: String = raw.chars().take_while(char::is_ascii_digit).collect();
    let date = match digits.len() {
        4 => digits.clone(),
        6 => format!("{}-{}", &digits[..4], &digits[4..6]),
        n if n >= 8 => format!("{}-{}-{}", &digits[..4], &digits[4..6], &digits[6..8]),
        _ => return None,
    };
    if digits.len() >= 6 && !(1..=12).contains(&digits[4..6].parse::<u32>().ok()?) {
        return None;
    }
    if digits.len() >= 8 && !(1..=31).contains(&digits[6..8].parse::<u32>().ok()?) {
        return None;
    }
    Some(date)
}

/// HL7 `TS` to a FHIR dateTime. The flag is `true` when a time was present but
/// had to be dropped because no UTC offset accompanied it.
fn fhir_datetime(raw: &str) -> Result<(String, bool), ()> {
    let raw = raw.trim();
    let date = fhir_date(raw).ok_or(())?;
    let digits: String = raw.chars().take_while(char::is_ascii_digit).collect();
    if digits.len() < 10 {
        return Ok((date, false));
    }
    let hh = &digits[8..10];
    let mm = digits.get(10..12).unwrap_or("00");
    let ss = digits.get(12..14).unwrap_or("00");
    if hh.parse::<u32>().map_err(|_| ())? > 23
        || mm.parse::<u32>().map_err(|_| ())? > 59
        || ss.parse::<u32>().map_err(|_| ())? > 59
    {
        return Err(());
    }
    // Offset follows the digits and optional fractional seconds.
    let tail = raw[digits.len()..].trim_start_matches(|c: char| c == '.' || c.is_ascii_digit());
    let offset = match tail.as_bytes() {
        [b'+' | b'-', a, b, c, d] if [a, b, c, d].iter().all(|x| x.is_ascii_digit()) => {
            Some(format!("{}{}:{}", &tail[..1], &tail[1..3], &tail[3..5]))
        }
        [b'Z'] => Some("Z".to_owned()),
        _ => None,
    };
    match offset {
        Some(offset) => Ok((format!("{date}T{hh}:{mm}:{ss}{offset}"), false)),
        None => Ok((date, true)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamps() {
        assert_eq!(fhir_datetime("2026").unwrap(), ("2026".into(), false));
        assert_eq!(fhir_datetime("202610").unwrap(), ("2026-10".into(), false));
        assert_eq!(
            fhir_datetime("20261001").unwrap(),
            ("2026-10-01".into(), false)
        );
        // A time with no offset cannot be a valid FHIR dateTime.
        assert_eq!(
            fhir_datetime("20261001120000").unwrap(),
            ("2026-10-01".into(), true)
        );
        assert_eq!(
            fhir_datetime("20261001120000+1300").unwrap(),
            ("2026-10-01T12:00:00+13:00".into(), false)
        );
        assert_eq!(
            fhir_datetime("202610011200.5-0500").unwrap(),
            ("2026-10-01T12:00:00-05:00".into(), false)
        );
        assert!(fhir_datetime("2026130101").is_err());
        assert!(fhir_datetime("abc").is_err());
        assert!(fhir_datetime("20261001256100+0000").is_err());
    }

    #[test]
    fn ids_are_sanitised() {
        assert_eq!(sanitize_id("a b/c_d", "x"), "a-b-c-d");
        assert_eq!(sanitize_id("", "x"), "x");
        assert_eq!(sanitize_id("///", "x"), "x");
        assert_eq!(sanitize_id(&"a".repeat(100), "x").len(), 64);
    }
}
