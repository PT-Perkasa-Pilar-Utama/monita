// Response shapes of /api/ui. Each mirrors a `Serialize` struct in src/<module>/api.rs.

export type Project = {
  name: string;
  slug: string;
  owner_email: string;
  description: string | null;
  icon_url: string | null;
  ip_allowlist: string | null;
  created_at: number;
  pages: PublicPages;
  /** Where people use the system; null for a project made before it was asked. */
  homepage_url: string | null;
};

/** The public tabs besides Status, which always shows. */
export type PublicPages = { incidents: boolean; known_issues: boolean; report: boolean; changelog: boolean };

export type Owner = { email: string; role: string };

/** One page of the project list; `total` counts every project that matches the search. */
/** How a project is doing right now, for the project list. `hours` holds the last 24, oldest first. */
export type Pulse = { kind: StateKind; monitors: Summary; heartbeats: Summary; incidents: number; tickets: number; hours: UptimeDay[] };

export type ProjectRow = Project & { pulse: Pulse };

export type ProjectList = {
  projects: ProjectRow[];
  total: number;
  page: number;
  page_size: number;
  owners: Owner[];
  /** Who owns a project, for a super admin's owner filter; empty for a tech lead. */
  owner_options: string[];
};

/** A project's numbers. Tabs show only the ones that flag a problem: down systems, open
 * incidents, tickets in Triage. */
export type TabCounts = { monitors: number; heartbeats: number; monitors_down: number; heartbeats_down: number; incidents: number; tickets: number; triage: number };

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

export type AlertRoute = { severity: Severity; emails: string[]; webhook_url: string | null; use_webhook: boolean; enabled: boolean };

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
  alert: boolean;
  auto_incident: boolean;
  fails_to_open: number;
  method: "GET" | "HEAD" | "POST" | "PUT";
  /** `Name: value` lines. */
  headers: string;
  body: string | null;
  /** How the check signs in. Never the secret's value. */
  auth_kind: "none" | "bearer" | "basic" | "header";
  auth_secret_id: number | null;
  /** Set on one monitor's page only. */
  auth_secret_name: string | null;
  auth_user: string | null;
  auth_header: string | null;
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
  alert: boolean;
  auto_incident: boolean;
  last_ping_at: number | null;
  /** The newest ping, good or failed; `last_ping_at` counts good pings only. */
  last: { ok: boolean; cause: string | null; at: number } | null;
  deadline_at: number | null;
  created_at: number;
  state: StateKind;
  due: Due;
};

export type HeartbeatList = { heartbeats: HeartbeatView[]; summary: ListSummary };

export type PingRow = { id: number; received_at: number; kind: "ok" | "fail"; source_ip: string | null; cause: string | null };

export type HeartbeatDetail = {
  heartbeat: HeartbeatView;
  /** Null for a supervisor, who must not get the heartbeat's token. */
  ping_url: string | null;
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
  /** Failed checks in a row a monitor may count as down. */
  /** The failed-check counts a monitor may choose from. */
  fails: { min: number; max: number };
  monitor: { interval_s: number; timeout_s: number; expected_status: string; severity: Severity };
  heartbeat: { period_s: number; grace_s: number; severity: Severity };
};

export type UpdateRow = { label: UpdateLabel; message: string; author_email: string | null; created_at: number };

export type IncidentDetail = { incident: IncidentRow; items: string[]; updates: UpdateRow[]; labels: UpdateLabel[] };

/** A monitor or heartbeat a manual incident may name. `state` is set only when it is down or has an issue. */
export type IncidentItem = { value: string; kind: "monitor" | "heartbeat"; name: string; state: "down" | "issue" | null };

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
  /** The ref ID's last 4 characters; null for a team ticket or an older ref ID. */
  ref_tail: string | null;
  ref_issued_at: number | null;
  // The team member who filed it; null for a reporter's ticket.
  filed_by: string | null;
};

export type StageCounts = Record<Stage, number>;

export type TicketBoard = { tickets: TicketRow[]; counts: StageCounts };

/** Who hears about new tickets: the owner's daily digest, and a webhook per ticket. */
export type TicketAlerts = { enabled: boolean; digest: boolean; webhook_url: string | null; use_webhook: boolean; webhook_moves: boolean };

