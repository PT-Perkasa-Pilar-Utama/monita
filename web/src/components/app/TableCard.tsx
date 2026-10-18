import type { ReactElement, ReactNode } from "react";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { cn } from "@/lib/utils";

/** A list's count, with Clear filters beside it while a search or filter narrows the list. */
export function FilterCount({ children, onClear }: { children: ReactNode; onClear?: (() => void) | undefined }): ReactElement {
  return (
    <span className="flex flex-wrap items-center gap-x-4 gap-y-1">
      <span>{children}</span>
      {onClear && (
        <Button variant="link" size="sm" className="h-auto p-0" onClick={onClear}>
          Clear filters
        </Button>
      )}
    </span>
  );
}

/**
 * A table in a card, opened by a band: the count or title on the left, then search and filters,
 * or an action, on the right. The band is the place for table-level actions.
 * Pass `onClear` only while the list is filtered.
 */
export function TableCard({
  title,
  tools,
  onClear,
  children,
  className,
}: {
  title: ReactNode;
  tools?: ReactNode;
  onClear?: (() => void) | undefined;
  children: ReactNode;
  className?: string;
}): ReactElement {
  return (
    <Card className={cn("gap-0 overflow-hidden py-0", className)}>
      <div className="flex min-h-[52px] flex-wrap items-center justify-between gap-x-3 gap-y-2 border-b bg-paper px-5 py-2">
        <div className="text-[13px] font-semibold">
          <FilterCount onClear={onClear}>{title}</FilterCount>
        </div>
        {tools && <div className="flex flex-wrap items-center gap-2 max-sm:w-full">{tools}</div>}
      </div>
      {children}
    </Card>
  );
}
