use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use std::sync::{
    Arc, RwLock,
    atomic::{AtomicBool, Ordering},
};
use thiserror::Error;
use wyrmgrid_domain::FlightPlanSnapshot;
use wyrmgrid_simbrief_api::{ClientError, SimBriefClient, UserReference, UserReferenceKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SimBriefReferenceKind {
    PilotId,
    Username,
}

impl From<SimBriefReferenceKind> for UserReferenceKind {
    fn from(value: SimBriefReferenceKind) -> Self {
        match value {
            SimBriefReferenceKind::PilotId => Self::PilotId,
            SimBriefReferenceKind::Username => Self::Username,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DispatchAvailability {
    Empty,
    Ready,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DispatchPersistence {
    SessionOnly,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DispatchStatus {
    pub provider_available: bool,
    pub availability: DispatchAvailability,
    pub persistence: DispatchPersistence,
    pub importing: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<FlightPlanSnapshot>,
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum DispatchError {
    #[error("The SimBrief importer is unavailable in this application session.")]
    ProviderUnavailable,
    #[error("A SimBrief import is already in progress.")]
    ImportInProgress,
    #[error("The local Dispatch state is unavailable.")]
    StateUnavailable,
    #[error(transparent)]
    Provider(#[from] ClientError),
}

type ProviderFuture<'a> =
    Pin<Box<dyn Future<Output = Result<FlightPlanSnapshot, ClientError>> + Send + 'a>>;

trait FlightPlanProvider: Send + Sync {
    fn fetch_latest<'a>(
        &'a self,
        kind: SimBriefReferenceKind,
        value: &'a str,
    ) -> ProviderFuture<'a>;
}

impl FlightPlanProvider for SimBriefClient {
    fn fetch_latest<'a>(
        &'a self,
        kind: SimBriefReferenceKind,
        value: &'a str,
    ) -> ProviderFuture<'a> {
        Box::pin(async move {
            let reference = UserReference::parse(kind.into(), value)?;
            SimBriefClient::fetch_latest(self, &reference).await
        })
    }
}

struct DispatchInner {
    provider: Option<Arc<dyn FlightPlanProvider>>,
    snapshot: RwLock<Option<FlightPlanSnapshot>>,
    importing: AtomicBool,
}

#[derive(Clone)]
pub struct DispatchSession {
    inner: Arc<DispatchInner>,
}

impl DispatchSession {
    pub fn with_default_provider() -> Self {
        let provider = SimBriefClient::new()
            .ok()
            .map(|provider| Arc::new(provider) as Arc<dyn FlightPlanProvider>);
        Self::new(provider)
    }

    fn new(provider: Option<Arc<dyn FlightPlanProvider>>) -> Self {
        Self {
            inner: Arc::new(DispatchInner {
                provider,
                snapshot: RwLock::new(None),
                importing: AtomicBool::new(false),
            }),
        }
    }

    pub fn status(&self) -> Result<DispatchStatus, DispatchError> {
        let snapshot = self
            .inner
            .snapshot
            .read()
            .map_err(|_| DispatchError::StateUnavailable)?
            .clone();
        Ok(DispatchStatus {
            provider_available: self.inner.provider.is_some(),
            availability: if snapshot.is_some() {
                DispatchAvailability::Ready
            } else {
                DispatchAvailability::Empty
            },
            persistence: DispatchPersistence::SessionOnly,
            importing: self.inner.importing.load(Ordering::Acquire),
            snapshot,
        })
    }

    pub async fn import_latest(
        &self,
        kind: SimBriefReferenceKind,
        value: &str,
    ) -> Result<DispatchStatus, DispatchError> {
        let provider = self
            .inner
            .provider
            .as_ref()
            .ok_or(DispatchError::ProviderUnavailable)?
            .clone();
        if self
            .inner
            .importing
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(DispatchError::ImportInProgress);
        }
        let _guard = ImportGuard(&self.inner.importing);
        let snapshot = provider.fetch_latest(kind, value).await?;
        snapshot
            .validate()
            .map_err(|_| DispatchError::Provider(ClientError::MalformedPlan))?;
        *self
            .inner
            .snapshot
            .write()
            .map_err(|_| DispatchError::StateUnavailable)? = Some(snapshot);
        self.status()
    }

    pub fn clear(&self) -> Result<DispatchStatus, DispatchError> {
        if self.inner.importing.load(Ordering::Acquire) {
            return Err(DispatchError::ImportInProgress);
        }
        *self
            .inner
            .snapshot
            .write()
            .map_err(|_| DispatchError::StateUnavailable)? = None;
        self.status()
    }
}

struct ImportGuard<'a>(&'a AtomicBool);

impl Drop for ImportGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::DateTime;
    use std::sync::Mutex;
    use uuid::Uuid;
    use wyrmgrid_domain::{
        FLIGHT_PLAN_SNAPSHOT_SCHEMA_VERSION, FlightPlanAirport, FlightPlanAirports,
        FlightPlanIdentity, FlightPlanSnapshotId, OperationalObservation, OperationalProvenance,
        ProvenanceKind, SnapshotFreshness,
    };

    struct FakeProvider {
        responses: Mutex<Vec<FlightPlanSnapshot>>,
    }

    impl FlightPlanProvider for FakeProvider {
        fn fetch_latest<'a>(
            &'a self,
            _kind: SimBriefReferenceKind,
            _value: &'a str,
        ) -> ProviderFuture<'a> {
            Box::pin(async move {
                self.responses
                    .lock()
                    .unwrap()
                    .pop()
                    .ok_or(ClientError::NoPlan)
            })
        }
    }

    fn snapshot(destination: &str) -> FlightPlanSnapshot {
        let retrieved_at = DateTime::from_timestamp(1_783_214_400, 0).unwrap();
        let provenance = OperationalProvenance {
            kind: ProvenanceKind::ExternalCalculation,
            provider: "simbrief".into(),
            provider_revision: Some("2607".into()),
            generated_at: Some(retrieved_at),
            retrieved_at,
            transformation_version: 1,
            freshness: SnapshotFreshness::Current,
        };
        FlightPlanSnapshot {
            schema_version: FLIGHT_PLAN_SNAPSHOT_SCHEMA_VERSION,
            id: FlightPlanSnapshotId(Uuid::new_v4()),
            identity: OperationalObservation {
                value: FlightPlanIdentity {
                    airac: Some("2607".into()),
                    provider_plan_reference: None,
                },
                provenance: provenance.clone(),
            },
            airports: OperationalObservation {
                value: FlightPlanAirports {
                    origin: FlightPlanAirport {
                        icao: "YSSY".into(),
                        name: None,
                        location: None,
                        planned_runway: None,
                    },
                    destination: FlightPlanAirport {
                        icao: destination.into(),
                        name: None,
                        location: None,
                        planned_runway: None,
                    },
                    alternates: Vec::new(),
                },
                provenance,
            },
            aircraft: None,
            schedule: None,
            weights: None,
            fuel: None,
            route: None,
        }
    }

    #[tokio::test]
    async fn replaces_the_session_snapshot_and_never_retains_the_user_reference() {
        let provider = FakeProvider {
            responses: Mutex::new(vec![snapshot("NZAA"), snapshot("YMML")]),
        };
        let session = DispatchSession::new(Some(Arc::new(provider)));

        let first = session
            .import_latest(SimBriefReferenceKind::Username, "private-user")
            .await
            .unwrap();
        assert_eq!(
            first.snapshot.unwrap().airports.value.destination.icao,
            "YMML"
        );
        let serialized = serde_json::to_string(&session.status().unwrap()).unwrap();
        assert!(!serialized.contains("private-user"));

        let second = session
            .import_latest(SimBriefReferenceKind::PilotId, "1234567")
            .await
            .unwrap();
        assert_eq!(
            second.snapshot.unwrap().airports.value.destination.icao,
            "NZAA"
        );
        assert_eq!(
            session.clear().unwrap().availability,
            DispatchAvailability::Empty
        );
    }
}
