export type DiagnosticEntry = {
  occurred_at: string;
  level: string;
  code: string;
  operation: string;
  message: string;
  plugin_id?: string;
};

export type DiagnosticLogView = {
  language: "English";
  storage: "local_file" | "memory_only";
  entries: DiagnosticEntry[];
};

export const emptyDiagnosticLog: DiagnosticLogView = {
  language: "English",
  storage: "memory_only",
  entries: [],
};
