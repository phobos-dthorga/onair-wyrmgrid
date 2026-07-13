import { atlasPreviewFbos, atlasPreviewFleet } from "$lib/atlas/sample";
import type { HistoricalCompanyDataView, HoardTimelineIndex } from "./types";

export const hoardPreviewTimeline: HoardTimelineIndex = {
  company: atlasPreviewFleet.company,
  observation_times: [
    "2026-07-10T06:00:00Z",
    "2026-07-11T06:00:00Z",
    "2026-07-12T06:00:00Z",
    "2026-07-13T06:00:00Z",
    "2026-07-14T00:00:00Z",
  ],
  fleet_history: [
    { observed_at: "2026-07-10T06:00:00Z", aircraft_count: 1 },
    { observed_at: "2026-07-11T06:00:00Z", aircraft_count: 1 },
    { observed_at: "2026-07-12T06:00:00Z", aircraft_count: 2 },
    { observed_at: "2026-07-13T06:00:00Z", aircraft_count: 2 },
    { observed_at: "2026-07-14T00:00:00Z", aircraft_count: 3 },
  ],
  current_fleet_composition: [
    { model: "Example Bush Aircraft", aircraft_count: 1 },
    { model: "Example Jet", aircraft_count: 1 },
    { model: "Example Turboprop", aircraft_count: 1 },
  ],
};

export function previewHistoricalCompanyData(
  selectedAt: string,
): HistoricalCompanyDataView {
  const selectedIndex = Math.max(
    0,
    hoardPreviewTimeline.observation_times.indexOf(selectedAt),
  );
  const fleetSize =
    hoardPreviewTimeline.fleet_history[selectedIndex]?.aircraft_count ?? 1;
  const aircraft = atlasPreviewFleet.snapshot.value.slice(0, fleetSize);
  const composition = aircraft.map((item) => ({
    model: item.model ?? "Unknown model",
    aircraft_count: 1,
  }));
  return {
    selected_at: selectedAt,
    fleet: {
      ...atlasPreviewFleet,
      snapshot: {
        ...atlasPreviewFleet.snapshot,
        value: aircraft,
        provenance: {
          ...atlasPreviewFleet.snapshot.provenance,
          observed_at: selectedAt,
        },
      },
    },
    fbos: {
      ...atlasPreviewFbos,
      snapshot: {
        ...atlasPreviewFbos.snapshot,
        value: selectedIndex >= 2 ? atlasPreviewFbos.snapshot.value : [],
        provenance: {
          ...atlasPreviewFbos.snapshot.provenance,
          observed_at: selectedAt,
        },
      },
    },
    fleet_composition: composition,
  };
}
