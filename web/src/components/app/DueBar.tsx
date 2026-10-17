import type { ReactElement } from "react";
import type { Due } from "@/lib/types";
import { cn } from "@/lib/utils";

/** How much of the wait before a heartbeat's deadline has passed. */
export function DueBar({ due }: { due: Due }): ReactElement {
  return (
    <div className="flex w-40 flex-col gap-1">
      <svg viewBox="0 0 100 6" preserveAspectRatio="none" aria-hidden="true" className="h-1.5 w-full">
        <rect width={100} height={6} rx={3} className="fill-muted" />
        <rect width={due.used} height={6} rx={3} className={due.late ? "fill-down" : "fill-up"} />
      </svg>
      <span className={cn("text-xs", due.late ? "font-medium text-down" : "text-muted-foreground")}>{due.text}</span>
    </div>
  );
}
