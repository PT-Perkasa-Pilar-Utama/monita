// Response shapes of /api/ui. Each mirrors a `Serialize` struct in src/<module>/api.rs.

export type Project = {
  name: string;
  slug: string;
  owner_email: string;
  description: string | null;
  icon_url: string | null;
  ip_allowlist: string | null;
  created_at: number;
};

export type Owner = { email: string; role: string };

export type ProjectList = { projects: Project[]; owners: Owner[] };

export type TabCounts = { monitors: number; heartbeats: number; incidents: number; tickets: number };

export type ProjectHead = { project: Project; counts: TabCounts };

/** The worst state among a project's monitors or heartbeats, and a sentence. */
export type Summary = { total: number; kind: StateKind; text: string };

export type StateKind = "up" | "down" | "issue" | "unknown";

export type Severity = "critical" | "warning";

export type UpdateLabel = "investigating" | "identified" | "monitoring" | "resolved";

export type IncidentRow = {
  id: number;
  origin: "auto" | "manual";
  title: string;
  severity: Severity;
  cause: string | null;
  started_at: number;
  resolved_at: number | null;
  last_label: UpdateLabel | null;
};

export type Overview = {
  monitors: Summary;
  heartbeats: Summary;
  triage: number;
  incidents: IncidentRow[];
  active_keys: number;
};

export type ApiKey = {
  id: number;
  name: string;
  prefix: string;
  last_used_at: number | null;
  revoked_at: number | null;
  created_at: number;
};

export type AlertRoute = { severity: Severity; emails: string[]; webhook_url: string | null; use_webhook: boolean };
