import type {
  DispatchStatus,
  OperationalProvenance,
} from "$lib/dispatch/types";
import type { AtlasPlannedRoute } from "$lib/atlas/types";

const provenance: OperationalProvenance = {
  kind: "external_calculation",
  provider: "simbrief",
  provider_revision: "2607",
  generated_at: "2026-07-05T00:00:00Z",
  retrieved_at: "2026-07-05T01:00:00Z",
  transformation_version: 1,
  freshness: "current",
};

const previewAtlasPlan: AtlasPlannedRoute = {
  schema_version: 1,
  plan_id: "129cc5ca-94de-46db-a10e-502bd39c7e98",
  origin_icao: "YSSY",
  destination_icao: "NZAA",
  airac: "2607",
  source_text: "TESAT Q29 LIZZI DCT MARLN A579 LUNBI",
  provenance,
  points: [
    {
      id: "origin:yssy",
      kind: "origin",
      label: "YSSY",
      location: { latitude: -33.9461, longitude: 151.1772 },
      on_route: true,
      gap_before: false,
    },
    {
      id: "route:0000:tesat",
      kind: "route_leg",
      label: "TESAT",
      sequence: 0,
      airway: "DCT",
      location: { latitude: -34.045, longitude: 151.03 },
      on_route: true,
      gap_before: false,
    },
    {
      id: "route:0001:lizzi",
      kind: "route_leg",
      label: "LIZZI",
      sequence: 1,
      airway: "Q29",
      location: { latitude: -35.661, longitude: 154.5 },
      on_route: true,
      gap_before: false,
    },
    {
      id: "route:0002:lunbi",
      kind: "route_leg",
      label: "LUNBI",
      sequence: 2,
      airway: "A579",
      location: { latitude: -36.52, longitude: 170.9 },
      on_route: true,
      gap_before: false,
    },
    {
      id: "destination:nzaa",
      kind: "destination",
      label: "NZAA",
      location: { latitude: -37.0081, longitude: 174.7917 },
      on_route: true,
      gap_before: false,
    },
    {
      id: "alternate:0000:nzwn",
      kind: "alternate",
      label: "NZWN",
      sequence: 0,
      location: { latitude: -41.3272, longitude: 174.8053 },
      on_route: false,
      gap_before: false,
    },
  ],
};

