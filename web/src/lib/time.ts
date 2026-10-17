// Time display, matching src/platform/time.rs. Storage and the API use Unix seconds, UTC.

const pad = (n: number, w = 2): string => String(n).padStart(w, "0");

/** `2026-10-16 08:00 UTC` */
export function utc(ts: number): string {
  const d = new Date(ts * 1000);
  return `${d.getUTCFullYear()}-${pad(d.getUTCMonth() + 1)}-${pad(d.getUTCDate())} ${pad(d.getUTCHours())}:${pad(d.getUTCMinutes())} UTC`;
}

/** `2026-10-16`, the UTC day a timestamp falls on. */
export function day(ts: number): string {
  return utc(ts).slice(0, 10);
}

/** `14:02`, the UTC time of day. */
export function hhmm(ts: number): string {
  return utc(ts).slice(11, 16);
}

/** `45s`, `6m 18s`, `2h 4m`, `3d 2h`. */
export function duration(secs: number): string {
  const s = Math.max(0, Math.floor(secs));
  if (s < 60) return `${s}s`;
  if (s < 3600) return `${Math.floor(s / 60)}m ${s % 60}s`;
  if (s < 86_400) return `${Math.floor(s / 3600)}h ${Math.floor((s % 3600) / 60)}m`;
  return `${Math.floor(s / 86_400)}d ${Math.floor((s % 86_400) / 3600)}h`;
}

/** `just now`, `6m ago`, `2h ago`, `3d ago`. */
export function ago(ts: number, now: number = nowSecs()): string {
  const s = Math.max(0, now - ts);
  if (s < 60) return "just now";
  if (s < 3600) return `${Math.floor(s / 60)}m ago`;
  if (s < 86_400) return `${Math.floor(s / 3600)}h ago`;
  return `${Math.floor(s / 86_400)}d ago`;
}

export function nowSecs(): number {
  return Math.floor(Date.now() / 1000);
}
