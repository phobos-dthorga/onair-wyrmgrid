use uuid::Uuid;

use crate::{
    AircraftClassId, AircraftClassQualification, STAFF_SNAPSHOT_SCHEMA_VERSION, StaffMemberId,
    StaffMemberSummary, StaffQualificationId, StaffSnapshot, StaffValidationError,
};

fn member() -> StaffMemberSummary {
    StaffMemberSummary {
        id: StaffMemberId(Uuid::parse_str("11111111-1111-4111-8111-111111111111").unwrap()),
        display_name: Some("Sample Pilot".to_owned()),
        avatar_reference: Some("sample-pilot-avatar.png".to_owned()),
        category_code: Some(1),
        status_code: Some(2),
        current_airport: None,
        home_airport: None,
        busy_until: None,
        is_online: Some(false),
        class_qualifications: vec![AircraftClassQualification {
            id: StaffQualificationId(
                Uuid::parse_str("22222222-2222-4222-8222-222222222222").unwrap(),
            ),
            aircraft_class_id: AircraftClassId(
                Uuid::parse_str("33333333-3333-4333-8333-333333333333").unwrap(),
            ),
            short_name: Some("MEP".to_owned()),
            name: Some("Multi-engine piston".to_owned()),
            last_validated_at: None,
        }],
    }
}

#[test]
fn accepts_bounded_sourced_staff_with_unavailable_optional_facts() {
    let snapshot = StaffSnapshot {
        schema_version: STAFF_SNAPSHOT_SCHEMA_VERSION,
        staff: vec![member()],
    };
    assert_eq!(snapshot.validate(), Ok(()));
}

#[test]
fn rejects_unknown_provider_codes_and_duplicate_qualifications() {
    let mut invalid_code = member();
    invalid_code.status_code = Some(12);
    assert_eq!(
        invalid_code.validate(),
        Err(StaffValidationError::InvalidMember)
    );

    let mut duplicate = member();
    duplicate
        .class_qualifications
        .push(duplicate.class_qualifications[0].clone());
    assert_eq!(
        duplicate.validate(),
        Err(StaffValidationError::InvalidQualification)
    );
}

#[test]
fn rejects_unbounded_or_control_character_avatar_references() {
    let mut invalid = member();
    invalid.avatar_reference = Some("invalid\navatar.png".to_owned());
    assert_eq!(invalid.validate(), Err(StaffValidationError::InvalidMember));
}

#[test]
fn rejects_duplicate_staff_without_fabricating_identity() {
    let duplicate = member();
    let snapshot = StaffSnapshot {
        schema_version: STAFF_SNAPSHOT_SCHEMA_VERSION,
        staff: vec![duplicate.clone(), duplicate],
    };
    assert_eq!(
        snapshot.validate(),
        Err(StaffValidationError::InvalidSnapshot)
    );
}
