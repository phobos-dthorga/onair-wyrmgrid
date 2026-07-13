//! Stable domain types shared by WyrmGrid services and public protocol adapters.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProvenanceKind {
    OnAirFact,
    ExternalFact,
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
pub struct CompanyId(pub Uuid);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompanySummary {
    pub id: CompanyId,
    pub name: String,
    pub airline_code: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AircraftSummary {
    pub id: AircraftId,
    pub registration: String,
    pub model: String,
    pub location: Option<Coordinates>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_coordinates_outside_wgs84_bounds() {
        assert!(
            Coordinates {
                latitude: -33.8688,
                longitude: 151.2093
            }
            .is_valid()
        );
        assert!(
            !Coordinates {
                latitude: 91.0,
                longitude: 0.0
            }
            .is_valid()
        );
    }
}
