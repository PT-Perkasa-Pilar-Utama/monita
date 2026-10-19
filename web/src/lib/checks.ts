// Words for a monitor's or heartbeat's settings, shared by both detail pages.

import type { CheckKind } from "@/lib/types";

/** What a monitor can check, as its form offers it. */
export const CHECK_KINDS: { value: CheckKind; label: string }[] = [
  { value: "http", label: "HTTP" },
  { value: "tcp", label: "TCP port" },
  { value: "dns", label: "DNS name" },
];

/** The target field of the monitor form for each kind: its label and its example. */
export function targetField(kind: CheckKind): { label: string; placeholder: string } {
  if (kind === "tcp") return { label: "Host and port", placeholder: "db.internal:5432" };
  if (kind === "dns") return { label: "Host name", placeholder: "api.example.com" };
  return { label: "URL", placeholder: "https://api.example.com/healthz" };
}

/** What a failure does, from the two "When it fails" boxes. */
export function whenItFails(alert: boolean, autoIncident: boolean): string {
  if (alert && autoIncident) return "Sends alerts and opens an incident";
  if (alert) return "Sends alerts only";
  if (autoIncident) return "Opens an incident only";
  return "Nothing: Monita only records it";
}

/** `Name: value` lines as pairs; a line without a colon is skipped (the server refuses it). */
export function parseHeaderLines(headers: string): [string, string][] {
  return headers
    .split("\n")
    .map((l) => l.trim())
    .filter((l) => l.includes(":"))
    .map((l) => {
      const at = l.indexOf(":");
      return [l.slice(0, at).trim(), l.slice(at + 1).trim()];
    });
}

/** One header row of the monitor form. */
export type HeaderRow = { name: string; value: string };

/** Rows back to `Name: value` lines. A row with neither part is dropped; a half-filled row is
 * kept so the server names the problem. */
export function headerText(rows: HeaderRow[]): string {
  return rows
    .map((r) => ({ name: r.name.trim(), value: r.value.trim() }))
    .filter((r) => r.name || r.value)
    .map((r) => `${r.name}: ${r.value}`)
    .join("\n");
}

/** How a monitor signs in, as its form lists it. */
export const AUTH_KINDS = [
  { value: "none", label: "No sign-in" },
  { value: "bearer", label: "Bearer token" },
  { value: "basic", label: "User name and password" },
  { value: "header", label: "A key in a header" },
] as const;

export type AuthKind = (typeof AUTH_KINDS)[number]["value"];

/** A secret's name as a shell variable: `api-token` becomes `API_TOKEN`. */
export function envName(secret: string | null): string {
  const n = (secret ?? "").toUpperCase().replace(/[^A-Z0-9_]/g, "_");
  return /^[A-Z_]/.test(n) ? n : `SECRET_${n}`.replace(/_$/, "");
}

/** How a monitor signs in, in words, naming the secret but never its value. */
export function signInWords(m: { auth_kind: string; auth_secret_name: string | null; auth_user: string | null; auth_header: string | null }): string {
  const from = `from secret ${m.auth_secret_name ?? "(deleted)"}`;
  switch (m.auth_kind) {
    case "bearer":
      return `Bearer token ${from}`;
    case "basic":
      return `User ${m.auth_user ?? ""}, password ${from}`;
    case "header":
      return `${m.auth_header ?? ""} header ${from}`;
    default:
      return "None";
  }
}
