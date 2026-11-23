// The compact Projects view's rules (v1.2 Q53).
import type { StateKind } from "@/lib/types";
import type { ViewMode } from "@/lib/useViewMode";

const STATE: Record<StateKind, string> = { up: "up", down: "down", issue: "has an issue", maintenance: "under maintenance", paused: "paused", unknown: "no data yet" };

/** A tile's tooltip and accessible name: the name hidden on the tile, and its state in words. */
export function tileLabel(name: string, kind: StateKind): string {
  return `${name}, ${STATE[kind]}`;
}

/** The compact view pages 100 projects and the others 20, so a switch between them goes back to page 1. */
export function switchResetsPage(from: ViewMode, to: ViewMode): boolean {
  return (from === "compact") !== (to === "compact");
}
