// Words for a monitor's or heartbeat's settings, shared by both detail pages.

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
