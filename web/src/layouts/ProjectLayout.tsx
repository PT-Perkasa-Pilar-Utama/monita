import type { ReactElement } from "react";
import { Link, Outlet, useParams } from "react-router-dom";
import { ProjectBadge } from "@/components/app/ProjectBadge";
import { AlertTriangle, Globe, Heartbeat, LayoutGrid, Activity, Settings, Ticket } from "@/components/icons";
import { PageFrame } from "@/components/layout/Page";
import { PageTabs } from "@/components/layout/PageTabs";
import { TopBar } from "@/components/layout/TopBar";
import { Button } from "@/components/ui/button";
import { NotFound } from "@/pages/NotFound";
import { ProjectProvider } from "@/lib/project";
import { SessionProvider } from "@/lib/session";
import type { ProjectHead } from "@/lib/types";
import { useApi } from "@/lib/useApi";

function ProjectChrome(): ReactElement | null {
  const { slug = "" } = useParams();
  const head = useApi<ProjectHead>(`/projects/${encodeURIComponent(slug)}`);
  if (head.error?.status === 404) return <NotFound />;
  if (!head.data) return null;
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
        { to: `${base}/monitors`, label: "Monitors", icon: Activity, count: counts.monitors, server: true },
        { to: `${base}/heartbeats`, label: "Heartbeats", icon: Heartbeat, count: counts.heartbeats, server: true },
        { to: `${base}/incidents`, label: "Incidents", icon: AlertTriangle, count: counts.incidents, server: true },
        { to: `${base}/tickets`, label: "Tickets", icon: Ticket, count: counts.tickets, server: true },
        { to: `${base}/settings`, label: "Settings", icon: Settings, match: [`${base}/alerts`] },
      ]}
      end={
        <Button asChild variant="ghost" size="sm" className="rounded-lg">
          <a href={`/status/${p.slug}`}>
            <Globe data-icon="inline-start" />
            Status page
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
  return (
    <SessionProvider>
      <ProjectChrome />
    </SessionProvider>
  );
}
