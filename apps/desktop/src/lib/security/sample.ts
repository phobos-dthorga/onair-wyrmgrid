import {
  CURRENT_PRIVACY_NOTICE_VERSION,
  CURRENT_TERMS_VERSION,
} from "$lib/legal/client";
import type { SecurityCentreStatus } from "./types";

const subjectId = "org.wyrmgrid.example.fleet-locations";
const scopeRevision = "plugin:0.1.0:map_layers_publish|on_air_fleet_read";

const base = {
  legal: {
    terms_version: CURRENT_TERMS_VERSION,
    privacy_notice_version: CURRENT_PRIVACY_NOTICE_VERSION,
    acknowledged: true,
    telemetry_enabled: false,
    acknowledged_at: "2026-07-15 09:00:00",
  },
  decision_retention_limit: 4096,
};

export const securityPreviewEmpty: SecurityCentreStatus = {
  ...base,
  active_grants: [],
  recent_decisions: [],
};

export const securityPreviewGranted: SecurityCentreStatus = {
  ...base,
  active_grants: [
    {
      subject_kind: "plugin",
      subject_id: subjectId,
      scope_revision: scopeRevision,
      capabilities: ["map_layers_publish", "on_air_fleet_read"],
      granted_at: "2026-07-15 09:12:00",
      lifetime: "standing",
    },
  ],
  recent_decisions: [
    {
      id: 1,
      subject_kind: "plugin",
      subject_id: subjectId,
      scope_revision: scopeRevision,
      decision: "grant",
      capability_count: 2,
      decided_at: "2026-07-15 09:12:00",
      lifetime: "standing",
    },
  ],
};

export const securityPreviewRevoked: SecurityCentreStatus = {
  ...base,
  active_grants: [],
  recent_decisions: [
    {
      id: 2,
      subject_kind: "plugin",
      subject_id: subjectId,
      scope_revision: scopeRevision,
      decision: "revoke",
      capability_count: 0,
      decided_at: "2026-07-15 09:20:00",
    },
    ...securityPreviewGranted.recent_decisions,
  ],
};
