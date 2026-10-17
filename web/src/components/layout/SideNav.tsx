import type { ReactElement, ReactNode } from "react";
import { Link } from "react-router-dom";
import type { AppIcon } from "@/components/icons";
import { cn } from "@/lib/utils";

export type SideNavItem = {
  to: string;
  label: string;
  icon: AppIcon;
  active: boolean;
  count?: number | undefined;
};

export type SideNavSection = { label?: string; items: SideNavItem[] };

/**
 * A page's left sidebar, only for areas with sub-pages (Settings, Admin). The open page is a
 * raised card with a brand bar on its left. On a phone it is one row that scrolls sideways.
 */
export function SideNav({ label, sections }: { label: string; sections: SideNavSection[] }): ReactElement {
  return (
    <nav aria-label={label} className="shrink-0 md:w-56">
      <div className="flex gap-1 overflow-x-auto md:flex-col md:gap-5 md:overflow-visible">
        {sections.map((s, n) => (
          <div key={s.label ?? n} className="flex shrink-0 gap-1 md:flex-col">
            {s.label && <h2 className="hidden px-3 pb-1 text-xs font-semibold text-muted-foreground md:block">{s.label}</h2>}
            {s.items.map((i) => (
              <Link
                key={i.to}
                to={i.to}
                aria-current={i.active ? "page" : undefined}
                className={cn(
                  "relative flex h-9 items-center gap-3 whitespace-nowrap rounded-lg px-3 text-sm",
                  i.active
                    ? "bg-card font-semibold text-foreground shadow-xs ring-1 ring-border md:before:absolute md:before:inset-y-2 md:before:left-0 md:before:w-0.5 md:before:rounded-full md:before:bg-primary"
                    : "text-muted-foreground hover:bg-muted hover:text-foreground",
                )}
              >
                <i.icon className="size-4 shrink-0" />
                {i.label}
                {i.count !== undefined && i.count > 0 && (
                  <span className="ml-auto rounded-full bg-muted px-1.5 text-xs font-semibold tabular-nums text-foreground">
                    {i.count.toLocaleString()}
                  </span>
                )}
              </Link>
            ))}
          </div>
        ))}
      </div>
    </nav>
  );
}

/** A page with a SideNav: the sidebar on the left, sticky, and the open page on the right. */
export function SidebarFrame({ nav, children }: { nav: ReactNode; children: ReactNode }): ReactElement {
  return (
    <div className="flex flex-col gap-4 md:flex-row md:gap-8">
      <div className="shrink-0 md:sticky md:top-[calc(var(--bar-h)+24px)] md:self-start">{nav}</div>
      <div className="min-w-0 flex-1">{children}</div>
    </div>
  );
}
