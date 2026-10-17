import type { ReactElement } from "react";
import { LabelBadge, StateBadge } from "@/components/app/Tone";
import { ChevronDown } from "@/components/icons";
import { barTitle, barTone, incidentState, when } from "@/lib/status";
import { utc } from "@/lib/time";
import type { PublicIncident, PublicItem } from "@/lib/types";
import { cn } from "@/lib/utils";

const STATE_WORD = { up: "Up", down: "Down", issue: "Issue", unknown: "Waiting" } as const;

export const uptimeText = (i: PublicItem): string => (i.uptime_90d === null ? "no data yet" : `${i.uptime_90d.toFixed(2)}% uptime`);

/** One public system: its state, its 90-day uptime, and a bar per day. */
export function SystemRow({ item }: { item: PublicItem }): ReactElement {
  return (
    <div className="flex flex-col gap-2">
      <div className="flex items-center justify-between gap-3">
        <span className="flex min-w-0 items-center gap-2">
          <StateBadge kind={item.state} label={STATE_WORD[item.state]} />
          <strong className="truncate font-semibold">{item.name}</strong>
        </span>
        <span className="shrink-0 text-sm text-muted-foreground">{uptimeText(item)}</span>
      </div>
      <div className="flex h-8 gap-[2px]" data-uptime-bars>
        {item.days.map((d) => {
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
    </div>
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
        <p className="text-sm text-muted-foreground">Affects {i.items.join(", ")}.</p>
        <ol className="flex flex-col gap-4 border-l pl-4">
          {i.updates.map((u, n) => (
            <li key={n} className="flex flex-col gap-1.5">
              <span className="flex flex-wrap items-center gap-x-3 gap-y-1 text-sm text-muted-foreground">
                <LabelBadge label={u.label} />
                {utc(u.at)}
              </span>
              <p className="whitespace-pre-wrap break-words">{u.message}</p>
            </li>
          ))}
        </ol>
      </div>
    </details>
  );
}
