import type { ReactElement, ReactNode } from "react";

/** Label and value pairs in two columns, such as a project's or an incident's facts. */
export function Facts({ rows }: { rows: [string, ReactNode][] }): ReactElement {
  return (
    <dl className="grid grid-cols-[max-content_minmax(0,1fr)] items-baseline gap-x-6 gap-y-2.5">
      {rows.map(([label, value]) => (
        <div key={label} className="contents">
          <dt className="text-muted-foreground">{label}</dt>
          <dd className="min-w-0 break-words">{value}</dd>
        </div>
      ))}
    </dl>
  );
}
