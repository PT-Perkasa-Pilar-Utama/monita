import { useEffect } from "react";
import type { ReactElement } from "react";
import { Link, Outlet, useParams } from "react-router-dom";
import { ProjectBadge } from "@/components/app/ProjectBadge";
import { AlertTriangle, ExternalLink, Globe, Heartbeat, LayoutGrid, Activity, ScrollText, Settings, Ticket } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { PageFrame } from "@/components/layout/Page";
import { PageTabs } from "@/components/layout/PageTabs";
import { TopBar } from "@/components/layout/TopBar";
import { NotFound } from "@/pages/NotFound";
import { ProjectProvider } from "@/lib/project";
import { forgetProject, rememberProject } from "@/lib/lastProject";
import { useMe } from "@/lib/session";
import { Skeleton } from "@/components/ui/skeleton";
import type { ProjectHead } from "@/lib/types";
import { useApi } from "@/lib/useApi";

function ProjectChrome(): ReactElement | null {
  const { slug = "" } = useParams();
  const me = useMe();
  const head = useApi<ProjectHead>(`/projects/${encodeURIComponent(slug)}`);
  const opened = head.data?.project;
  const gone = head.error?.status === 404;
  useEffect(() => {
    if (opened) rememberProject(me.email, opened);
    else if (gone) forgetProject(me.email, slug);
  }, [opened, gone, me.email, slug]);
  if (gone) return <NotFound />;
  // The header stays up while the project loads, so moving between areas never blinks it away.
  if (!head.data) {
    return (
      <>
        <TopBar
          crumb={<Skeleton className="h-6 w-40" />}
          tabs={
            <div className="px-4 sm:px-8 lg:px-12">
              <div className="mx-auto flex h-11 max-w-app items-center gap-6">
                {Array.from({ length: 6 }, (_, i) => (
                  <Skeleton key={i} className="h-4 w-20" />
                ))}
              </div>
            </div>
          }
        />
        <PageFrame>
          <Skeleton className="h-96 rounded-xl" />
        </PageFrame>
      </>
    );
  }
  const { project: p, counts } = head.data;
  const base = `/projects/${p.slug}`;
  const crumb = (
    <Link to={base} className="flex min-w-0 items-center gap-2 rounded-lg px-2 py-1 font-semibold hover:bg-accent">
      <ProjectBadge name={p.name} iconUrl={p.icon_url} />
      <span className="truncate">{p.name}</span>
    </Link>
  );
  const tabs = (
    <PageTabs
      label="Project"
      tabs={[
        { to: base, label: "Overview", icon: LayoutGrid, end: true },
        // A number on a tab flags something to look at: what is down, what is open, what waits.
        { to: `${base}/monitors`, label: "Monitors", icon: Activity, count: counts.monitors_down, alarm: true },
        { to: `${base}/heartbeats`, label: "Heartbeats", icon: Heartbeat, count: counts.heartbeats_down, alarm: true },
        { to: `${base}/incidents`, label: "Incidents", icon: AlertTriangle, count: counts.incidents, alarm: true },
        { to: `${base}/tickets`, label: "Tickets", icon: Ticket, count: counts.triage },
        { to: `${base}/changelog`, label: "Changelog", icon: ScrollText },
        { to: `${base}/settings`, label: "Settings", icon: Settings },
      ]}
      end={
        <Button asChild variant="outline" size="sm">
          <a href={`/status/${p.slug}`} target="_blank" rel="noreferrer" aria-label="Status page, opens in a new tab">
            <Globe data-icon="inline-start" />
            <span className="max-sm:hidden">Status page</span>
            <ExternalLink data-icon="inline-end" className="max-sm:hidden" />
          </a>
        </Button>
      }
    />
  );
  return (
    <ProjectProvider value={{ ...head.data, reload: head.reload }}>
      <TopBar crumb={crumb} tabs={tabs} />
      <PageFrame>
        <Outlet />
      </PageFrame>
    </ProjectProvider>
  );
}

/** Every page inside a project: the top bar with the project, its tab bar, then the page. */
export function ProjectLayout(): ReactElement {
  return <ProjectChrome />;
}
