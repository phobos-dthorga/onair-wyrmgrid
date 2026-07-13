import type { FleetSnapshotView } from "./types";

export const atlasPreviewFleet: FleetSnapshotView = {
  company: {
    name: "Synthetic Preview Company",
    airline_code: "WYR",
  },
  availability: "preview",
  storage: "preview",
  snapshot: {
    value: [
      {
        id: "11111111-1111-4111-8111-111111111111",
        registration: "WYR-101",
        model: "Example Turboprop",
        location: { latitude: -37.8136, longitude: 144.9631 },
        current_airport: {
          id: "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
          icao: "YTEST",
          name: "Synthetic Regional Airport",
          location: { latitude: -37.8136, longitude: 144.9631 },
        },
      },
      {
        id: "22222222-2222-4222-8222-222222222222",
        registration: "WYR-202",
        model: "Example Jet",
        location: { latitude: 51.5072, longitude: -0.1276 },
        current_airport: null,
      },
      {
        id: "33333333-3333-4333-8333-333333333333",
        registration: "WYR-303",
        model: "Example Bush Aircraft",
        location: { latitude: 61.2181, longitude: -149.9003 },
        current_airport: null,
      },
    ],
    provenance: {
      kind: "calculated",
      source: "wyrmgrid:browser-preview",
      observed_at: "2026-07-14T00:00:00Z",
    },
  },
};
