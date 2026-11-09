//! EDI tables loaded at runtime: structural errors name the line; semantic
//! checks stay with the validators.

use tpt_doc_core::Location;
use tpt_doc_edi::schema::{ElementTables, TransactionTables};

#[test]
fn element_table_loads_from_text() {
    assert!(ElementTables::from_tsv("BOG\t1\tan\t1\t9\tM\nBOG\t2\tn\t1\t3\tC\n").is_ok());
}

#[test]
fn unknown_element_type_is_refused_at_its_line() {
    let err = ElementTables::from_tsv("BOG\t1\tan\t1\t9\tM\nBOG\t2\tzz\t1\t3\tM\n")
        .err()
        .expect("zz is not an element type");
    assert!(err.to_string().contains("zz"), "{err}");
    assert_eq!(err.location(), Some(&Location::Index(2)));
}

#[test]
fn element_position_zero_is_not_a_load_error() {
    // Semantic: reported by the validator against the document, not here.
    assert!(ElementTables::from_tsv("BOG\t0\tn\t1\t9\tM\n").is_ok());
}

#[test]
fn transaction_table_requirement_must_be_m_or_o() {
    assert!(TransactionTables::from_tsv("835\tST\tM\n835\tSE\tO\n").is_ok());
    let err = TransactionTables::from_tsv("835\tST\tC\n")
        .err()
        .expect("C is not a transaction-set requirement");
    assert!(err.to_string().contains("M or O"), "{err}");
}
