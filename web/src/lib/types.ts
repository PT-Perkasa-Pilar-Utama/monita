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

/** One page of the project list; `total` counts every project that matches the search. */
export type ProjectList = {
  projects: Project[];
  total: number;
  page: number;
  page_size: number;
  owners: Owner[];
  /** Who owns a project, for a super admin's owner filter; empty for a tech lead. */
  owner_options: string[];
};

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

export type UpdateRow = { label: UpdateLabel; message: string; author_email: string | null; created_at: number };

export type IncidentDetail = { incident: IncidentRow; items: string[]; updates: UpdateRow[]; labels: UpdateLabel[] };

export type IncidentItem = { value: string; name: string };

export type Stage = "triage" | "backlog" | "in_progress" | "done";

export type CloseReason = "fixed" | "wont_fix" | "duplicate";

export type TicketRow = {
  number: number;
  title: string;
  public_title: string | null;
  description: string;
  page_url: string | null;
  stage: Stage;
  close_reason: CloseReason | null;
  created_at: number;
  updated_at: number;
  ref_revoked: boolean;
  // The team member who filed it; null for a reporter's ticket.
  filed_by: string | null;
};

export type StageCounts = Record<Stage, number>;

export type TicketBoard = { tickets: TicketRow[]; counts: StageCounts };

/** Who hears about new tickets: the owner's daily digest, and a webhook per ticket. */
export type TicketAlerts = { digest: boolean; webhook_url: string | null; use_webhook: boolean };

/** One more page of one column (`?stage=&page=`). */
export type ColumnPage<T> = { tickets: T[]; page: number; total: number; page_size: number };

/** SEC-14: what a visitor may see of a ticket. */
export type PublicTicket = { number: number; title: string; stage: Stage; created_at: number; updated_at: number };

/** The public board has no Triage column, not even a count (SEC-14). */
export type PublicBoard = { tickets: PublicTicket[]; counts: Omit<StageCounts, "triage"> };

export type MyTicket = { number: number; title: string; stage: Stage; close_reason: CloseReason | null; created_at: number; updated_at: number };

export type PublicProject = { name: string; slug: string };

/** SEC-21: an update on a public page has no author. */
export type PublicUpdate = { label: UpdateLabel; message: string; at: number };

export type PublicIncident = {
  title: string;
  origin: "auto" | "manual";
  severity: Severity;
  started_at: number;
  resolved_at: number | null;
  items: string[];
  updates: PublicUpdate[];
};

export type UptimeDay = { day: string; uptime: number | null; downtime_s: number };

export type PublicItem = { kind: "monitor" | "heartbeat"; name: string; state: StateKind; uptime_90d: number | null; days: UptimeDay[] };

export type PublicStatus = {
  status: { project: PublicProject; overall: StateKind; items: PublicItem[]; incidents: PublicIncident[] };
  recent: PublicIncident[];
};

export type PublicHistory = { project: PublicProject; incidents: PublicIncident[]; older: string | null };

export type AdminUserRow = { id: number; email: string; role: "super_admin" | "tl"; status: "active" | "invited" | "disabled"; created_at: number };

export type Invited = { email: string; link: string };

export type LimitRow = { key: string; label: string; group: string; help: string; value: number; default: number; max: number; at: number; default_at: number };

export type SystemView = {
  limits: LimitRow[];
  groups: string[];
  mail: { server: string | null; user: string | null; from: string };
  email: { sent: number; failed: number; waiting: number; last_sent_at: number | null; last_error: [string, number] | null };
  email_days: number;
  server: [string, string][];
  caps: { label: string; max: number; used: number | null }[];
};
