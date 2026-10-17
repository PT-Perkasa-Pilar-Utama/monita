import type { ReactElement, ReactNode } from "react";
import { Card } from "@/components/ui/card";
import { cn } from "@/lib/utils";

/**
 * A table in a card, opened by a band: the count or title on the left, then search and filters,
 * or an action, on the right. The band is the place for table-level actions.
 */
export function TableCard({
  title,
  tools,
  children,
  className,
}: {
  title: ReactNode;
  tools?: ReactNode;
  children: ReactNode;
  className?: string;
}): ReactElement {
  return (
    <Card className={cn("gap-0 overflow-hidden py-0", className)}>
      <div className="flex min-h-[52px] flex-wrap items-center justify-between gap-x-3 gap-y-2 border-b bg-paper px-5 py-2">
        <div className="text-[13px] font-semibold">{title}</div>
        {tools && <div className="flex flex-wrap items-center gap-2 max-sm:w-full">{tools}</div>}
      </div>
      {children}
    </Card>
  );
}
