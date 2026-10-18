import type { ReactElement } from "react";
import { ChevronLeft, ChevronRight } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { pageCount, rangeLabel } from "@/lib/page";

// Pager sits under a list the server reads one page at a time: which rows
// show, and Previous and Next. It hides when every row fits on one page.
export function Pager({
  page,
  pageSize,
  total,
  noun,
  onPage,
}: {
  page: number;
  pageSize: number;
  total: number;
  // The rows' singular noun: "project".
  noun: string;
  onPage: (page: number) => void;
}): ReactElement | null {
  if (total <= pageSize) return null;
  const pages = pageCount(total, pageSize);
  return (
    <nav aria-label={`Pages of ${noun}s`} className="flex flex-wrap items-center justify-between gap-3 text-sm">
      <span className="text-muted-foreground">{rangeLabel(page, pageSize, total, noun)}</span>
      <div className="flex items-center gap-2">
        <Button variant="outline" size="sm" disabled={page <= 1} onClick={() => onPage(page - 1)}>
          <ChevronLeft data-icon="inline-start" />
          Previous
        </Button>
        <span className="text-muted-foreground tabular-nums">
          Page {page} of {pages}
        </span>
        <Button variant="outline" size="sm" disabled={page >= pages} onClick={() => onPage(page + 1)}>
          Next
          <ChevronRight data-icon="inline-end" />
        </Button>
      </div>
    </nav>
  );
}
