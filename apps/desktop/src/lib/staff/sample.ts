import type { StaffSnapshotView } from "$lib/atlas/types";

export const staffPreview: StaffSnapshotView = {
  company: { name: "Synthetic Preview Company", airline_code: "WYR" },
  availability: "preview",
  storage: "preview",
  snapshot: {
    provenance: {
      kind: "calculated",
      source: "wyrmgrid:browser-preview",
      observed_at: "2026-07-17T00:00:00Z",
    },
    value: {
      schema_version: 1,
      staff: [
        {
          id: "11111111-1111-4111-8111-111111111111",
          display_name: "Example Aviator",
          avatar_reference: "synthetic-preview-avatar.png",
          category_code: 1,
          status_code: 2,
          current_airport: {
            id: "22222222-2222-4222-8222-222222222222",
            icao: "YTEST",
            name: "Synthetic Test Airport",
            location: null,
          },
          home_airport: {
            id: "33333333-3333-4333-8333-333333333333",
            icao: "YHOME",
            name: "Synthetic Home Airport",
            location: null,
          },
          is_online: true,
          class_qualifications: [
            {
              id: "44444444-4444-4444-8444-444444444444",
              aircraft_class_id: "55555555-5555-4555-8555-555555555555",
              short_name: "TP",
              name: "Synthetic Turboprop",
            },
          ],
        },
        {
          id: "66666666-6666-4666-8666-666666666666",
          display_name: "Example Crew Member",
          category_code: 3,
          status_code: 0,
          is_online: false,
          class_qualifications: [],
        },
      ],
    },
  },
};