export const dispatchPreviewEmpty: DispatchStatus = {
  provider_available: true,
  availability: "empty",
  persistence: "session_only",
  importing: false,
  operation_change: "none",
  journey: {
    schema_version: 1,
    stages: [
      { stage: "plan", state: "available" },
      { stage: "weather", state: "unavailable" },
      { stage: "jobs", state: "not_started" },
      { stage: "manifest", state: "not_started" },
      { stage: "fleet", state: "not_started" },
      { stage: "staff", state: "not_started" },
      { stage: "review", state: "not_started" },
      { stage: "atlas", state: "unavailable" },
    ],
  },
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
  operation_change: "none",
  atlas_plan: previewAtlasPlan,
  journey: {
    schema_version: 1,
    stages: [
      { stage: "plan", state: "ready" },
      { stage: "weather", state: "ready" },
      { stage: "jobs", state: "not_started" },
      { stage: "manifest", state: "not_started" },
      { stage: "fleet", state: "not_started" },
      { stage: "staff", state: "not_started" },
      { stage: "review", state: "not_started" },
      { stage: "atlas", state: "ready" },
    ],
  },
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
          location: { latitude: -33.9461, longitude: 151.1772 },
          planned_runway: "34L",
        },
        destination: {
          icao: "NZAA",
          name: "Auckland",
          location: { latitude: -37.0082, longitude: 174.785 },
          planned_runway: "23L",
        },
        alternates: [
          {
            icao: "NZWN",
            name: "Wellington",
            location: { latitude: -41.3272, longitude: 174.805 },
          },
        ],
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
          {
            sequence: 0,
            ident: "TESAT",
            airway: "DCT",
            location: { latitude: -34.204, longitude: 151.831 },
          },
          {
            sequence: 1,
            ident: "LIZZI",
            airway: "Q29",
            location: { latitude: -35.46, longitude: 158.8 },
          },
          { sequence: 2, ident: "LUNBI", airway: "A579" },
        ],
      },
      provenance,
    },
  },
  atlas_route: {
    projection_version: 1,
    plan_id: "129cc5ca-94de-46db-a10e-502bd39c7e98",
    airac: "2607",
    source_text: "TESAT Q29 LIZZI DCT MARLN A579 LUNBI",
    route_feature_ids: [
      "129cc5ca-94de-46db-a10e-502bd39c7e98:origin",
      "129cc5ca-94de-46db-a10e-502bd39c7e98:route-fix:0",
      "129cc5ca-94de-46db-a10e-502bd39c7e98:route-fix:1",
      "129cc5ca-94de-46db-a10e-502bd39c7e98:route-fix:2",
      "129cc5ca-94de-46db-a10e-502bd39c7e98:destination",
    ],
    mapped_route_feature_count: 4,
    unresolved_route_feature_count: 1,
    provenance,
    features: [
      {
        id: "129cc5ca-94de-46db-a10e-502bd39c7e98:origin",
        kind: "origin",
        ident: "YSSY",
        name: "Sydney Kingsford Smith",
        availability: "resolved",
        location: { latitude: -33.9461, longitude: 151.1772 },
      },
      {
        id: "129cc5ca-94de-46db-a10e-502bd39c7e98:route-fix:0",
        kind: "route_fix",
        ident: "TESAT",
        sequence: 0,
        airway: "DCT",
        availability: "resolved",
        location: { latitude: -34.204, longitude: 151.831 },
      },
      {
        id: "129cc5ca-94de-46db-a10e-502bd39c7e98:route-fix:1",
        kind: "route_fix",
        ident: "LIZZI",
        sequence: 1,
        airway: "Q29",
        availability: "resolved",
        location: { latitude: -35.46, longitude: 158.8 },
      },
      {
        id: "129cc5ca-94de-46db-a10e-502bd39c7e98:route-fix:2",
        kind: "route_fix",
        ident: "LUNBI",
        sequence: 2,
        airway: "A579",
        availability: "location_unavailable",
      },
      {
        id: "129cc5ca-94de-46db-a10e-502bd39c7e98:destination",
        kind: "destination",
        ident: "NZAA",
        name: "Auckland",
        availability: "resolved",
        location: { latitude: -37.0082, longitude: 174.785 },
      },
      {
        id: "129cc5ca-94de-46db-a10e-502bd39c7e98:alternate:0",
        kind: "alternate",
        ident: "NZWN",
        name: "Wellington",
        availability: "resolved",
        location: { latitude: -41.3272, longitude: 174.805 },
      },
    ],
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
              retrieved_at: "2026-07-14T01:03:00Z",
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
              retrieved_at: "2026-07-14T01:03:00Z",
              generated_at: "2026-07-13T23:24:00Z",
            },
          },
        },
        {
          station_icao: "NZAA",
          metar: {
            value: {
              observed_at: "2026-07-14T01:30:00Z",
              raw_text:
                "METAR NZAA 140130Z AUTO 32007KT 8000 -SHRA SCT020 BKN030 18/12 Q1024",
              report_type: "METAR",
              flight_category: "mvfr",
              wind_direction: { kind: "degrees", value: 320 },
              wind_speed_kt: 7,
              visibility_sm: "5",
              temperature_c: 18,
              dewpoint_c: 12,
              altimeter_hpa: 1024,
              present_weather: "-SHRA",
            },
            provenance: {
              ...provenance,
              kind: "external_fact",
              provider: "aviationweather.gov",
              provider_revision: "data-api-v4",
              retrieved_at: "2026-07-14T01:33:00Z",
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
              retrieved_at: "2026-07-14T01:33:00Z",
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
                "METAR NZWN 140130Z AUTO 35018G28KT 4000 BR BKN008 15/11 Q1019",
              report_type: "METAR",
              flight_category: "ifr",
              wind_direction: { kind: "degrees", value: 350 },
              wind_speed_kt: 18,
              wind_gust_kt: 28,
              visibility_sm: "2.5",
              temperature_c: 15,
              dewpoint_c: 11,
              altimeter_hpa: 1019,
              present_weather: "BR",
            },
            provenance: {
              ...provenance,
              kind: "external_fact",
              provider: "aviationweather.gov",
              provider_revision: "data-api-v4",
              retrieved_at: "2026-07-14T01:33:00Z",
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
              retrieved_at: "2026-07-14T01:33:00Z",
              generated_at: "2026-07-13T23:08:00Z",
            },
          },
        },
      ],
    },
  },
};

const previewWeather = dispatchPreviewReady.weather.snapshot;
if (previewWeather) {
  dispatchPreviewReady.atlas_weather = {
    schema_version: 1,
    plan_id: dispatchPreviewReady.snapshot?.id ?? "preview-plan",
    weather_snapshot_id: previewWeather.id,
    stations: [
      {
        id: "weather:origin:yssy",
        role: "origin",
        location: { latitude: -33.9461, longitude: 151.1772 },
        ...previewWeather.airports[0],
      },
      {
        id: "weather:destination:nzaa",
        role: "destination",
        location: { latitude: -37.0081, longitude: 174.7917 },
        ...previewWeather.airports[1],
      },
      {
        id: "weather:alternate:0000:nzwn",
        role: "alternate",
        location: { latitude: -41.3272, longitude: 174.8053 },
        ...previewWeather.airports[2],
      },
    ],
  };
}
