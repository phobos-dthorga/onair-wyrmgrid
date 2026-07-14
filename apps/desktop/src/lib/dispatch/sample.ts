import type {
  DispatchStatus,
  OperationalProvenance,
} from "$lib/dispatch/types";

const provenance: OperationalProvenance = {
  kind: "external_calculation",
  provider: "simbrief",
  provider_revision: "2607",
  generated_at: "2026-07-05T00:00:00Z",
  retrieved_at: "2026-07-05T01:00:00Z",
  transformation_version: 1,
  freshness: "current",
};

export const dispatchPreviewEmpty: DispatchStatus = {
  provider_available: true,
  availability: "empty",
  persistence: "session_only",
  importing: false,
};

export const dispatchPreviewReady: DispatchStatus = {
  provider_available: true,
  availability: "ready",
  persistence: "session_only",
  importing: false,
  snapshot: {
    schema_version: 1,
    id: "129cc5ca-94de-46db-a10e-502bd39c7e98",
    identity: {
      value: {
        airac: "2607",
        provider_plan_reference: "wyrmgrid-preview-001",
      },
      provenance,
    },
    airports: {
      value: {
        origin: {
          icao: "YSSY",
          name: "Sydney Kingsford Smith",
          planned_runway: "34L",
        },
        destination: {
          icao: "NZAA",
          name: "Auckland",
          planned_runway: "23L",
        },
        alternates: [{ icao: "NZWN", name: "Wellington" }],
      },
      provenance,
    },
    aircraft: {
      value: {
        icao_type: "B738",
        registration: "VH-WYR",
        model: "Boeing 737-800",
      },
      provenance,
    },
    schedule: {
      value: {
        scheduled_out: "2026-07-05T02:00:00Z",
        scheduled_off: "2026-07-05T02:15:00Z",
        scheduled_on: "2026-07-05T05:00:00Z",
        scheduled_in: "2026-07-05T05:10:00Z",
        estimated_enroute_seconds: 9900,
      },
      provenance,
    },
    weights: {
      value: {
        payload: { value: 14820, unit: "kilograms" },
        zero_fuel: { value: 62140, unit: "kilograms" },
        takeoff: { value: 70420, unit: "kilograms" },
        landing: { value: 64830, unit: "kilograms" },
      },
      provenance,
    },
    fuel: {
      value: {
        taxi: { value: 240, unit: "kilograms" },
        enroute: { value: 5590, unit: "kilograms" },
        reserve: { value: 1190, unit: "kilograms" },
        alternate: { value: 640, unit: "kilograms" },
        ramp: { value: 8520, unit: "kilograms" },
        takeoff: { value: 8280, unit: "kilograms" },
        landing: { value: 2690, unit: "kilograms" },
      },
      provenance,
    },
    route: {
      value: {
        source_text: "TESAT Q29 LIZZI DCT MARLN A579 LUNBI",
        initial_altitude_ft: 36000,
        distance_nm: 1184.6,
        legs: [
          { sequence: 0, ident: "TESAT", airway: "DCT" },
          { sequence: 1, ident: "LIZZI", airway: "Q29" },
          { sequence: 2, ident: "LUNBI", airway: "A579" },
        ],
      },
      provenance,
    },
  },
};
