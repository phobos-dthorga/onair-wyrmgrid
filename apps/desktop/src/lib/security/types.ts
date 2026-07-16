import type { LegalStatus } from "$lib/legal/client";

export type SecuritySubjectKind = "plugin";
export type SecurityDecision = "grant" | "revoke";
export type AuthorizationGrantLifetime = "once" | "session" | "standing";

export type SecurityGrantView = {
  subject_kind: SecuritySubjectKind;
  subject_id: string;
  scope_revision: string;
  capabilities: string[];
  granted_at: string;
  lifetime: AuthorizationGrantLifetime;
};

export type SecurityDecisionView = {
  id: number;
  subject_kind: SecuritySubjectKind;
  subject_id: string;
  scope_revision: string;
  decision: SecurityDecision;
  capability_count: number;
  decided_at: string;
  lifetime?: AuthorizationGrantLifetime;
};

export type SecurityCentreStatus = {
  legal: LegalStatus;
  active_grants: SecurityGrantView[];
  recent_decisions: SecurityDecisionView[];
  decision_retention_limit: number;
};
