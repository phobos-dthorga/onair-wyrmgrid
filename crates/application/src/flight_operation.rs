use serde::Serialize;

pub const FLIGHT_OPERATION_JOURNEY_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FlightOperationStage {
    Plan,
    Weather,
    Jobs,
    Manifest,
    Fleet,
    Staff,
    Review,
    Atlas,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FlightOperationStageState {
    NotStarted,
    Available,
    Ready,
    NeedsAttention,
    Stale,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct FlightOperationStageView {
    pub stage: FlightOperationStage,
    pub state: FlightOperationStageState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FlightOperationJourneyView {
    pub schema_version: u32,
    pub stages: Vec<FlightOperationStageView>,
}

pub(crate) struct InitialJourneyEvidence {
    pub plan_provider_available: bool,
    pub plan_available: bool,
    pub weather_provider_available: bool,
    pub weather_available: bool,
    pub weather_stale: bool,
    pub atlas_available: bool,
}

pub(crate) fn build_initial_flight_operation_journey(
    evidence: InitialJourneyEvidence,
) -> FlightOperationJourneyView {
    let plan = if evidence.plan_available {
        FlightOperationStageState::Ready
    } else if evidence.plan_provider_available {
        FlightOperationStageState::Available
    } else {
        FlightOperationStageState::Unavailable
    };
    let weather = if !evidence.plan_available || !evidence.weather_provider_available {
        FlightOperationStageState::Unavailable
    } else if evidence.weather_available && evidence.weather_stale {
        FlightOperationStageState::Stale
    } else if evidence.weather_available {
        FlightOperationStageState::Ready
    } else {
        FlightOperationStageState::Available
    };
    let atlas = if evidence.atlas_available {
        FlightOperationStageState::Ready
    } else {
        FlightOperationStageState::Unavailable
    };

    FlightOperationJourneyView {
        schema_version: FLIGHT_OPERATION_JOURNEY_SCHEMA_VERSION,
        stages: vec![
            stage(FlightOperationStage::Plan, plan),
            stage(FlightOperationStage::Weather, weather),
            stage(
                FlightOperationStage::Jobs,
                FlightOperationStageState::NotStarted,
            ),
            stage(
                FlightOperationStage::Manifest,
                FlightOperationStageState::NotStarted,
            ),
            stage(
                FlightOperationStage::Fleet,
                FlightOperationStageState::NotStarted,
            ),
            stage(
                FlightOperationStage::Staff,
                FlightOperationStageState::NotStarted,
            ),
            stage(
                FlightOperationStage::Review,
                FlightOperationStageState::NotStarted,
            ),
            stage(FlightOperationStage::Atlas, atlas),
        ],
    }
}

fn stage(
    stage: FlightOperationStage,
    state: FlightOperationStageState,
) -> FlightOperationStageView {
    FlightOperationStageView { stage, state }
}

#[cfg(test)]
#[path = "tests/flight_operation.rs"]
mod tests;
