import { Badge } from "@/components/ui/badge";
import type { ReactElement, ReactNode } from "react";
import { Link } from "react-router-dom";
import { cn } from "@/lib/utils";

/** The page frame under the top bar: the content column, as wide as the docs and landing. */
export function PageFrame({ children }: { children: ReactNode }): ReactElement {
  return (
    <main className="px-4 py-8 sm:px-8 lg:px-12">
      <div className="mx-auto max-w-app">{children}</div>
    </main>
  );
}

export type Crumb = { label: string; to?: string };

/** Detail and form pages only: a list page's tab already says where you are. */
export function Crumbs({ items }: { items: Crumb[] }): ReactElement {
  return (
    <nav aria-label="Breadcrumb" className="mb-3 flex min-w-0 items-center gap-2 text-sm text-muted-foreground">
      {items.map((c, i) => (
        <span key={c.label + i} className="flex min-w-0 items-center gap-2">
          {i > 0 && <span aria-hidden="true">/</span>}
          {c.to ? (
            <Link to={c.to} className="hover:text-foreground">
              {c.label}
            </Link>
          ) : (
            <span className="truncate font-medium text-foreground">{c.label}</span>
          )}
        </span>
      ))}
    </nav>
  );
}

/** One row: a leading mark, the title and its meta line, then the page's actions at the right. */
export function PageHeader({
  title,
  filtered = false,
  lead,
  meta,
  actions,
  className,
}: {
  title: ReactNode;
  // A search or filter narrows the page's list.
  filtered?: boolean;
  lead?: ReactNode;
  meta?: ReactNode;
  actions?: ReactNode;
  className?: string;
}): ReactElement {
  return (
    <div className={cn("mb-6 flex flex-wrap items-start justify-between gap-3", className)}>
      <div className="flex min-w-0 items-center gap-3">
        {lead}
        <div className="flex min-w-0 flex-col gap-2">
          <h1 className="flex flex-wrap items-center gap-3 text-2xl font-bold tracking-tight break-words sm:text-[32px] sm:leading-tight">
            {title}
            {filtered && (
              <Badge variant="neutral" className="px-2.5 py-1 text-sm tracking-normal">
                Filtered
              </Badge>
            )}
          </h1>
          {meta && <div className="flex flex-wrap items-center gap-x-4 gap-y-1 text-muted-foreground">{meta}</div>}
        </div>
      </div>
      {actions && <div className="ml-auto flex flex-wrap items-center gap-2">{actions}</div>}
    </div>
  );
}
