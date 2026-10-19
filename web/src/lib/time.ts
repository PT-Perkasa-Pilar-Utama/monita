// Time display, matching src/platform/time.rs. Storage and the API use Unix seconds, UTC. Pages
// show times in the fixed UTC offset a super admin sets; the app and the public pages load it
// once and call `setOffset`.

const pad = (n: number, w = 2): string => String(n).padStart(w, "0");

let offsetMin = 0;

/** Minutes east of UTC that every time on the page uses. */
export function setOffset(min: number): void {
  offsetMin = min;
}

/** `UTC`, `UTC+7`, `UTC+5:30`, `UTC-3`. */
export function zone(off: number = offsetMin): string {
  if (off === 0) return "UTC";
  const a = Math.abs(off);
  const hm = a % 60 === 0 ? `${a / 60}` : `${Math.floor(a / 60)}:${pad(a % 60)}`;
  return `UTC${off > 0 ? "+" : "-"}${hm}`;
}

/** `2026-10-16 15:00`, the time in the offset without its name. */
function local(ts: number, off: number): string {
  const d = new Date((ts + off * 60) * 1000);
  return `${d.getUTCFullYear()}-${pad(d.getUTCMonth() + 1)}-${pad(d.getUTCDate())} ${pad(d.getUTCHours())}:${pad(d.getUTCMinutes())}`;
}

/** `2026-10-16 15:00 UTC+7`: the time in the offset, with the offset named. */
export function stamp(ts: number, off: number = offsetMin): string {
  return `${local(ts, off)} ${zone(off)}`;
}

/** `2026-10-16`, the day a timestamp falls on in the offset. */
export function day(ts: number, off: number = offsetMin): string {
  return local(ts, off).slice(0, 10);
}

/** `14:02`, the time of day in the offset. */
export function hhmm(ts: number, off: number = offsetMin): string {
  return local(ts, off).slice(11, 16);
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
