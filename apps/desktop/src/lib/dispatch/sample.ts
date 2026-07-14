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
  weather: {
    provider_available: true,
    availability: "not_requested",
    refreshing: false,
    cache: "none",
  },
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
        registration: "WYR-101",
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
  comparison: {
    fleet_available: true,
    fleet_observed_at: "2026-07-14T00:00:00Z",
    matched_aircraft: {
      basis: "registration",
      registration: "WYR-101",
      model: "Example Turboprop",
      current_airport_icao: "YTEST",
    },
    findings: [
      {
        category: "aircraft_identity",
        status: "match",
        message_key: "dispatch-finding-registration-match",
        title: "Registration matched",
        explanation:
          "The SimBrief registration exactly matches one aircraft in the observed OnAir fleet.",
        plan_value: "WYR-101",
        onair_value: "WYR-101",
      },
      {
        category: "aircraft_model",
        status: "difference",
        message_key: "dispatch-finding-model-difference",
        title: "Model labels differ",
        explanation:
          "The source labels differ. No unverified aircraft-type crosswalk was applied.",
        plan_value: "Boeing 737-800",
        onair_value: "Example Turboprop",
      },
      {
        category: "aircraft_position",
        status: "difference",
        message_key: "dispatch-finding-position-difference",
        title: "Aircraft is away from origin",
        explanation:
          "The matched OnAir aircraft is observed at another airport; positioning may be required.",
        plan_value: "YSSY",
        onair_value: "YTEST",
      },
      {
        category: "payload",
        status: "unavailable",
        message_key: "dispatch-finding-payload-unavailable",
        title: "Payload limits not observed",
        explanation:
          "The current OnAir fleet contract does not include weight limits, so no compatibility is inferred.",
        plan_value: "14820 kg",
      },
      {
        category: "schedule",
        status: "unavailable",
        message_key: "dispatch-finding-deadline-unavailable",
        title: "Deadlines not observed",
        explanation:
          "The current OnAir slice does not include job schedules or deadlines.",
        plan_value: "2026-07-05T02:00:00Z",
      },
    ],
    provenance: {
      kind: "calculated",
      provider: "wyrmgrid",
      generated_at: "2026-07-14T00:00:00Z",
      retrieved_at: "2026-07-14T00:00:00Z",
      transformation_version: 1,
      freshness: "current",
    },
  },
  weather: {
    provider_available: true,
    availability: "ready",
    refreshing: false,
    cache: "fresh",
    snapshot: {
      schema_version: 1,
      id: "f5501d52-b462-4f96-86b0-57e283d19de7",
      airports: [
        {
          station_icao: "YSSY",
          metar: {
            value: {
              observed_at: "2026-07-14T01:00:00Z",
              raw_text: "METAR YSSY 140100Z AUTO 31013KT 9999 NCD 18/04 Q1021",
              report_type: "METAR",
              flight_category: "vfr",
              wind_direction: { kind: "degrees", value: 310 },
              wind_speed_kt: 13,
              visibility_sm: "6+",
              temperature_c: 18,
              dewpoint_c: 4,
              altimeter_hpa: 1021,
            },
            provenance: {
              ...provenance,
              kind: "external_fact",
              provider: "aviationweather.gov",
              provider_revision: "data-api-v4",
              generated_at: "2026-07-14T01:00:00Z",
            },
          },
          taf: {
            value: {
              issued_at: "2026-07-13T23:24:00Z",
              valid_from: "2026-07-14T00:00:00Z",
              valid_to: "2026-07-15T06:00:00Z",
              raw_text:
                "TAF YSSY 132324Z 1400/1506 32014KT CAVOK FM140200 26018KT CAVOK",
            },
            provenance: {
              ...provenance,
              kind: "external_fact",
              provider: "aviationweather.gov",
              provider_revision: "data-api-v4",
              generated_at: "2026-07-13T23:24:00Z",
            },
          },
        },
        {
          station_icao: "NZAA",
          metar: {
            value: {
              observed_at: "2026-07-14T01:30:00Z",
              raw_text: "METAR NZAA 140130Z AUTO 32007KT 9999 NCD 18/12 Q1024",
              report_type: "METAR",
              flight_category: "vfr",
              wind_direction: { kind: "degrees", value: 320 },
              wind_speed_kt: 7,
              visibility_sm: "6+",
              temperature_c: 18,
              dewpoint_c: 12,
              altimeter_hpa: 1024,
            },
            provenance: {
              ...provenance,
              kind: "external_fact",
              provider: "aviationweather.gov",
              provider_revision: "data-api-v4",
              generated_at: "2026-07-14T01:30:00Z",
            },
          },
          taf: {
            value: {
              issued_at: "2026-07-13T23:08:00Z",
              valid_from: "2026-07-14T00:00:00Z",
              valid_to: "2026-07-15T06:00:00Z",
              raw_text:
                "TAF NZAA 132308Z 1400/1506 03010KT 9999 -SHRA SCT020 BKN030",
            },
            provenance: {
              ...provenance,
              kind: "external_fact",
              provider: "aviationweather.gov",
              provider_revision: "data-api-v4",
              generated_at: "2026-07-13T23:08:00Z",
            },
          },
        },
        {
          station_icao: "NZWN",
          metar: {
            value: {
              observed_at: "2026-07-14T01:30:00Z",
              raw_text:
                "METAR NZWN 140130Z AUTO 35018G28KT 9999 BKN018 15/11 Q1019",
              report_type: "METAR",
              flight_category: "mvfr",
              wind_direction: { kind: "degrees", value: 350 },
              wind_speed_kt: 18,
              wind_gust_kt: 28,
              visibility_sm: "6+",
              temperature_c: 15,
              dewpoint_c: 11,
              altimeter_hpa: 1019,
            },
            provenance: {
              ...provenance,
              kind: "external_fact",
              provider: "aviationweather.gov",
              provider_revision: "data-api-v4",
              generated_at: "2026-07-14T01:30:00Z",
            },
          },
          taf: {
            value: {
              issued_at: "2026-07-13T23:08:00Z",
              valid_from: "2026-07-14T00:00:00Z",
              valid_to: "2026-07-15T06:00:00Z",
              raw_text: "TAF NZWN 132308Z 1400/1506 01020G30KT 9999 BKN020",
            },
            provenance: {
              ...provenance,
              kind: "external_fact",
              provider: "aviationweather.gov",
              provider_revision: "data-api-v4",
              generated_at: "2026-07-13T23:08:00Z",
            },
          },
        },
      ],
    },
  },
};
