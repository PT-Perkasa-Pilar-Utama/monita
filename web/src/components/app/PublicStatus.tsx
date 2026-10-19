import type { ReactElement } from "react";
import { LabelBadge, StateBadge } from "@/components/app/Tone";
import { AlertCircle, CheckCircle, ChevronDown, MinusCircle, XCircle } from "@/components/icons";
import { barTitle, barTone, incidentState, when } from "@/lib/status";
import { stamp } from "@/lib/time";
import type { PublicIncident, PublicItem, StateKind, UptimeDay } from "@/lib/types";
import { cn } from "@/lib/utils";

const STATE_WORD = { up: "Up", down: "Down", issue: "Issue", unknown: "No data yet" } as const;

export const uptimeText = (i: PublicItem): string => (i.uptime_90d === null ? "no data yet" : `${i.uptime_90d.toFixed(2)}% uptime`);

const STATE_LINE = { up: "Normal", down: "Down", issue: "Has an issue", unknown: "No data yet" } as const;

/** A check, a warning or a cross in the state's colour; a dash before the first check. */
export function StateIcon({ kind }: { kind: StateKind }): ReactElement {
  const label = STATE_WORD[kind];
  if (kind === "up") return <CheckCircle weight="fill" aria-label={label} className="size-6 shrink-0 text-up" />;
  if (kind === "down") return <XCircle weight="fill" aria-label={label} className="size-6 shrink-0 text-down" />;
  if (kind === "issue") return <AlertCircle weight="fill" aria-label={label} className="size-6 shrink-0 text-issue" />;
  return <MinusCircle weight="fill" aria-label={label} className="size-6 shrink-0 text-muted-foreground" />;
}

/** A bar per day, oldest on the left. */
export function DayBars({ days }: { days: UptimeDay[] }): ReactElement {
  return (
    <div className="flex h-9 gap-px sm:gap-[2px]" data-uptime-bars>
      {days.map((d) => {
        const tone = barTone(d);
        return (
          <span
            key={d.day}
            title={barTitle(d)}
            className={cn(
              "min-w-0 flex-1 rounded-[2px]",
              tone === "up" && "bg-up",
              tone === "partial" && "bg-issue",
              tone === "down" && "bg-down",
              tone === "none" && "bg-muted",
            )}
          />
        );
      })}
    </div>
  );
}

/** One public system as a cell of the uptime grid: name, state, 90 bars and the uptime between
 * "90 days ago" and "Today". */
export function SystemCard({ item }: { item: PublicItem }): ReactElement {
  return (
    <div className="flex min-w-0 flex-col gap-3 bg-card px-6 py-5">
      <div className="flex items-center justify-between gap-3">
        <strong className="truncate text-[15px] font-semibold">{item.name}</strong>
        <StateIcon kind={item.state} />
      </div>
      <DayBars days={item.days} />
      <div className="flex items-center gap-3 text-sm text-muted-foreground">
        <span className="shrink-0">{item.days.length} days ago</span>
        <span aria-hidden className="h-px min-w-4 flex-1 bg-border" />
        <span className="shrink-0">{uptimeText(item)}</span>
        <span aria-hidden className="h-px min-w-4 flex-1 bg-border" />
        <span className="shrink-0">Today</span>
      </div>
      <p className={cn("text-sm", item.state === "down" ? "text-down" : item.state === "issue" ? "text-issue" : "text-muted-foreground")}>{STATE_LINE[item.state]}</p>
    </div>
  );
}

/** The updates of a public incident, newest first. SEC-21: no author, no cause. */
export function Updates({ incident: i }: { incident: PublicIncident }): ReactElement {
  return (
    <>
      <p className="text-sm text-muted-foreground">Affects {i.items.join(", ")}.</p>
      <ol className="flex flex-col gap-4 border-l pl-4">
        {i.updates.map((u, n) => (
          <li key={n} className="flex flex-col gap-1.5">
            <span className="flex flex-wrap items-center gap-x-3 gap-y-1 text-sm text-muted-foreground">
              <LabelBadge label={u.label} />
              {stamp(u.at)}
            </span>
            <p className="whitespace-pre-wrap break-words">{u.message}</p>
          </li>
        ))}
      </ol>
    </>
  );
}

/** A public incident, folded unless it is still open. SEC-21: no author, no cause. */
export function IncidentCard({ incident: i }: { incident: PublicIncident }): ReactElement {
  const [kind, word] = incidentState(i);
  const open = i.resolved_at === null;
  return (
    <details
      open={open}
      className={cn("group rounded-xl border bg-card shadow-xs", open && (kind === "down" ? "border-down/40" : "border-issue/40"))}
    >
      <summary className="flex cursor-pointer list-none flex-col gap-2 p-5 [&::-webkit-details-marker]:hidden">
        <span className="flex items-start justify-between gap-3">
          <strong className="text-[15px] font-semibold">{i.title}</strong>
          <ChevronDown className="mt-0.5 size-4 shrink-0 text-muted-foreground transition-transform group-open:rotate-180" />
        </span>
        <span className="flex flex-wrap items-center gap-x-3 gap-y-1 text-sm text-muted-foreground">
          <StateBadge kind={kind} label={word} />
          {when(i)}
        </span>
      </summary>
      <div className="flex flex-col gap-4 px-5 pb-5">
        <Updates incident={i} />
      </div>
    </details>
  );
}
