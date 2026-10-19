import { useState } from "react";
import type { DragEvent, ReactElement, ReactNode } from "react";
import { Button } from "@/components/ui/button";
import { Spinner } from "@/components/ui/spinner";
import { stageLabel } from "@/lib/tickets";
import type { Stage, StageCounts } from "@/lib/types";
import { cn } from "@/lib/utils";

/** A ring per column: dashed grey for Triage, empty for Backlog, half yellow for In progress, full green for Done. */
export function StageRing({ stage }: { stage: Stage }): ReactElement {
  return (
    <span
      aria-hidden="true"
      className={cn(
        "inline-block size-3 shrink-0 rounded-full border-[1.5px]",
        stage === "triage" && "border-dashed border-unknown",
        stage === "backlog" && "border-neutral-strong",
        stage === "in_progress" && "border-issue bg-[conic-gradient(var(--issue)_0_50%,transparent_50%_100%)]",
        stage === "done" && "border-up bg-up",
      )}
    />
  );
}

// The count pill takes its column's colour.
const COUNT: Record<Stage, string> = {
  triage: "bg-muted text-muted-foreground",
  backlog: "bg-neutral-strong/12 text-neutral-strong",
  in_progress: "bg-issue/14 text-issue",
  done: "bg-up/12 text-up",
};

/**
 * Columns of ticket cards, one per stage. A column shows a page of its newest cards; the count is
 * every ticket in the stage, and `footer` offers the rest. With `onMove` a card drags to another
 * column; the drop names the whole column, not a place in it.
 */
export function TicketBoard<T extends { number: number; stage: Stage }>({
  stages,
  tickets,
  counts,
  card,
  notes,
  footer,
  onMove,
}: {
  stages: Stage[];
  tickets: T[];
  counts: Partial<StageCounts>;
  card: (t: T) => ReactNode;
  notes?: Partial<Record<Stage, string>>;
  // Under the cards when the column holds more than it shows.
  footer?: (stage: Stage, shown: number, total: number) => ReactNode;
  onMove?: ((t: T, to: Stage) => void) | undefined;
}): ReactElement {
  // dataTransfer is unreadable during dragover, so the dragged card lives here.
  const [dragging, setDragging] = useState<T | null>(null);
  const [over, setOver] = useState<Stage | null>(null);
  const end = (): void => {
    setDragging(null);
    setOver(null);
  };
  const target = (s: Stage) =>
    onMove && {
      onDragOver: (e: DragEvent) => {
        if (!dragging) return;
        e.preventDefault();
        e.dataTransfer.dropEffect = "move";
        if (over !== s) setOver(s);
      },
      onDragLeave: (e: DragEvent) => {
        if (!e.currentTarget.contains(e.relatedTarget as Node | null)) setOver(null);
      },
      onDrop: (e: DragEvent) => {
        e.preventDefault();
        if (dragging) onMove(dragging, s);
        end();
      },
    };
  return (
    <div className="grid auto-cols-[minmax(16.5rem,1fr)] grid-flow-col gap-3 overflow-x-auto pb-2">
      {stages.map((s) => {
        const cards = tickets.filter((t) => t.stage === s);
        const total = counts[s] ?? 0;
        return (
          <section
            key={s}
            aria-labelledby={`col-${s}`}
            className={cn(
              "flex h-[calc(100dvh-15rem)] min-h-[26rem] min-w-0 flex-col rounded-xl border bg-paper transition-colors",
              over === s && dragging?.stage !== s && "border-foreground/30 bg-accent",
            )}
            {...target(s)}
          >
            <div className="flex flex-col gap-1 px-4 pt-3.5 pb-3">
              <h2 id={`col-${s}`} className="flex items-center gap-2 text-sm font-semibold">
                <StageRing stage={s} />
                {stageLabel(s)}
                <span className={cn("rounded-full px-1.5 text-xs font-semibold tabular-nums", COUNT[s])} data-count>
                  {total.toLocaleString()}
                </span>
              </h2>
              {notes?.[s] && (
                <p className="truncate text-[13px] text-muted-foreground" title={notes[s]}>
                  {notes[s]}
                </p>
              )}
            </div>
            <div className="flex min-h-0 flex-1 flex-col gap-2 overflow-y-auto px-2 pb-2">
              {over === s && dragging && dragging.stage !== s && (
                <div aria-hidden="true" className="h-16 shrink-0 rounded-lg border-2 border-dashed border-foreground/25 bg-background/60" />
              )}
              {cards.map((t) => (
                <div
                  key={t.number}
                  draggable={onMove ? true : undefined}
                  onDragStart={
                    onMove &&
                    ((e) => {
                      // Safari starts a drag only once something is set.
                      e.dataTransfer.setData("text/plain", `#${t.number}`);
                      e.dataTransfer.effectAllowed = "move";
                      // The card itself follows the pointer, held where it was grabbed, not a link preview.
                      const box = e.currentTarget.getBoundingClientRect();
                      e.dataTransfer.setDragImage(e.currentTarget, e.clientX - box.left, e.clientY - box.top);
                      setDragging(t);
                    })
                  }
                  onDragEnd={onMove && end}
                  className={cn("shrink-0", dragging?.number === t.number && "opacity-50")}
                >
                  {card(t)}
                </div>
              ))}
              {cards.length === 0 && <p className="px-2 py-3 text-[13px] text-muted-foreground">No tickets.</p>}
              {total > cards.length &&
                (footer?.(s, cards.length, total) ?? (
                  <p className="px-2 py-1 text-xs text-muted-foreground">
                    Newest {cards.length} of {total.toLocaleString()}
                  </p>
                ))}
            </div>
          </section>
        );
      })}
    </div>
  );
}

/** One card: the number and what else matters, then the title, then its badges. */
export function TicketCard({ title, number, meta, aside }: { title: string; number: number; meta?: ReactNode; aside?: ReactNode }): ReactElement {
  return (
    <div className="flex flex-col gap-1.5 rounded-lg border bg-card p-3 shadow-xs transition-colors">
      <div className="flex items-center gap-2 text-xs text-muted-foreground">
        <span className="font-mono">#{number}</span>
        {aside && <span className="ml-auto">{aside}</span>}
      </div>
      <span className="text-sm font-medium break-words">{title}</span>
      {meta && <div className="flex flex-wrap items-center gap-2 text-xs text-muted-foreground empty:hidden">{meta}</div>}
    </div>
  );
}

/** The foot of a column that holds more than it shows. */
export function ShowMore({ busy, onClick }: { busy: boolean; onClick: () => void }): ReactElement {
  return (
    <Button type="button" variant="ghost" size="sm" className="w-full text-muted-foreground" disabled={busy} onClick={onClick}>
      {busy && <Spinner data-icon="inline-start" />}
      Show more
    </Button>
  );
}
