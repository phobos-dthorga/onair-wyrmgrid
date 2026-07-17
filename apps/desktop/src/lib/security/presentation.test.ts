import { describe, expect, it } from "vitest";
import type { SecurityDecisionView, SecurityGrantView } from "./types";
import {
  activeSecurityFilterCount,
  defaultSecurityFilters,
  filterSecurityDecisions,
  filterSecurityGrants,
  securityFilterOptions,
} from "./presentation";

const grant: SecurityGrantView = {
  subject_kind: "plugin",
  subject_id: "org.wyrmgrid.example",
  scope_revision: "v1",
  capabilities: ["map_layers_publish"],
  granted_at: "2026-07-17T00:00:00Z",
  lifetime: "standing",
};

const decision: SecurityDecisionView = {
  id: 1,
  subject_kind: "plugin",
  subject_id: grant.subject_id,
  scope_revision: grant.scope_revision,
  decision: "grant",
  capability_count: 1,
  decided_at: grant.granted_at,
  lifetime: grant.lifetime,
};

describe("security exploration", () => {
  it("derives lifetime and capability options from current grants", () => {
    expect(securityFilterOptions([grant])).toEqual({
      lifetimes: ["standing"],
      capabilities: ["map_layers_publish"],
    });
  });

  it("filters grants without changing authorization state", () => {
    const filters = {
      ...defaultSecurityFilters,
      query: "example",
      capability: "map_layers_publish",
    };
    expect(filterSecurityGrants([grant], filters)).toEqual([grant]);
    expect(activeSecurityFilterCount(filters, "access")).toBe(2);
  });

  it("filters symbolic decisions separately from active grants", () => {
    const filters = { ...defaultSecurityFilters, decision: "grant" as const };
    expect(filterSecurityDecisions([decision], filters)).toEqual([decision]);
    expect(activeSecurityFilterCount(filters, "history")).toBe(1);
  });

  it("keeps filter clearing available for a sort-only change", () => {
    const filters = { ...defaultSecurityFilters, sort: "subject" as const };
    expect(activeSecurityFilterCount(filters, "access")).toBe(1);
    expect(activeSecurityFilterCount(filters, "history")).toBe(1);
  });
});
