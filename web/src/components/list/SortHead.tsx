import type { ReactElement } from "react";
import { ArrowDown, ArrowUp, ArrowUpDown } from "@/components/icons";
import { TableHead } from "@/components/ui/table";
import type { Sort } from "@/lib/sort";
import { cn } from "@/lib/utils";

/** A column heading that sorts the table: a button, so the keyboard reaches it, and an arrow for the order. */
export function SortHead({ by, label, sort, onSort, className }: { by: string; label: string; sort: Sort; onSort: (by: string) => void; className?: string }): ReactElement {
  const active = sort?.by === by;
  const Icon = !active ? ArrowUpDown : sort.dir === "asc" ? ArrowUp : ArrowDown;
  return (
    <TableHead aria-sort={active ? (sort.dir === "asc" ? "ascending" : "descending") : undefined} className={className}>
      <button type="button" onClick={() => onSort(by)} className={cn("inline-flex items-center gap-1 rounded-sm outline-none hover:text-foreground focus-visible:ring-[3px] focus-visible:ring-ring", active && "text-foreground")}>
        {label}
        <Icon className={cn("size-3.5", !active && "opacity-50")} />
      </button>
    </TableHead>
  );
}
