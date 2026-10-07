//! Compiles only with `full`; checks every re-export resolves.
#![cfg(feature = "full")]

#[test]
fn every_format_is_reexported() {
    let _ = tpt_doc::pdf::Document::new;
    let _ = tpt_doc::spreadsheet::xlsx::XlsxWriter::new;
    let _ = tpt_doc::layout::Renderer::new;
    let _ = tpt_doc::hl7v2::prelude::Hl7Parser::new;
    let _ = tpt_doc::edi::edifact::EdifactParser::new;
    let _: Option<tpt_doc::sign::PrivateKey> = None;
    let _ = tpt_doc::core::Limits::default;
}
