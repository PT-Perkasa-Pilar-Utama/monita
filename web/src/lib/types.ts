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

export type LastCheck = { checked_at: number; ok: boolean; summary: string };

export type MonitorView = {
  id: number;
  name: string;
  url: string;
  interval_s: number;
  every: string;
  timeout_s: number;
  expected_status: string;
  keyword: string | null;
  skip_tls_verify: boolean;
  severity: Severity;
  is_public: boolean;
  created_at: number;
  next_check_at: number;
  state: StateKind;
  last: LastCheck | null;
};

/** One monitor's last 24 hours, drawn in a 160 x 32 view box. */
export type Day = { path: string; fails: number[]; uptime: string | null; mean_ms: number | null };

export type MonitorRow = MonitorView & { day: Day | null };

export type ListSummary = { kind: StateKind; text: string };

export type MonitorList = { monitors: MonitorRow[]; summary: ListSummary };

export type Stat = { label: string; value: string; sub: string; tone: "up" | "down" | "plain" };

export type Mark = { x: number; w: number; y: number; h: number; title: string };
export type Tick = { at: number; label: string };

/** Chart geometry the server lays out in a 960 x 240 view box. */
export type Chart = {
  line: string;
  area: string;
  dots: [number, number][];
  up: Mark[];
  down: Mark[];
  fails: Mark[];
  oks: Mark[];
  hovers: Mark[];
  y_ticks: Tick[];
  x_ticks: Tick[];
  empty: boolean;
};

export type Period = {
  label: string;
  availability: number | null;
  downtime_s: number;
  incidents: number;
  longest_s: number | null;
  average_s: number | null;
};

export type Range = "hour" | "day" | "week" | "month";

export type Insight = {
  title: string;
  range: Range;
  total: number;
  ok: number;
  mean_ms: number | null;
  chart: Chart;
  periods: Period[];
};

export type CheckRow = {
  id: number;
  checked_at: number;
  ok: boolean;
  status_code: number | null;
  latency_ms: number | null;
  error: string | null;
};

export type MonitorDetail = {
  monitor: MonitorView;
  stats: Stat[];
  insight: Insight;
  checks: CheckRow[];
  older: string | null;
};

export type Due = { used: number; late: boolean; text: string };

export type HeartbeatView = {
  id: number;
  name: string;
  period_s: number;
  grace_s: number;
  every: string;
  grace: string;
  severity: Severity;
  is_public: boolean;
  last_ping_at: number | null;
  deadline_at: number | null;
  created_at: number;
  state: StateKind;
  due: Due;
};

export type HeartbeatList = { heartbeats: HeartbeatView[]; summary: ListSummary };

export type PingRow = { id: number; received_at: number; kind: "ok" | "fail"; source_ip: string | null };

export type HeartbeatDetail = {
  heartbeat: HeartbeatView;
  ping_url: string;
  stats: Stat[];
  insight: Insight;
  pings: PingRow[];
  older: string | null;
};

export type Choice = { value: number; label: string };

export type CheckOptions = {
  intervals: Choice[];
  periods: Choice[];
  graces: Choice[];
  monitor: { interval_s: number; timeout_s: number; expected_status: string; severity: Severity };
  heartbeat: { period_s: number; grace_s: number; severity: Severity };
};
