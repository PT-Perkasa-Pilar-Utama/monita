import { clsx } from "clsx";
import type { ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]): string {
  return twMerge(clsx(inputs));
}

// todayISODate returns the local calendar date as YYYY-MM-DD. Used as the `max`
// of date inputs that must not accept a future date (workflow fixed date,
// workspace mock start date, manual-run override). The backend enforces the
// same rule (pkg/datectx.IsFuture); this is the UX guard.
export function todayISODate(): string {
  const d = new Date();
  const pad = (n: number): string => (n < 10 ? `0${n}` : `${n}`);
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

// isCalendarDay reports whether a string is a real day as YYYY-MM-DD.
// 2026-02-30 has the shape but no day, and a Date built from it rolls into
// March, so the check is a round trip, not a regex.
export function isCalendarDay(value: string): boolean {
  if (!/^\d{4}-\d{2}-\d{2}$/.test(value)) return false;
  const d = new Date(`${value}T00:00:00Z`);
  return !Number.isNaN(d.getTime()) && d.toISOString().slice(0, 10) === value;
}

// formatDMY renders a date as DD/MM/YYYY regardless of browser locale (en-GB
// ordering). Returns "-" for empty/unparseable input.
export function formatDMY(value: string | number | null | undefined): string {
  if (!value) return "-";
  const d = new Date(value);
  if (Number.isNaN(d.getTime())) return "-";
  return d.toLocaleDateString("en-GB");
}

// dayDMY renders a YYYY-MM-DD day as DD/MM/YYYY without a time zone, so the
// day never shifts. Anything else returns "-".
export function dayDMY(day: string | null | undefined): string {
  const m = day ? /^(\d{4})-(\d{2})-(\d{2})/.exec(day) : null;
  return m ? `${m[3]}/${m[2]}/${m[1]}` : "-";
}

// formatDMYTime renders a timestamp as DD/MM/YYYY, HH:MM:SS (en-GB), keeping the
// time for the started/completed columns. Returns "-" for empty/unparseable input.
export function formatDMYTime(value: string | number | null | undefined): string {
  if (!value) return "-";
  const d = new Date(value);
  if (Number.isNaN(d.getTime())) return "-";
  return d.toLocaleString("en-GB");
}
