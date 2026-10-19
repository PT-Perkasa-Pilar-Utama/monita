import { useEffect, useRef } from "react";
import type { ReactElement } from "react";
import { Link, useLocation } from "react-router-dom";
import type { AppIcon } from "@/components/icons";
import { cn } from "@/lib/utils";

export type PageTab = {
  /** The tab's full path; the URL is the open tab. */
  to: string;
  label: string;
  icon: AppIcon;
  /** Shown beside the label when above zero. A count flags something to look at, never a total. */
  count?: number | undefined;
  /** The count means something is broken (down, open incident): it shows in red. */
  alarm?: boolean;
  /** Open only on its exact path, such as an overview whose path starts every other tab's. */
  end?: boolean;
  /** Other paths that open this tab, such as the pages in its sidebar. */
  match?: readonly string[] | undefined;
};

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
  center = false,
}: {
  label: string;
  tabs: PageTab[];
  end?: ReactElement | undefined;
  /** The tabs sit in the middle of the bar, as on the public pages. */
  center?: boolean;
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
            // Auto margins on the end tabs centre the row and still let a narrow screen scroll from its start.
            <li key={t.to} className={cn("shrink-0", center && "first:ms-auto last:me-auto")}>
              <Link
                to={t.to}
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
                  <span className={cn("min-w-[18px] rounded-full px-1.5 text-center text-xs font-semibold tabular-nums", t.alarm ? "bg-down/12 text-down" : "bg-muted text-foreground")}>
                    {t.count.toLocaleString()}
                  </span>
                )}
              </Link>
            </li>
          ))}
        </ul>
        {end && <div className="ml-auto shrink-0 pl-3">{end}</div>}
      </div>
    </nav>
  );
}
