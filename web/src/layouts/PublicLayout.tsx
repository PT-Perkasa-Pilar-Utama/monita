import type { ReactElement } from "react";
import { Outlet, useParams } from "react-router-dom";
import { Mark } from "@/components/brand/Mark";
import { Bug, Globe, ListChecks, Ticket } from "@/components/icons";
import { PageFrame } from "@/components/layout/Page";
import { PageTabs } from "@/components/layout/PageTabs";
import { ThemeToggle } from "@/components/layout/ThemeToggle";
import type { PublicProject } from "@/lib/types";
import { useApi } from "@/lib/useApi";
import { NotFound } from "@/pages/NotFound";

/** A project's public pages: no sign-in, the project's name, its public tabs and a footer. */
export function PublicLayout(): ReactElement | null {
  const { slug = "" } = useParams();
  const p = useApi<PublicProject>(`/public/${encodeURIComponent(slug)}`);
  if (p.error?.status === 404) return <NotFound />;
  if (!p.data) return null;
  const base = `/status/${p.data.slug}`;
  return (
    <div className="flex min-h-dvh flex-col">
      <header className="sticky top-0 z-40 border-b bg-background" data-has-tabs="">
        <div className="px-4 sm:px-8 lg:px-12">
          <div className="mx-auto flex h-14 max-w-app items-center gap-3">
            {/* ponytail: the status page is a server page until R5. */}
            <a href={base} className="flex min-w-0 items-center gap-2 text-lg font-semibold tracking-tight">
              <Mark className="size-[26px] shrink-0 text-primary" />
              <span className="truncate">{p.data.name}</span>
            </a>
            <ThemeToggle className="ml-auto" />
          </div>
        </div>
        <PageTabs
          label="Public pages"
          tabs={[
            { to: base, label: "Status", icon: Globe, end: true, server: true },
            { to: `${base}/tickets`, label: "Known issues", icon: ListChecks },
            { to: `${base}/report`, label: "Report a bug", icon: Bug },
            { to: `${base}/mine`, label: "My tickets", icon: Ticket },
          ]}
        />
      </header>
      <PageFrame>
        <Outlet context={p.data} />
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
