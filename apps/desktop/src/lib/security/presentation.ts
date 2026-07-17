import {
  compareOptionalDate,
  compareOptionalText,
  countActiveFilters,
  matchesQuery,
  uniqueReportedValues,
} from "$lib/exploration/collection";
import type {
  AuthorizationGrantLifetime,
  SecurityDecision,
  SecurityDecisionView,
  SecurityGrantView,
} from "./types";

export type SecurityFilters = {
  query: string;
  lifetime: "all" | AuthorizationGrantLifetime;
  capability: string | null;
  decision: "all" | SecurityDecision;
  sort: "newest" | "subject";
};

export const defaultSecurityFilters: SecurityFilters = {
  query: "",
  lifetime: "all",
  capability: null,
  decision: "all",
  sort: "newest",
};

export function securityFilterOptions(grants: readonly SecurityGrantView[]): {
  lifetimes: AuthorizationGrantLifetime[];
  capabilities: string[];
} {
  return {
    lifetimes: uniqueReportedValues(grants.map((grant) => grant.lifetime)),
    capabilities: uniqueReportedValues(
      grants.flatMap((grant) => grant.capabilities),
    ).sort(compareOptionalText),
  };
}

export function filterSecurityGrants(
  grants: readonly SecurityGrantView[],
  filters: SecurityFilters,
): SecurityGrantView[] {
  return grants
    .filter(
      (grant) =>
        matchesQuery(filters.query, [
          grant.subject_id,
          grant.subject_kind,
          grant.scope_revision,
          ...grant.capabilities,
        ]) &&
        (filters.lifetime === "all" || grant.lifetime === filters.lifetime) &&
        (filters.capability === null ||
          grant.capabilities.includes(filters.capability)),
    )
    .sort((left, right) =>
      filters.sort === "subject"
        ? compareOptionalText(left.subject_id, right.subject_id)
        : -compareOptionalDate(left.granted_at, right.granted_at),
    );
}

export function filterSecurityDecisions(
  decisions: readonly SecurityDecisionView[],
  filters: SecurityFilters,
): SecurityDecisionView[] {
  return decisions
    .filter(
      (decision) =>
        matchesQuery(filters.query, [
          decision.subject_id,
          decision.subject_kind,
          decision.scope_revision,
          decision.decision,
        ]) &&
        (filters.decision === "all" || decision.decision === filters.decision),
    )
    .sort((left, right) =>
      filters.sort === "subject"
        ? compareOptionalText(left.subject_id, right.subject_id)
        : -compareOptionalDate(left.decided_at, right.decided_at),
    );
}

export function activeSecurityFilterCount(
  filters: SecurityFilters,
  section: "access" | "history",
): number {
  return countActiveFilters([
    filters.query.trim().length > 0,
    section === "access" && filters.lifetime !== "all",
    section === "access" && filters.capability !== null,
    section === "history" && filters.decision !== "all",
    filters.sort !== "newest",
  ]);
}
