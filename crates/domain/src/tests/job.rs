use super::*;

#[test]
fn validates_the_version_one_fixture() {
    let snapshot: JobSnapshot = serde_json::from_str(include_str!(
        "../../../../schemas/fixtures/job-snapshot-v1.json"
    ))
    .expect("job fixture should deserialize");
    snapshot.validate().expect("job fixture should validate");
    assert_eq!(snapshot.jobs[0].cargo_weight_lb(), Some(4_000.0));
    assert_eq!(snapshot.jobs[1].passenger_count(), Some(8));
}

#[test]
fn rejects_duplicate_jobs_and_non_monotonic_legs() {
    let mut snapshot: JobSnapshot = serde_json::from_str(include_str!(
        "../../../../schemas/fixtures/job-snapshot-v1.json"
    ))
    .unwrap();
    snapshot.jobs.push(snapshot.jobs[0].clone());
    assert_eq!(
        snapshot.validate(),
        Err(JobValidationError::InvalidSnapshot)
    );

    snapshot.jobs.pop();
    snapshot.jobs[0].legs[1].sequence = 0;
    assert_eq!(snapshot.validate(), Err(JobValidationError::InvalidLegs));
}

#[test]
fn rejects_unbounded_invalid_and_unsafe_job_facts() {
    let fixture = include_str!("../../../../schemas/fixtures/job-snapshot-v1.json");
    let baseline = || serde_json::from_str::<JobSnapshot>(fixture).unwrap();

    let mut oversized = baseline();
    oversized.jobs = (0..=MAX_JOBS_PER_SNAPSHOT)
        .map(|_| {
            let mut job = oversized.jobs[0].clone();
            job.id = JobId(Uuid::new_v4());
            job
        })
        .collect();
    assert_eq!(
        oversized.validate(),
        Err(JobValidationError::InvalidSnapshot)
    );

    let mut invalid_number = baseline();
    invalid_number.jobs[0].legs[0].cargo_weight_lb = Some(f64::NAN);
    assert_eq!(
        invalid_number.validate(),
        Err(JobValidationError::InvalidLegs)
    );

    let mut unsafe_text = baseline();
    unsafe_text.jobs[0].description = Some("hidden\u{0000}control".into());
    assert_eq!(unsafe_text.validate(), Err(JobValidationError::InvalidText));

    let mut duplicate_leg = baseline();
    duplicate_leg.jobs[0].legs[1].id = duplicate_leg.jobs[0].legs[0].id.clone();
    assert_eq!(
        duplicate_leg.validate(),
        Err(JobValidationError::InvalidLegs)
    );
}
