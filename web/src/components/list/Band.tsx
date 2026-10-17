import type { ReactElement, ReactNode } from "react";

// Band is the paper strip that opens a section of a card and names its count,
// as the Storage table names a day. Anything on the right sits at its end.
export function Band({
  icon,
  label,
  count,
  right,
}: {
  // A status mark before the label, such as a section's worst finding.
  icon?: ReactNode | undefined;
  label: string;
  count?: number | undefined;
  right?: ReactNode | undefined;
}): ReactElement {
  return (
    <div className="flex min-h-9 flex-wrap items-center justify-between gap-x-3 gap-y-1 border-b bg-paper px-4 py-1.5 text-xs">
      <span className="flex items-center gap-2 font-semibold">
        {icon}
        {label}
        {count !== undefined && (
          <span className="font-normal text-muted-foreground tabular-nums">{count}</span>
        )}
      </span>
      {/* A right side that wraps stays at the end, under the label. */}
      {right && <div className="ml-auto">{right}</div>}
    </div>
  );
}
