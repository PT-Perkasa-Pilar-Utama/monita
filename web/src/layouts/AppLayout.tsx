import type { ReactElement } from "react";
import { Outlet } from "react-router-dom";
import { AlertTriangle, BookOpen, ExternalLink, Folder, Settings, Ticket } from "@/components/icons";
import { PageFrame } from "@/components/layout/Page";
import { PageTabs } from "@/components/layout/PageTabs";
import { DOCS, TopBar } from "@/components/layout/TopBar";
import { Button } from "@/components/ui/button";
import { SessionProvider, useMe } from "@/lib/session";
import type { HomeCounts } from "@/lib/types";
import { LIVE_MS, useApi } from "@/lib/useApi";

/** The menu above the projects: Projects, Incidents and Tickets across them, Admin settings for a
 * super admin, and the documentation. */
function HomeTabs(): ReactElement {
  const me = useMe();
  const counts = useApi<HomeCounts>("/home-counts", LIVE_MS).data;
  return (
    <PageTabs
      label="Monita"
      tabs={[
        { to: "/", label: "Projects", icon: Folder, end: true, count: counts?.projects },
        { to: "/incidents", label: "Incidents", icon: AlertTriangle, count: counts?.incidents },
        { to: "/tickets", label: "Tickets", icon: Ticket, count: counts?.tickets },
        ...(me.is_admin ? [{ to: "/admin/users", label: "Admin settings", icon: Settings, match: ["/admin"] }] : []),
      ]}
      end={
        <Button asChild variant="outline" size="sm">
          <a href={DOCS} target="_blank" rel="noreferrer" aria-label="Documentation, opens in a new tab">
            <BookOpen data-icon="inline-start" />
            <span className="max-sm:hidden">Documentation</span>
            <ExternalLink data-icon="inline-end" className="max-sm:hidden" />
          </a>
        </Button>
      }
    />
  );
}

/** Every signed-in page outside a project: the top bar with the menu, then the page. */
export function AppLayout(): ReactElement {
  return (
    <SessionProvider>
      <TopBar tabs={<HomeTabs />} />
      <PageFrame>
        <Outlet />
      </PageFrame>
    </SessionProvider>
  );
}
