import type { ReactElement, ReactNode } from "react";
import { Button } from "@/components/ui/button";
import {
  Empty,
  EmptyContent,
  EmptyDescription,
  EmptyHeader,
  EmptyTitle,
} from "@/components/ui/empty";

// TableEmpty is what a table or list shows in place of its rows when it has
// none, the same on every page. It sits inside the list's card, under its
// filter bar, and replaces the table and its header.
//
// Two cases, never mixed: the list has nothing at all ("No runs yet."), with
// at most one action that makes the first one; or a search or filter emptied
// it, which always says so and offers Clear filters.
export function TableEmpty({
  title,
  description,
  action,
  filtered = false,
  noun,
  onClear,
}: {
  // The nothing-at-all line, such as "No runs yet."
  title: string;
  // One more sentence when the reader needs to know where rows come from.
  description?: ReactNode | undefined;
  // The one button that makes the first row. Not shown while filtered.
  action?: ReactNode | undefined;
  // A search or a filter narrowed the list to nothing.
  filtered?: boolean;
  // The rows' plural noun, for the filtered line: "No runs match these filters."
  noun?: string | undefined;
  onClear?: (() => void) | undefined;
}): ReactElement {
  return (
    <Empty className="page-in gap-4 px-6 py-12 md:py-14">
      <EmptyHeader>
        <EmptyTitle role="heading" aria-level={2} className="text-sm font-semibold tracking-normal">
          {filtered ? `No ${noun ?? "rows"} match these filters.` : title}
        </EmptyTitle>
        {!filtered && description && <EmptyDescription>{description}</EmptyDescription>}
      </EmptyHeader>
      {filtered && onClear && (
        <EmptyContent>
          <Button variant="outline" size="sm" onClick={onClear}>
            Clear filters
          </Button>
        </EmptyContent>
      )}
      {!filtered && action && <EmptyContent>{action}</EmptyContent>}
    </Empty>
  );
}
