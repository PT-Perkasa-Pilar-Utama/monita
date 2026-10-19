import { useEffect } from "react";
import type { ReactElement } from "react";
import { Link, Outlet, useParams } from "react-router-dom";
import { ProjectBadge } from "@/components/app/ProjectBadge";
import { AlertTriangle, Heartbeat, LayoutGrid, Activity, Settings, Ticket } from "@/components/icons";
import { PageFrame } from "@/components/layout/Page";
import { PageTabs } from "@/components/layout/PageTabs";
import { TopBar } from "@/components/layout/TopBar";
import { NotFound } from "@/pages/NotFound";
import { ProjectProvider } from "@/lib/project";
import { forgetProject, rememberProject } from "@/lib/lastProject";
import { SessionProvider, useMe } from "@/lib/session";
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
        { to: `${base}/monitors`, label: "Monitors", icon: Activity, count: counts.monitors },
        { to: `${base}/heartbeats`, label: "Heartbeats", icon: Heartbeat, count: counts.heartbeats },
        { to: `${base}/incidents`, label: "Incidents", icon: AlertTriangle, count: counts.incidents },
        { to: `${base}/tickets`, label: "Tickets", icon: Ticket, count: counts.tickets },
        { to: `${base}/settings`, label: "Settings", icon: Settings },
      ]}
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
