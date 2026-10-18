import type { ReactElement } from "react";
import { Badge } from "@/components/ui/badge";
import { reasonLabel, stageLabel } from "@/lib/tickets";
import type { CloseReason, Severity, Stage, StateKind, UpdateLabel } from "@/lib/types";

const STATE: Record<StateKind, string> = { up: "Up", down: "Down", issue: "Issue", unknown: "Waiting" };

/** A monitor's or heartbeat's state: Up, Down, Issue, or Waiting before its first result. */
export function StateBadge({ kind, label, id }: { kind: StateKind; label?: string; id?: string }): ReactElement {
  return (
    <Badge id={id} variant={kind}>
      {label ?? STATE[kind]}
    </Badge>
  );
}

/** Critical alerts by email and webhook; Warning by email only. */
export function SeverityBadge({ severity }: { severity: Severity }): ReactElement {
  return <Badge variant={severity === "critical" ? "down" : "issue"}>{severity === "critical" ? "Critical" : "Warning"}</Badge>;
}

const LABEL: Record<UpdateLabel, [string, "down" | "issue" | "neutral" | "up"]> = {
  investigating: ["Investigating", "down"],
  identified: ["Identified", "issue"],
  monitoring: ["Monitoring", "neutral"],
  resolved: ["Resolved", "up"],
};

export const labelTitle = (l: UpdateLabel): string => LABEL[l][0];

/** A status update's label, as the status page shows it. */
export function LabelBadge({ label }: { label: UpdateLabel }): ReactElement {
  const [text, variant] = LABEL[label];
  return <Badge variant={variant}>{text}</Badge>;
}

type Tone = "up" | "issue" | "neutral" | "unknown";

// Grey until someone accepts it, yellow while worked on, green once closed. Magenta stays the brand.
export const STAGE_TONE: Record<Stage, Tone> = { triage: "unknown", backlog: "neutral", in_progress: "issue", done: "up" };

const REASON_TONE: Record<CloseReason, Tone> = { fixed: "up", wont_fix: "neutral", duplicate: "unknown" };

/** A ticket's column as a coloured pill. `ring` puts the column's ring in front. */
export function StageBadge({ stage, ring }: { stage: Stage; ring?: ReactElement }): ReactElement {
  return (
    <Badge variant={STAGE_TONE[stage]}>
      {ring}
      {stageLabel(stage)}
    </Badge>
  );
}

/** Why a Done ticket closed: Fixed in green, Won't fix and Duplicate in grey. */
export function ReasonBadge({ reason }: { reason: CloseReason }): ReactElement {
  return <Badge variant={REASON_TONE[reason]}>{reasonLabel(reason)}</Badge>;
}
