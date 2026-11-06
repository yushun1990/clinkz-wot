use clinkz_wot_core::{collection::*, data_schema::PreparedSchema};

fn schema() -> PropertyCollectionSchema {
    PropertyCollectionSchema::try_new(
        ["a", "b", "c"]
            .into_iter()
            .map(|name| PropertyMemberSchema {
                name: name.into(),
                schema: PreparedSchema::Any,
            })
            .collect(),
    )
    .unwrap()
}

#[test]
fn member_evidence_keeps_unknown_separate_from_rejection_and_atomicity() {
    let s = schema();
    let report = PropertyWriteReport::try_new(s.all(), 1, 2, WriteAtomicity::NonAtomic)
        .unwrap()
        .with_native_status(0, 0)
        .unwrap()
        .with_native_status(1, 712)
        .unwrap();
    assert_eq!(report.effect(0), Some(MemberWriteEffect::Completed));
    assert_eq!(report.effect(1), Some(MemberWriteEffect::NotApplied));
    assert_eq!(report.effect(2), Some(MemberWriteEffect::Unknown));
    assert_eq!(report.effect(32), None);
    assert_eq!(report.native_status(0), Some(0));
    assert_eq!(report.native_status(1), Some(712));
    assert_eq!(report.native_status(2), None);
    assert_eq!(report.native_status(usize::MAX), None);
    assert!(report.completion().is_err());
    assert!(
        report
            .validate_selection(s.select(&["a"]).unwrap())
            .is_err()
    );
    assert!(report.with_native_status(0, 2).is_err());
    assert!(report.with_native_status(32, 2).is_err());
    assert!(PropertyWriteReport::try_new(s.all(), 1, 2, WriteAtomicity::Atomic).is_err());
    assert!(PropertyWriteReport::try_new(s.all(), 1, 0, WriteAtomicity::Atomic).is_err());
    assert!(PropertyWriteReport::try_new(s.all(), 0, 2, WriteAtomicity::Atomic).is_err());
    assert!(PropertyWriteReport::try_new(s.all(), 1, 1, WriteAtomicity::Unspecified).is_err());
    assert!(
        PropertyWriteReport::try_new(s.select(&["a"]).unwrap(), 2, 0, WriteAtomicity::Unspecified)
            .is_err()
    );
    let rolled_back = PropertyWriteReport::try_new(s.all(), 0, 7, WriteAtomicity::Atomic).unwrap();
    assert_eq!(rolled_back.atomicity(), WriteAtomicity::Atomic);
    assert!(rolled_back.completion().is_err());
    assert!(
        PropertyWriteReport::try_new(s.all(), 7, 0, WriteAtomicity::Atomic)
            .unwrap()
            .completion()
            .is_ok()
    );
}
