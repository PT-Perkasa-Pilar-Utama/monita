import type { ReactElement, ReactNode } from "react";
import { stageLabel } from "@/lib/tickets";
import type { Stage, StageCounts } from "@/lib/types";
import { cn } from "@/lib/utils";

/** A ring per column: dashed for Triage, empty for Backlog, half for In progress, full for Done. */
function StageRing({ stage }: { stage: Stage }): ReactElement {
  return (
    <span
      aria-hidden="true"
      className={cn(
        "inline-block size-3 shrink-0 rounded-full border-[1.5px]",
        stage === "triage" && "border-dashed border-muted-foreground",
        stage === "backlog" && "border-muted-foreground",
        stage === "in_progress" && "border-issue bg-[conic-gradient(var(--issue)_0_50%,transparent_50%_100%)]",
        stage === "done" && "border-neutral-strong bg-neutral-strong",
      )}
    />
  );
}

/**
 * Columns of ticket cards, one per stage. A column shows its newest cards; the count is every
 * ticket in the stage, and a note says when the column holds more than it shows.
 */
export function TicketBoard<T extends { number: number; stage: Stage }>({
  stages,
  tickets,
  counts,
  card,
}: {
  stages: Stage[];
  tickets: T[];
  counts: StageCounts;
  card: (t: T) => ReactNode;
}): ReactElement {
  return (
    <div className="grid gap-4 md:grid-cols-2 xl:auto-cols-fr xl:grid-flow-col">
      {stages.map((s) => {
        const cards = tickets.filter((t) => t.stage === s);
        const total = counts[s];
        return (
          <section key={s} aria-labelledby={`col-${s}`} className="flex min-w-0 flex-col gap-2 rounded-xl border bg-paper p-2">
            <h2 id={`col-${s}`} className="flex items-center gap-2 px-2 py-1.5 text-[13px] font-semibold">
              <StageRing stage={s} />
              {stageLabel(s)}
              <span className="ml-auto rounded-full bg-muted px-1.5 text-xs tabular-nums" data-count>
                {total.toLocaleString()}
              </span>
            </h2>
            {cards.map((t) => (
              <div key={t.number}>{card(t)}</div>
            ))}
            {cards.length === 0 && <p className="px-2 py-3 text-[13px] text-muted-foreground">No tickets.</p>}
            {total > cards.length && (
              <p className="px-2 py-1 text-xs text-muted-foreground">
                Newest {cards.length} of {total.toLocaleString()}
              </p>
            )}
          </section>
        );
      })}
    </div>
  );
}

/** One card: the title, then the number and what else matters on the right. */
export function TicketCard({ title, number, meta, aside }: { title: string; number: number; meta?: ReactNode; aside?: ReactNode }): ReactElement {
  return (
    <div className="flex flex-col gap-2 rounded-lg border bg-card p-3 shadow-xs">
      <span className="font-medium break-words">{title}</span>
      <div className="flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
        <span className="font-mono">#{number}</span>
        {meta}
        {aside && <span className="ml-auto">{aside}</span>}
      </div>
    </div>
  );
}
