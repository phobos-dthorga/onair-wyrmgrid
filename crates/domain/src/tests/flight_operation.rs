use super::*;
use chrono::Utc;

use crate::{
    AirportId, JobId, JobLeg, JobSnapshot, OperationalProvenance, ProvenanceKind, SnapshotFreshness,
};

fn airport(icao: &str) -> AirportSummary {
    AirportSummary {
        id: AirportId(Uuid::new_v4()),
        icao: Some(icao.into()),
        name: None,
        location: None,
    }
}

fn job() -> JobSummary {
    JobSummary {
        id: JobId(Uuid::new_v4()),
        mission_type: Some("Mixed transport".into()),
        description: None,
        reported_pay: None,
        created_at: None,
        taken_at: None,
        expires_at: None,
        legs: vec![
            JobLeg {
                id: JobLegId(Uuid::new_v4()),
                sequence: 0,
                kind: JobLegKind::Passengers,
                departure: Some(airport("YSSY")),
                destination: Some(airport("YMML")),
                current_airport: None,
                cargo_weight_lb: None,
                passengers: Some(12),
                distance_nm: None,
                description: None,
            },
            JobLeg {
                id: JobLegId(Uuid::new_v4()),
                sequence: 1,
                kind: JobLegKind::Cargo,
                departure: Some(airport("YMML")),
                destination: Some(airport("YPAD")),
                current_airport: None,
                cargo_weight_lb: None,
                passengers: None,
                distance_nm: None,
                description: None,
            },
        ],
    }
}

#[test]
fn manifest_preserves_received_loads_and_marks_missing_facts() {
    let manifest = FlightManifest::from_job(&job());

    assert_eq!(manifest.legs.len(), 2);
    assert_eq!(manifest.legs[0].passengers.as_ref().unwrap().count, 12);
    assert!(manifest.legs[0].freight.is_none());
    assert!(manifest.legs[0].unavailable_fields.is_empty());
    assert_eq!(
        manifest.legs[1].unavailable_fields,
        vec![ManifestUnavailableField::FreightWeight]
    );
    assert!(manifest.needs_attention());
    manifest.validate().unwrap();
}

#[test]
fn manifest_rejects_duplicate_source_legs() {
    let mut manifest = FlightManifest::from_job(&job());
    manifest.legs[1].source_job_leg_id = manifest.legs[0].source_job_leg_id.clone();

    assert_eq!(
        manifest.validate(),
        Err(FlightOperationValidationError::InvalidManifest)
    );
}

#[test]
fn operation_rejects_a_manifest_that_does_not_match_its_job_evidence() {
    let selected_job = job();
    let mut manifest = FlightManifest::from_job(&selected_job);
    manifest.legs[0].passengers.as_mut().unwrap().count += 1;
    let now = Utc::now();
    let plan: FlightPlanSnapshot = serde_json::from_str(include_str!(
        "../../../../schemas/fixtures/flight-plan-snapshot-v1.json"
    ))
    .unwrap();
    let revision = FlightOperationRevision {
        schema_version: FLIGHT_OPERATION_SCHEMA_VERSION,
        operation_id: FlightOperationId(Uuid::new_v4()),
        revision: 1,
        reason: FlightOperationRevisionReason::Initial,
        operation_created_at: now,
        revised_at: now,
        plan,
        selected_job_company_id: Some(CompanyId(Uuid::new_v4())),
        selected_job: Some(OperationalObservation {
            value: selected_job,
            provenance: OperationalProvenance {
                kind: ProvenanceKind::OnAirFact,
                provider: "OnAir".into(),
                provider_revision: None,
                generated_at: None,
                retrieved_at: now,
                transformation_version: 1,
                freshness: SnapshotFreshness::Current,
            },
        }),
        manifest,
    };

    assert_eq!(
        revision.validate(),
        Err(FlightOperationValidationError::InvalidManifest)
    );
}

#[test]
fn operation_rejects_invalid_revision_identity_reason_and_provenance() {
    let selected_job = job();
    let now = Utc::now();
    let plan: FlightPlanSnapshot = serde_json::from_str(include_str!(
        "../../../../schemas/fixtures/flight-plan-snapshot-v1.json"
    ))
    .unwrap();
    let mut revision = FlightOperationRevision {
        schema_version: FLIGHT_OPERATION_SCHEMA_VERSION,
        operation_id: FlightOperationId(Uuid::nil()),
        revision: 1,
        reason: FlightOperationRevisionReason::JobChanged,
        operation_created_at: now,
        revised_at: now,
        plan,
        selected_job_company_id: Some(CompanyId(Uuid::new_v4())),
        selected_job: Some(OperationalObservation {
            value: selected_job.clone(),
            provenance: OperationalProvenance {
                kind: ProvenanceKind::Calculated,
                provider: "wyrmgrid".into(),
                provider_revision: None,
                generated_at: Some(now),
                retrieved_at: now,
                transformation_version: 1,
                freshness: SnapshotFreshness::Current,
            },
        }),
        manifest: FlightManifest::from_job(&selected_job),
    };

    assert_eq!(
        revision.validate(),
        Err(FlightOperationValidationError::InvalidRevision)
    );

    revision.operation_id = FlightOperationId(Uuid::new_v4());
    revision.reason = FlightOperationRevisionReason::Initial;
    assert_eq!(
        revision.validate(),
        Err(FlightOperationValidationError::InvalidJob)
    );

    revision.selected_job.as_mut().unwrap().provenance.kind = ProvenanceKind::OnAirFact;
    revision.selected_job.as_mut().unwrap().provenance.provider = "OnAir".into();
    revision.revision = 2;
    assert_eq!(
        revision.validate(),
        Err(FlightOperationValidationError::InvalidRevision)
    );

    revision.revision = 1;
    revision.selected_job_company_id = None;
    assert_eq!(
        revision.validate(),
        Err(FlightOperationValidationError::InvalidJob)
    );
}

#[test]
fn job_snapshot_fixture_remains_valid() {
    let snapshot = JobSnapshot {
        schema_version: crate::JOB_SNAPSHOT_SCHEMA_VERSION,
        jobs: vec![job()],
    };
    snapshot.validate().unwrap();

    let observation = OperationalObservation {
        value: snapshot.jobs[0].clone(),
        provenance: OperationalProvenance {
            kind: ProvenanceKind::OnAirFact,
            provider: "OnAir".into(),
            provider_revision: None,
            generated_at: None,
            retrieved_at: Utc::now(),
            transformation_version: 1,
            freshness: SnapshotFreshness::Current,
        },
    };
    assert!(observation.provenance.is_valid());
}
