import type { ReactElement } from "react";
import { Badge } from "@/components/ui/badge";
import type { Severity, StateKind, UpdateLabel } from "@/lib/types";

const STATE: Record<StateKind, string> = { up: "Up", down: "Down", issue: "Issue", unknown: "Waiting" };

/** A monitor's or heartbeat's state: Up, Down, Issue, or Waiting before its first result. */
export function StateBadge({ kind, label }: { kind: StateKind; label?: string }): ReactElement {
  return <Badge variant={kind}>{label ?? STATE[kind]}</Badge>;
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
