import { useEffect, useRef } from "react";
import type * as React from "react";
import type { ReactElement } from "react";
import { Link, useLocation } from "react-router-dom";
import type { AppIcon } from "@/components/icons";
import { cn } from "@/lib/utils";

export type PageTab = {
  /** The tab's full path; the URL is the open tab. */
  to: string;
  label: string;
  icon: AppIcon;
  /** Shown beside the label when above zero. */
  count?: number | undefined;
  /** Open only on its exact path, such as an overview whose path starts every other tab's. */
  end?: boolean;
  /** Other paths that open this tab, such as the pages in its sidebar. */
  match?: readonly string[] | undefined;
  // ponytail: a server page not yet moved to React loads in full; drop once every tab is React.
  server?: boolean;
};

function TabLink({ t, ...props }: { t: PageTab } & Omit<React.ComponentProps<"a">, "href">): ReactElement {
  return t.server ? <a href={t.to} {...props} /> : <Link to={t.to} {...props} />;
}

function opens(t: PageTab, pathname: string): boolean {
  return [t.to, ...(t.match ?? [])].some((p) => pathname === p || (!t.end && pathname.startsWith(`${p}/`)));
}

/**
 * An area's tab bar under the top bar. The open tab carries the brand underline, inset from its
 * edges. The bar scrolls sideways on a narrow screen. `end` holds links that leave the area.
 */
export function PageTabs({
  label,
  tabs,
  end,
}: {
  label: string;
  tabs: PageTab[];
  end?: ReactElement | undefined;
}): ReactElement {
  const pathname = useLocation().pathname;
  const bar = useRef<HTMLUListElement>(null);
  const open = tabs.find((t) => opens(t, pathname))?.to;
  const layout = tabs.map((t) => `${t.to}:${t.count ?? ""}`).join();
  useEffect(() => {
    bar.current?.querySelector('[aria-current="page"]')?.scrollIntoView({ block: "nearest", inline: "nearest" });
  }, [open, layout]);
  return (
    <nav aria-label={label} className="px-4 sm:px-8 lg:px-12">
      <div className="mx-auto flex max-w-app items-center">
        {/* Pulled out by a tab's own padding, so the first icon lines up with the logo. */}
        <ul ref={bar} className="-mx-3 flex min-w-0 flex-1 overflow-x-auto [scrollbar-width:none]">
          {tabs.map((t) => (
            <li key={t.to} className="shrink-0">
              <TabLink
                t={t}
                aria-current={open === t.to ? "page" : undefined}
                className={cn(
                  "relative flex h-11 items-center gap-2 px-3 text-sm transition-colors",
                  "after:absolute after:inset-x-2 after:bottom-0 after:h-0.5 after:rounded-full",
                  open === t.to
                    ? "font-semibold text-foreground after:bg-primary"
                    : "text-muted-foreground hover:text-foreground hover:after:bg-input",
                )}
              >
                <t.icon className="size-4 shrink-0" />
                {t.label}
                {t.count !== undefined && t.count > 0 && (
                  <span className="min-w-[18px] rounded-full bg-muted px-1.5 text-center text-xs font-semibold tabular-nums text-foreground">
                    {t.count.toLocaleString()}
                  </span>
                )}
              </TabLink>
            </li>
          ))}
        </ul>
        {end && <div className="ml-auto hidden shrink-0 sm:block">{end}</div>}
      </div>
    </nav>
  );
}
