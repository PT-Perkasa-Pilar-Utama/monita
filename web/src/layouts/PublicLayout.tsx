import type { ReactElement } from "react";
import { Link, Outlet, useLocation, useParams } from "react-router-dom";
import { Mark } from "@/components/brand/Mark";
import { Bug, Globe, ListChecks, Ticket } from "@/components/icons";
import { PageFrame } from "@/components/layout/Page";
import { PageTabs } from "@/components/layout/PageTabs";
import { ThemeToggle } from "@/components/layout/ThemeToggle";
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
  const base = `/status/${p.data.slug}`;
  const shown = p.data.pages;
  const tabs: { key: keyof PublicPages; to: string; label: string; icon: typeof Globe }[] = [
    { key: "known_issues", to: `${base}/tickets`, label: "Known issues", icon: ListChecks },
    { key: "report", to: `${base}/report`, label: "Report a bug", icon: Bug },
    { key: "mine", to: `${base}/mine`, label: "My tickets", icon: Ticket },
  ];
  // A tab the project turned off is not found by URL either; its JSON answers 404 too.
  const hidden = tabs.some((t) => !shown[t.key] && pathname.startsWith(t.to));
  return (
    <div className="flex min-h-dvh flex-col">
      <header className="sticky top-0 z-40 border-b bg-background" data-has-tabs="">
        <div className="px-4 sm:px-8 lg:px-12">
          <div className="mx-auto flex h-14 max-w-app items-center gap-3">
            <Link to={base} className="flex min-w-0 items-center gap-2 text-lg font-semibold tracking-tight">
              <Mark className="size-[26px] shrink-0 text-primary" />
              <span className="truncate">{p.data.name}</span>
            </Link>
            <ThemeToggle className="ml-auto" />
          </div>
        </div>
        <PageTabs
          label="Public pages"
          tabs={[{ to: base, label: "Status", icon: Globe, end: true, match: [`${base}/history`] }, ...tabs.filter((t) => shown[t.key]).map(({ to, label, icon }) => ({ to, label, icon }))]}
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
