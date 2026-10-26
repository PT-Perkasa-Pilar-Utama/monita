import { readLocal, writeLocal } from "@/lib/store";

/** One release as the What's new dialog shows it: a few plain highlights, not the full notes. */
export type WhatsNew = { version: string; summary: string; highlights: { title: string; text: string }[] };

/** Newest first. Add an entry with each release that changes what people see or do. */
export const RELEASES: WhatsNew[] = [
  {
    version: "1.1.0",
    summary: "Monita 1.1 adds maintenance windows, more kinds of checks and new places to send alerts.",
    highlights: [
      { title: "Plan maintenance or pause", text: "Planned work opens no incident and sends no alert, and you can pause a monitor or heartbeat, or mark an incident as planned work or a false alarm afterwards." },
      { title: "More kinds of checks", text: "Watch a TCP port, a DNS name or a ping beside web addresses, and get a warning before an HTTPS certificate expires." },
      { title: "Alerts where your team is", text: "Alert cards send to email lists, Discord, Telegram, ntfy, PagerDuty or any webhook, each listening to critical alerts, warnings or both." },
      { title: "Check from more than one place", text: "Locations run your checks from other networks too, so a fault in this server's own network is not mistaken for an outage." },
      { title: "Two-factor sign-in", text: "Protect your account with an authenticator app and recovery codes." },
      { title: "Manage projects from scripts", text: "Project API keys can now read or change monitors, heartbeats, incidents and maintenance, for example to pause a monitor while you deploy." },
    ],
  },
];

/** The release notes on GitHub for one version. */
export function releaseUrl(version: string): string {
  return `https://github.com/PT-Perkasa-Pilar-Utama/monita/releases/tag/v${version}`;
}

/** -1, 0 or 1 for two versions such as 1.0.1 and 1.1.0, part by part as numbers. */
export function compareVersions(a: string, b: string): number {
  const pa = a.split(".").map((n) => Number.parseInt(n, 10) || 0);
  const pb = b.split(".").map((n) => Number.parseInt(n, 10) || 0);
  for (let i = 0; i < Math.max(pa.length, pb.length); i++) {
    const d = (pa[i] ?? 0) - (pb[i] ?? 0);
    if (d !== 0) return d > 0 ? 1 : -1;
  }
  return 0;
}

/** The banner shows until this browser has seen the newest release, once per release. */
export function showsWhatsNew(latest: string, seen: string): boolean {
  return seen === "" || compareVersions(latest, seen) > 0;
}

/** Per user, as the two-factor note is, so a second account in the same browser sees its own. */
const key = (email: string): string => `monita.whats-new-seen:${email}`;

export function seenVersion(email: string): string {
  return readLocal(key(email));
}

export function markSeen(email: string, version: string): void {
  writeLocal(key(email), version);
}
