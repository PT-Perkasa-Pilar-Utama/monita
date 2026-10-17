import type { ReactElement } from "react";
import type { ListSummary } from "@/lib/types";
import { cn } from "@/lib/utils";

/** One sentence over a list that sums up its state: "1 of 4 monitors is down." */
export function Overall({ summary }: { summary: ListSummary }): ReactElement {
  return (
    <p
      className={cn(
        "mb-4 flex items-center gap-3 rounded-xl border px-5 py-3 text-[15px] font-medium",
        "before:w-1 before:self-stretch before:rounded-full",
        summary.kind === "up" && "border-up/30 bg-up/8 before:bg-up",
        summary.kind === "down" && "border-down/30 bg-down/8 before:bg-down",
        summary.kind === "issue" && "border-issue/30 bg-issue/8 before:bg-issue",
        summary.kind === "unknown" && "bg-card before:bg-unknown",
      )}
    >
      {summary.text}
    </p>
  );
}
