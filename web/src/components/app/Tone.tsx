import type { ReactElement } from "react";
import { Bell, BellRinging } from "@/components/icons";
import { Badge } from "@/components/ui/badge";
import { reasonLabel, stageLabel } from "@/lib/tickets";
import type { CloseReason, Severity, Stage, StateKind, UpdateLabel } from "@/lib/types";

const STATE: Record<StateKind, string> = { up: "Up", down: "Down", issue: "Issue", maintenance: "Maintenance", paused: "Paused", unknown: "No data yet" };

/** A monitor's or heartbeat's state: Up, Down, Issue, Maintenance, or No data yet before its first check or ping. */
export function StateBadge({ kind, label, id }: { kind: StateKind; label?: string; id?: string }): ReactElement {
  return (
    <Badge id={id} variant={kind === "paused" ? "unknown" : kind}>
      {label ?? STATE[kind]}
    </Badge>
  );
}

/**
 * Which alert setup a monitor, heartbeat or incident uses. It is a setting, not a state, so it is
 * grey: beside a green Up, a red Critical read as an outage. The bell tells the two apart.
 */
export function SeverityBadge({ severity }: { severity: Severity }): ReactElement {
  const critical = severity === "critical";
  return (
    <Badge variant="outline">
      {critical ? <BellRinging weight="fill" aria-hidden /> : <Bell aria-hidden />}
      {critical ? "Critical" : "Warning"}
    </Badge>
  );
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