/** One more page of one column (`?stage=&page=`). */
export type ColumnPage<T> = { tickets: T[]; page: number; total: number; page_size: number };

/** SEC-14: what a visitor may see of a ticket. */
export type PublicTicket = { number: number; title: string; stage: Stage; created_at: number; updated_at: number };

/** The public board has no Triage column, not even a count (SEC-14). */
export type PublicBoard = { tickets: PublicTicket[]; counts: Omit<StageCounts, "triage"> };

export type MyTicket = { number: number; title: string; stage: Stage; close_reason: CloseReason | null; created_at: number; updated_at: number };

export type PublicProject = { name: string; slug: string };

/** The public layout's project: its name and the tabs it shows. */
export type PublicSite = PublicProject & { pages: PublicPages; utc_offset_min: number; homepage_url: string | null };

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

export type PublicItem = { kind: "monitor" | "heartbeat"; id: number; name: string; state: StateKind; uptime_90d: number | null; days: UptimeDay[] };

/** One public system, as the past-uptime picker lists it. */
export type PublicSystem = { kind: "monitor" | "heartbeat"; id: number; name: string };

export type UptimeMonth = { month: string; uptime: number | null; days: UptimeDay[] };

/** Three months of one system. `to` values move the window; null at an end. */
export type PublicCalendar = { project: PublicProject; systems: PublicSystem[]; system: PublicSystem | null; months: UptimeMonth[]; prev_to: number | null; next_to: number | null };

/** One month of public incidents; `more` is the history cursor when the month has more than one page. */
export type IncidentMonth = { month: string; incidents: PublicIncident[]; more: string | null };

export type PublicMonths = { project: PublicProject; months: IncidentMonth[]; prev_to: number | null; next_to: number | null };

export type PublicStatus = {
  status: { project: PublicProject; overall: StateKind; items: PublicItem[]; incidents: PublicIncident[] };
  recent: PublicIncident[];
};

export type PublicHistory = { project: PublicProject; incidents: PublicIncident[]; older: string | null };

export type AdminUserRow = { id: number; email: string; role: "super_admin" | "tl" | "supervisor"; status: "active" | "invited" | "disabled"; created_at: number };

export type Invited = { email: string; link: string };

export type LimitRow = { key: string; label: string; group: string; help: string; value: number; default: number; max: number; at: number; default_at: number };

export type SystemView = {
  limits: LimitRow[];
  groups: string[];
  mail: MailView;
  email: { sent: number; failed: number; waiting: number; last_sent_at: number | null; last_error: [string, number] | null };
  email_days: number;
  server: [string, string][];
  caps: { label: string; max: number; used: number | null }[];
  utc_offset_min: number;
};

/** The mail server on the System page. The password never comes back; `has_password` says one is saved. */
export type MailView = {
  source: "saved" | "env" | "none";
  server: string | null;
  user: string | null;
  from: string;
  host: string | null;
  port: number | null;
  security: "tls" | "starttls" | "none" | null;
  has_password: boolean;
  skip_verify: boolean;
};

/** A sealed project secret, by name. Its value never reaches the browser. */
export type Secret = { id: number; name: string; updated_at: number; used_by: number };

/** An open incident with the project it belongs to, for the cross-project Incidents page. */
export type HomeIncident = IncidentRow & { project: Project };

/** A new ticket in Triage with its project, for the cross-project Tickets page. */
export type HomeTicket = { project: Project; number: number; title: string; created_at: number };

export type HomeList<K extends string, T> = Record<K, T[]> & { total: number; page: number; page_size: number };


/** One kind of change in a release, and its lines. */
export type ReleaseSection = { tag: "Added" | "Changed" | "Fixed" | "Removed"; items: string[] };

/** A release as the team edits it. */
export type Release = { id: number; version: string; released_on: string; title: string; sections: ReleaseSection[] };

export type ReleaseList = { releases: Release[]; total: number; page: number; page_size: number };

/** A release in the public index. */
export type ReleaseHead = { version: string; title: string; released_on: string };

export type PublicReleaseIndex = { releases: ReleaseHead[]; older: string | null };

export type PublicRelease = { release: Omit<Release, "id">; newer: ReleaseHead | null; older: ReleaseHead | null };
