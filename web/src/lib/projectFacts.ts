// Sentences for a project's settings at a glance, shown in the Overview's About column.
import type { PublicPages } from "@/lib/types";

const TAB_NAMES: [keyof PublicPages, string][] = [
  ["incidents", "Incidents"],
  ["known_issues", "Issues"],
  ["report", "Ticket"],
  ["changelog", "Changelog"],
];

/** The public tabs that are on, Uptime first because it always shows. */
export function publicTabs(pages: PublicPages): string[] {
  return ["Uptime", ...TAB_NAMES.filter(([k]) => pages[k]).map(([, name]) => name)];
}

/** How many addresses or ranges the allowlist holds; 0 means it is off. */
export function allowlistSize(list: string | null): number {
  return (list ?? "").split(/[\n,]/).filter((l) => l.trim() !== "").length;
}
