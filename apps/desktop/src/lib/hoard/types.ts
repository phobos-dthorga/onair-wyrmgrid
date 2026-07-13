import type { FboSnapshotView, FleetSnapshotView } from "$lib/atlas/types";

export type FleetHistoryPoint = {
  observed_at: string;
  aircraft_count: number;
};

export type FboHistoryPoint = {
  observed_at: string;
  fbo_count: number;
};

export type FleetCompositionPoint = {
  model: string;
  aircraft_count: number;
};

export type HoardTimelineIndex = {
  company: FleetSnapshotView["company"] | null;
  observation_times: string[];
  fleet_history: FleetHistoryPoint[];
  fbo_history: FboHistoryPoint[];
  current_fleet_composition: FleetCompositionPoint[];
};

export type HistoricalCompanyDataView = {
  selected_at: string;
  fleet: FleetSnapshotView | null;
  fbos: FboSnapshotView | null;
  fleet_composition: FleetCompositionPoint[];
};

export type TimelineMode = "live" | "historical";
