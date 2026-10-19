// Sentences for a project's settings at a glance, shown in the Overview's About column.
import type { AlertRoute, PublicPages } from "@/lib/types";

const TAB_NAMES: [keyof PublicPages, string][] = [
  ["incidents", "Incidents"],
  ["known_issues", "Known issues"],
  ["report", "Report a bug"],
];

/** The public tabs that are on, Uptime first because it always shows. */
export function publicTabs(pages: PublicPages): string[] {
  return ["Uptime", ...TAB_NAMES.filter(([k]) => pages[k]).map(([, name]) => name)];
}

/** "Critical: 2 emails and the webhook." for one alert route. */
export function alertLine(r: AlertRoute): string {
  const sev = r.severity === "critical" ? "Critical" : "Warning";
  if (!r.enabled) return `${sev}: off.`;
  const parts: string[] = [];
  if (r.emails.length > 0) parts.push(r.emails.length === 1 ? "1 email" : `${r.emails.length} emails`);
  if (r.use_webhook && r.webhook_url) parts.push("the webhook");
  return parts.length === 0 ? `${sev}: no one is alerted.` : `${sev}: ${parts.join(" and ")}.`;
}

/** How many addresses or ranges the allowlist holds; 0 means it is off. */
export function allowlistSize(list: string | null): number {
  return (list ?? "").split(/[\n,]/).filter((l) => l.trim() !== "").length;
}
