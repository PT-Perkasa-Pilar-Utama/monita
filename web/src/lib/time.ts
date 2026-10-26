// Time display, matching src/platform/time.rs. Storage and the API use Unix seconds, UTC. Pages
// show times in the fixed UTC offset a super admin sets; the app and the public pages load it
// once and call `setOffset`.

const pad = (n: number, w = 2): string => String(n).padStart(w, "0");

let offsetMin = 0;

/** Minutes east of UTC that every time on the page uses. */
export function setOffset(min: number): void {
  offsetMin = min;
}

/** The offset every time on the page uses, in minutes east of UTC. */
export function offset(): number {
  return offsetMin;
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

/** `2026-10-16T15:00`, a `datetime-local` input's value for the time in the offset. */
export function inputValue(ts: number, off: number = offsetMin): string {
  return local(ts, off).replace(" ", "T");
}

/** The Unix seconds a `datetime-local` value names in the offset, or null when it is not a time. */
export function fromInputValue(v: string, off: number = offsetMin): number | null {
  const m = /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2})/.exec(v);
  if (!m) return null;
  const [y, mo, d, h, mi] = m.slice(1).map(Number) as [number, number, number, number, number];
  return Date.UTC(y, mo - 1, d, h, mi) / 1000 - off * 60;
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

/** The digest keeps its UTC hour (MONITA_DIGEST_UTC_HOUR) whatever offset is chosen. */
export function digestNote(hourUtc: number, off: number): string {
  const utc = `${pad(hourUtc)}:00 UTC`;
  return off === 0 ? `The daily ticket digest goes out at ${utc}.` : `The daily ticket digest stays at ${utc}, ${hhmm(hourUtc * 3600, off)} in ${zone(off)}.`;
}

/** Minutes east of UTC on this computer right now. */
export function browserOffset(): number {
  return -new Date().getTimezoneOffset();
}

/**
 * One line under a window's start and end: when it starts and how long it lasts, so a time typed
 * in the wrong zone shows before it is saved. When this computer's zone is not Monita's, it says
 * so too.
 */
export function windowHint(starts: number | null, ends: number | null, now: number, off: number, browser: number): string {
  let when = "";
  if (starts !== null && ends !== null && ends > starts) {
    const minutes = (secs: number): number => Math.round(secs / 60) * 60;
    const start = starts <= now + 60 ? "It starts now" : `It starts in ${duration(minutes(starts - now))}`;
    when = `${start} and lasts ${duration(minutes(ends - Math.max(starts, now)))}.`;
  }
  if (browser === off) return when;
  const gap = Math.abs(browser - off);
  const ahead = `${duration(gap * 60).replace(" 0m", "")} ${browser > off ? "behind" : "ahead of"} your computer`;
  return `${when} Times here are in ${zone(off)}, Monita's time zone, which is ${ahead}.`.trim();
}
