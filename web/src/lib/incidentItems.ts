import { matches } from "@/lib/filter";
import type { IncidentItem } from "@/lib/types";

const byName = new Intl.Collator(undefined, { numeric: true, sensitivity: "base" });

/** The report form's two lists: the items whose name matches `q`, split by kind, by name with numbers in order (API 2 before API 11). */
export function pickItems(items: IncidentItem[], q: string): { monitors: IncidentItem[]; heartbeats: IncidentItem[] } {
  const shown = items.filter((i) => matches(q, i.name)).sort((a, b) => byName.compare(a.name, b.name));
  return { monitors: shown.filter((i) => i.kind === "monitor"), heartbeats: shown.filter((i) => i.kind === "heartbeat") };
}
