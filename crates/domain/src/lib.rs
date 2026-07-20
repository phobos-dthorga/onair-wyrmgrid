//! Stable domain types shared by WyrmGrid services and public protocol adapters.

mod audio;
mod flight_operation;
mod flight_plan;
mod job;
mod operational;
mod simulator;
mod staff;
mod weather;

pub use audio::*;
pub use flight_operation::*;
pub use flight_plan::*;
pub use job::*;
pub use operational::*;
pub use simulator::*;
pub use staff::*;
pub use weather::*;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProvenanceKind {
    OnAirFact,
    ExternalFact,
    ExternalCalculation,
    Calculated,
    Recommendation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provenance {
    pub kind: ProvenanceKind,
    pub source: String,
    pub observed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Observed<T> {
    pub value: T,
    pub provenance: Provenance,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Coordinates {
    pub latitude: f64,
    pub longitude: f64,
}

impl Coordinates {
    pub fn is_valid(self) -> bool {
        (-90.0..=90.0).contains(&self.latitude) && (-180.0..=180.0).contains(&self.longitude)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AircraftId(pub Uuid);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AirportId(pub Uuid);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CompanyId(pub Uuid);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FboId(pub Uuid);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompanySummary {
    pub id: CompanyId,
    pub name: String,
    pub airline_code: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AirportSummary {
    pub id: AirportId,
    pub icao: Option<String>,
    pub name: Option<String>,
    pub location: Option<Coordinates>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AircraftSummary {
    pub id: AircraftId,
    pub registration: Option<String>,
    pub model: Option<String>,
    pub location: Option<Coordinates>,
    pub current_airport: Option<AirportSummary>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FboSummary {
    pub id: FboId,
    pub name: Option<String>,
    pub airport: Option<AirportSummary>,
}

#[cfg(test)]
#[path = "tests/domain.rs"]
mod tests;
