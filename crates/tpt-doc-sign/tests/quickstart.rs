//! Integration tests for the workspace README quick-start.
//!
//! The `## Quick Start` block in the repository README is the first thing a
//! new user copies. It lives in this crate because it is the only one that
//! combines an FHIR resource with the signing fixtures, so the snippet can be
//! exercised end to end rather than merely type-checked in isolation.

use ring::rand::SystemRandom;
use tpt_doc_fhir::prelude::*;
use tpt_doc_sign::{cades, pem, PrivateKey};

/// The README snippet, verbatim apart from the PEM paths and the error type.
#[test]
fn readme_quick_start_compiles_and_runs() -> Result<(), Box<dyn std::error::Error>> {
    // Build a strictly typed FHIR Patient resource. `build()` only exists once
    // every mandatory field is set, so an incomplete patient will not compile.
    let patient = Patient::builder()
        .id("nz-12345")
        .name(HumanName::new("Smith", "John"))
        .build();

    // Serialize to compliant FHIR JSON.
    let json_bytes = patient.to_json()?;
    assert!(
        String::from_utf8_lossy(&json_bytes).contains(r#""resourceType": "Patient""#),
        "serialized patient must carry its resourceType"
    );

    // Detached CAdES signature over those bytes, for HIPC 2020 audit trails.
    let key = PrivateKey::from_pem(include_bytes!("fixtures/test_key.pem"))?;
    let cert_der = pem::pem_to_der(include_bytes!("fixtures/test_cert.pem"))?;
    let signature =
        cades::sign_detached(&json_bytes, &key, &cert_der, &SystemRandom::new())?;
    assert!(!signature.is_empty(), "signature must not be empty");

    Ok(())
}
