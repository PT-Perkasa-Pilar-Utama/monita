import type { ReactElement, ReactNode } from "react";
import { LabelBadge } from "@/components/app/Tone";
import { AlertCircle, CheckCircle, ChevronDown, MinusCircle, XCircle } from "@/components/icons";
import { barTitle, barTone, incidentState, span } from "@/lib/status";
import { stamp } from "@/lib/time";
import type { PublicIncident, PublicItem, StateKind, UptimeDay } from "@/lib/types";
import { cn } from "@/lib/utils";

const STATE_WORD = { up: "Up", down: "Down", issue: "Issue", unknown: "No data yet" } as const;

export const uptimeText = (i: PublicItem): string => (i.uptime_90d === null ? "no data yet" : `${i.uptime_90d.toFixed(2)}% uptime`);

const STATE_LINE = { up: "Normal", down: "Down", issue: "Has an issue", unknown: "No data yet" } as const;

/** A check, a warning or a cross in the state's colour; a dash before the first check. */
export function StateIcon({ kind, label = STATE_WORD[kind], className }: { kind: StateKind; label?: string; className?: string }): ReactElement {
  const cls = (tone: string): string => cn("size-6 shrink-0", tone, className);
  if (kind === "up") return <CheckCircle weight="fill" aria-label={label} className={cls("text-up")} />;
  if (kind === "down") return <XCircle weight="fill" aria-label={label} className={cls("text-down")} />;
  if (kind === "issue") return <AlertCircle weight="fill" aria-label={label} className={cls("text-issue")} />;
  return <MinusCircle weight="fill" aria-label={label} className={cls("text-muted-foreground")} />;
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

/** A public incident as a row: its state icon, title, newest update and when. It opens to every
 * update. The same row serves Happening now, the months and the history. SEC-21: no author, no cause. */
export function IncidentRow({ incident: i }: { incident: PublicIncident }): ReactElement {
  const [kind, word] = incidentState(i);
  const latest = i.updates[0];
  return (
    <li>
      <details className="group">
        <summary className="flex cursor-pointer list-none gap-3 group/row px-5 py-4 sm:px-6 [&::-webkit-details-marker]:hidden">
          <StateIcon kind={kind} label={word} className="mt-px size-5" />
          <span className="flex min-w-0 flex-1 flex-col gap-1.5">
            <span className="flex items-start justify-between gap-3">
              <strong className="text-[15px] leading-snug font-semibold group-hover/row:underline">{i.title}</strong>
              <ChevronDown className="mt-0.5 size-4 shrink-0 text-muted-foreground transition-transform group-open:rotate-180" />
            </span>
            {latest && <span className="line-clamp-2 break-words whitespace-pre-wrap group-open:hidden">{latest.message}</span>}
            <span className="flex flex-wrap items-center gap-x-3 gap-y-1 text-sm text-muted-foreground">
              {latest && <LabelBadge label={latest.label} />}
              {span(i)}
            </span>
          </span>
        </summary>
        <div className="flex flex-col gap-4 pr-5 pb-5 pl-13 sm:pr-6 sm:pl-14">
          <Updates incident={i} />
        </div>
      </details>
    </li>
  );
}

/** Incident rows in one card. `footer` closes the card, such as Show all. */
export function IncidentList({ incidents, footer }: { incidents: PublicIncident[]; footer?: ReactNode }): ReactElement {
  return (
    <div className="overflow-hidden rounded-xl border bg-card shadow-xs">
      <ol className="divide-y">
        {incidents.map((i, n) => (
          <IncidentRow key={n} incident={i} />
        ))}
      </ol>
      {footer && <div className="border-t">{footer}</div>}
    </div>
  );
}
