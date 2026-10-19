import type { ReactElement } from "react";
import { ChevronLeft, ChevronRight } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { windowLabel } from "@/lib/status";

/** "‹ August 2026 to October 2026 ›": moves a three-month window. An arrow with nowhere to go is off. */
export function WindowNav({ months, prev, next, onMove }: { months: string[]; prev: number | null; next: number | null; onMove: (to: number) => void }): ReactElement {
  return (
    <nav aria-label="Months" className="flex items-center gap-3">
      <Button variant="outline" size="icon" aria-label="Earlier months" disabled={prev === null} onClick={() => prev !== null && onMove(prev)}>
        <ChevronLeft />
      </Button>
      <span className="min-w-0 text-sm font-medium tabular-nums">{windowLabel(months)}</span>
      <Button variant="outline" size="icon" aria-label="Later months" disabled={next === null} onClick={() => next !== null && onMove(next)}>
        <ChevronRight />
      </Button>
    </nav>
  );
}
