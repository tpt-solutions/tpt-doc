use crate::FhirValidationError;
use crate::encounter::Encounter;
use crate::observation::Observation;
use crate::organization::Organization;
use crate::patient::Patient;

/// A typed FHIR resource: any supported resource type.
///
/// The `resourceType` discriminant in JSON selects the variant.
#[derive(Debug, Clone, PartialEq)]
pub enum Resource {
    /// A `Patient` resource.
    Patient(Patient),
    /// An `Observation` resource.
    Observation(Observation),
    /// An `Encounter` resource.
    Encounter(Encounter),
    /// An `Organization` resource.
    Organization(Organization),
}

impl Resource {
    /// The `resourceType` discriminant value.
    #[must_use]
    pub fn resource_type(&self) -> &'static str {
        match self {
            Self::Patient(_) => "Patient",
            Self::Observation(_) => "Observation",
            Self::Encounter(_) => "Encounter",
            Self::Organization(_) => "Organization",
        }
    }

    fn to_value(&self) -> Result<serde_json::Value, FhirValidationError> {
        let value = match self {
            Self::Patient(r) => serde_json::to_value(r),
            Self::Observation(r) => serde_json::to_value(r),
            Self::Encounter(r) => serde_json::to_value(r),
            Self::Organization(r) => serde_json::to_value(r),
        }
        .map_err(|e| FhirValidationError::new("/", format!("JSON serialization failed: {e}")))?;
        Ok(value)
    }

    fn from_value(value: &serde_json::Value) -> Result<Self, FhirValidationError> {
        fn from<T: serde::de::DeserializeOwned>(
            v: &serde_json::Value,
        ) -> Result<T, FhirValidationError> {
            serde_json::from_value(v.clone()).map_err(|e| {
                FhirValidationError::new("/", format!("JSON deserialization failed: {e}"))
            })
        }
        match value
            .get("resourceType")
            .and_then(serde_json::Value::as_str)
        {
            Some("Patient") => from(value).map(Self::Patient),
            Some("Observation") => from(value).map(Self::Observation),
            Some("Encounter") => from(value).map(Self::Encounter),
            Some("Organization") => from(value).map(Self::Organization),
            Some(other) => Err(FhirValidationError::new(
                "/entry/*/resource/resourceType",
                format!("unsupported resource type `{other}`"),
            )),
            None => Err(FhirValidationError::new(
                "/entry/*/resource/resourceType",
                "missing `resourceType` discriminant",
            )),
        }
    }
}

/// A FHIR Bundle: a collection of resources (R5 `Bundle`).
///
/// Only `collection`-style bundles are modelled — entries hold resources
/// without request/search metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct Bundle {
    /// Logical id of the bundle.
    pub id: Option<String>,
    /// Bundle type code — `"collection"` for this crate's output.
    pub bundle_type: String,
    /// Resources in the bundle, in entry order.
    pub entries: Vec<Resource>,
}

impl Bundle {
    /// Create a `collection` bundle with the given entries.
    #[must_use]
    pub fn collection(entries: Vec<Resource>) -> Self {
        Self {
            id: None,
            bundle_type: "collection".to_owned(),
            entries,
        }
    }

    /// Count entries of each resource type, in fixed resource-type order.
    #[must_use]
    pub fn counts(&self) -> Vec<(&'static str, usize)> {
        ["Patient", "Observation", "Encounter", "Organization"]
            .into_iter()
            .map(|rt| {
                (
                    rt,
                    self.entries
                        .iter()
                        .filter(|e| e.resource_type() == rt)
                        .count(),
                )
            })
            .collect()
    }

    /// Serialize to FHIR-compliant JSON bytes.
    ///
    /// # Errors
    /// Returns an error if JSON serialization fails.
    pub fn to_json(&self) -> Result<Vec<u8>, FhirValidationError> {
        let mut entries = Vec::with_capacity(self.entries.len());
        for resource in &self.entries {
            entries.push(serde_json::json!({
                "resource": resource.to_value()?,
            }));
        }
        let mut value = serde_json::json!({
            "resourceType": "Bundle",
            "type": self.bundle_type,
            "entry": entries,
        });
        if let Some(id) = &self.id {
            value["id"] = serde_json::json!(id);
        }
        serde_json::to_vec_pretty(&value)
            .map_err(|e| FhirValidationError::new("/", format!("JSON serialization failed: {e}")))
    }

    /// Deserialize from FHIR-compliant JSON bytes.
    ///
    /// Walks `entry[].resource`, dispatching on the `resourceType`
    /// discriminant of each entry.
    ///
    /// # Errors
    /// Returns an error if the bytes are not a valid Bundle JSON document or
    /// an entry uses an unsupported resource type.
    pub fn from_json(bytes: &[u8]) -> Result<Self, FhirValidationError> {
        let value: serde_json::Value = serde_json::from_slice(bytes).map_err(|e| {
            FhirValidationError::new("/", format!("JSON deserialization failed: {e}"))
        })?;
        if value
            .get("resourceType")
            .and_then(serde_json::Value::as_str)
            != Some("Bundle")
        {
            return Err(FhirValidationError::new(
                "/resourceType",
                "expected `resourceType: \"Bundle\"`",
            ));
        }
        let empty = Vec::new();
        let entry_list = value
            .get("entry")
            .and_then(serde_json::Value::as_array)
            .unwrap_or(&empty);
        let entries = entry_list
            .iter()
            .map(|entry| {
                entry
                    .get("resource")
                    .ok_or_else(|| {
                        FhirValidationError::new(
                            "/entry/resource",
                            "bundle entry is missing its resource",
                        )
                    })
                    .and_then(Resource::from_value)
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            id: value
                .get("id")
                .and_then(serde_json::Value::as_str)
                .map(ToOwned::to_owned),
            bundle_type: value
                .get("type")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("collection")
                .to_owned(),
            entries,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::HumanName;

    #[test]
    fn collection_bundle_round_trip() {
        let bundle = Bundle::collection(vec![
            Resource::Patient(
                Patient::builder()
                    .id("nz-12345")
                    .name(HumanName::new("Smith", "John"))
                    .build(),
            ),
            Resource::Organization(
                Organization::builder()
                    .id("org-1")
                    .name("ACME Health")
                    .build(),
            ),
        ]);
        let restored = Bundle::from_json(&bundle.to_json().expect("json")).expect("parse");
        assert_eq!(bundle, restored);
        assert_eq!(
            bundle.counts(),
            vec![
                ("Patient", 1),
                ("Observation", 0),
                ("Encounter", 0),
                ("Organization", 1)
            ]
        );
    }

    #[test]
    fn non_bundle_json_is_rejected() {
        let error = Bundle::from_json(br#"{"resourceType": "Patient"}"#).expect_err("must fail");
        assert!(error.to_string().contains("Bundle"));
    }

    #[test]
    fn unsupported_resource_type_is_reported() {
        let bytes =
            br#"{"resourceType":"Bundle","entry":[{"resource":{"resourceType":"Medication"}}]}"#;
        let error = Bundle::from_json(bytes).expect_err("must fail");
        assert!(error.to_string().contains("Medication"));
    }
}
