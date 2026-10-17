import type { CloseReason, Stage } from "@/lib/types";

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
