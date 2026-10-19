import type { MouseEvent, ReactElement, ReactNode } from "react";
import { useNavigate } from "react-router-dom";
import { TableRow } from "@/components/ui/table";
import { cn } from "@/lib/utils";

// A control inside the row keeps its own click.
const OWN_CLICK = "a, button, input, select, textarea, label, [role=checkbox], [role=menuitem]";

/**
 * A table row that opens `to` from a click anywhere on it. The row's first cell still holds a
 * real link, for the keyboard and for a screen reader; Cmd or Ctrl opens a new tab.
 */
export function LinkRow({ to, state, className, children }: { to: string; state?: unknown; className?: string; children: ReactNode }): ReactElement {
  const navigate = useNavigate();
  const open = (e: MouseEvent<HTMLTableRowElement>): void => {
    if (e.target instanceof Element && e.target.closest(OWN_CLICK)) return;
    if (e.metaKey || e.ctrlKey) window.open(to, "_blank", "noopener");
    else navigate(to, { state });
  };
  return (
    <TableRow className={cn("cursor-pointer hover:bg-accent", className)} onClick={open}>
      {children}
    </TableRow>
  );
}

/** The row's own link: plain text that the row's hover already marks. */
export const ROW_LINK = "font-medium outline-none focus-visible:underline";
