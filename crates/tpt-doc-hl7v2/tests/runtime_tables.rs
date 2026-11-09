//! Tables loaded at runtime use the same validator as the embedded ones, and a
//! malformed table is refused with the line that caused it.

use tpt_doc_core::Location;
use tpt_doc_hl7v2::Hl7Parser;
use tpt_doc_hl7v2::schema::SegmentTables;

#[test]
fn runtime_table_validates_a_custom_segment() {
    let tables = SegmentTables::from_tsv("ZXX\t1\tM\tRequired thing\nZXX\t2\tO\tOptional\n")
        .expect("valid table");
    let segments: Vec<_> =
        Hl7Parser::new(b"MSH|^~\\&|A|B|C|D|20260101||ADT^A01|1|P|2.5.1\rZXX||x\r")
            .collect::<Result<_, _>>()
            .expect("parse");
    let violations = tables.validate_segment(1, &segments[1]);
    assert_eq!(violations.len(), 1, "{violations:?}");
    assert!(violations[0].message.contains("Required thing"));
}

#[test]
fn malformed_requirement_is_refused_at_its_line() {
    let err = SegmentTables::from_tsv("# comment\nZXX\t1\tX\tname\n")
        .err()
        .expect("requirement X is not valid");
    assert!(err.to_string().contains("M or O"), "{err}");
    assert_eq!(err.location(), Some(&Location::Index(2)));
}

#[test]
fn non_numeric_field_is_refused() {
    assert!(SegmentTables::from_tsv("ZXX\tfirst\tM\tname\n").is_err());
}
