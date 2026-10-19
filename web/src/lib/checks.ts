// Words for a monitor's or heartbeat's settings, shared by both detail pages.

/** What a failure does, from the two "When it fails" boxes. */
export function whenItFails(alert: boolean, autoIncident: boolean): string {
  if (alert && autoIncident) return "Sends alerts and opens an incident";
  if (alert) return "Sends alerts only";
  if (autoIncident) return "Opens an incident only";
  return "Nothing: Monita only records it";
}

/** How many `Name: value` lines a monitor sends; the values stay on its form. */
export function headerCount(headers: string): number {
  return headers.split("\n").filter((l) => l.trim() !== "").length;
}
