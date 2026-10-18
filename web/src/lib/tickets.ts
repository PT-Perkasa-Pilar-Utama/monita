import type { CloseReason, Stage, TicketBoard } from "@/lib/types";

export const STAGES: [Stage, string][] = [
  ["triage", "Triage"],
  ["backlog", "Backlog"],
  ["in_progress", "In progress"],
  ["done", "Done"],
];

export const REASONS: [CloseReason, string][] = [
  ["fixed", "Fixed"],
  ["wont_fix", "Won't fix"],
  ["duplicate", "Duplicate"],
];

export const stageLabel = (s: Stage): string => STAGES.find(([k]) => k === s)?.[1] ?? s;

export const reasonLabel = (r: CloseReason): string => REASONS.find(([k]) => k === r)?.[1] ?? r;

/** What a column says under its name on the team's board. */
export const STAGE_NOTES: Record<Stage, string> = {
  triage: "New reports. Only your team sees them.",
  backlog: "Accepted. Shown on the public board.",
  in_progress: "Someone is working on it.",
  done: "Fixed, won't fix or duplicate.",
};

/** What a column says under its name on the public board. */
export const PUBLIC_NOTES: Partial<Record<Stage, string>> = {
  backlog: "Known, and waiting for a fix.",
  in_progress: "The team is working on it.",
  done: "Closed recently.",
};

/** The board's first page with later pages of some columns after it, each ticket once. */
export function withMore<T extends { number: number }>(first: T[], more: T[]): T[] {
  const seen = new Set(first.map((t) => t.number));
  return [...first, ...more.filter((t) => !seen.has(t.number) && seen.add(t.number))];
}

/** A card dropped on a column: nothing, a move, or a move that first needs a close reason. */
export function dropOutcome(from: Stage, to: Stage): "none" | "move" | "ask_reason" {
  if (from === to) return "none";
  return to === "done" ? "ask_reason" : "move";
}

/**
 * The board as it looks once ticket `n` sits in `to`, before the server answers: the card moves
 * to the top of its new column, Done keeps its reason, and both column counts change by one.
 */
export function moveOnBoard(board: TicketBoard, n: number, to: Stage, reason: CloseReason | null, now: number): TicketBoard {
  const t = board.tickets.find((x) => x.number === n);
  if (!t || t.stage === to) return board;
  const moved = { ...t, stage: to, close_reason: to === "done" ? reason : null, updated_at: now };
  return {
    tickets: [moved, ...board.tickets.filter((x) => x.number !== n)],
    counts: { ...board.counts, [t.stage]: board.counts[t.stage] - 1, [to]: board.counts[to] + 1 },
  };
}
