import type { ReactElement } from "react";
import { Link, Outlet, useLocation, useParams } from "react-router-dom";
import { Mark } from "@/components/brand/Mark";
import { Activity, AlertTriangle, Bug, ExternalLink, ListChecks, ScrollText } from "@/components/icons";
import { PageFrame } from "@/components/layout/Page";
import { PageTabs } from "@/components/layout/PageTabs";
import { ThemeToggle } from "@/components/layout/ThemeToggle";
import { setOffset } from "@/lib/time";
import type { PublicPages, PublicSite } from "@/lib/types";
import { useApi } from "@/lib/useApi";
import { NotFound } from "@/pages/NotFound";

/** A project's public pages: no sign-in, the project's name, its public tabs and a footer. */
export function PublicLayout(): ReactElement | null {
  const { slug = "" } = useParams();
  const { pathname } = useLocation();
  const p = useApi<PublicSite>(`/public/${encodeURIComponent(slug)}`);
  if (p.error?.status === 404) return <NotFound />;
  if (!p.data) return null;
  setOffset(p.data.utc_offset_min);
  const base = `/status/${p.data.slug}`;
  const shown = p.data.pages;
  const tabs: { key: keyof PublicPages; to: string; label: string; icon: typeof Activity; match?: string[] }[] = [
    { key: "incidents", to: `${base}/incidents`, label: "Incidents", icon: AlertTriangle, match: [`${base}/history`] },
    { key: "known_issues", to: `${base}/issues`, label: "Issues", icon: ListChecks },
    { key: "report", to: `${base}/ticket`, label: "Ticket", icon: Bug },
    { key: "changelog", to: `${base}/changelog`, label: "Changelog", icon: ScrollText },
  ];
  // A tab the project turned off is not found by URL either; its JSON answers 404 too.
  const hidden = tabs.some((t) => !shown[t.key] && [t.to, ...(t.match ?? [])].some((to) => pathname.startsWith(to)));
  return (
    <div className="flex min-h-dvh flex-col">
      <header className="sticky top-0 z-40 border-b bg-background" data-has-tabs="">
        <div className="px-4 sm:px-8 lg:px-12">
          <div className="mx-auto flex h-14 max-w-app items-center gap-3">
            <Link to={base} className="flex min-w-0 items-center gap-2 text-lg font-semibold tracking-tight">
              <Mark className="size-[26px] shrink-0 text-primary" />
              <span className="truncate">{p.data.name}</span>
            </Link>
            {p.data.homepage_url && (
              <a
                href={p.data.homepage_url}
                rel="noreferrer"
                className="ml-auto flex min-w-0 items-center gap-1.5 text-sm text-muted-foreground underline-offset-4 hover:text-foreground hover:underline"
              >
                <span className="truncate">{p.data.homepage_url.replace(/^https?:\/\//, "").replace(/\/$/, "")}</span>
                <ExternalLink className="size-3.5 shrink-0" />
              </a>
            )}
            <ThemeToggle className={p.data.homepage_url ? "" : "ml-auto"} />
          </div>
        </div>
        <PageTabs
          label="Public pages"
          tabs={[{ to: base, label: "Uptime", icon: Activity, end: true, match: [`${base}/uptime`] }, ...tabs.filter((t) => shown[t.key]).map(({ to, label, icon, match }) => ({ to, label, icon, ...(match ? { match } : {}) }))]}
        />
      </header>
      <PageFrame>
        {hidden ? <NotFound /> : <Outlet context={p.data} />}
      </PageFrame>
      <footer className="mt-auto px-4 py-8 text-center text-sm text-muted-foreground">
        Powered by{" "}
        <a href="https://github.com/PT-Perkasa-Pilar-Utama/monita" className="underline-offset-4 hover:text-foreground hover:underline">
          Monita
        </a>
      </footer>
    </div>
  );
}
