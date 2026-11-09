//! Reading a 997 back: what `build_997` writes, `parse_997` reads.

use tpt_doc_edi::x12_ack::{build_997, parse_997};

const REMITTANCE: &[u8] = include_bytes!("fixtures/sample_835.x12");

#[test]
fn an_accepted_997_reads_back_with_its_group_and_set() {
    let ack = build_997(REMITTANCE, true, "000000042", "260101", "0930").expect("build");
    let groups = parse_997(&ack).expect("parse");
    assert_eq!(groups.len(), 1);
    let group = &groups[0];
    assert_eq!(group.functional_id, "HP");
    assert_eq!(group.group_control, "1");
    assert!(group.is_accepted());
    assert_eq!(group.sets.len(), 1);
    assert_eq!(group.sets[0].set_id, "835");
    assert_eq!(group.sets[0].set_control, "0001");
    assert_eq!(group.sets[0].code, "A");
    assert_eq!(
        (
            group.sets_included,
            group.sets_received,
            group.sets_accepted
        ),
        (1, 1, 1)
    );
}

#[test]
fn a_rejected_997_is_not_accepted() {
    let ack = build_997(REMITTANCE, false, "000000043", "260101", "0931").expect("build");
    let groups = parse_997(&ack).expect("parse");
    assert!(!groups[0].is_accepted());
    assert_eq!(groups[0].sets[0].code, "R");
    assert_eq!(groups[0].sets_accepted, 0);
}

#[test]
fn an_ak9_count_that_disagrees_with_the_sets_is_refused() {
    let ack = build_997(REMITTANCE, true, "000000042", "260101", "0930").expect("build");
    let text = String::from_utf8(ack)
        .expect("ascii")
        .replace("AK9*A*1*1*1", "AK9*A*2*2*2");
    let err = parse_997(text.as_bytes()).expect_err("count mismatch");
    assert!(err.to_string().contains("counts 2 sets"), "{err}");
}

#[test]
fn a_group_without_an_ak9_is_refused() {
    let ack = build_997(REMITTANCE, true, "000000042", "260101", "0930").expect("build");
    let text = String::from_utf8(ack)
        .expect("ascii")
        .split('~')
        .filter(|s| !s.starts_with("AK9"))
        .collect::<Vec<_>>()
        .join("~");
    assert!(parse_997(text.as_bytes()).is_err());
}
