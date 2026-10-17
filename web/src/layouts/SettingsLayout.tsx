import type { ReactElement } from "react";
import { Outlet, useLocation } from "react-router-dom";
import { Bell, Key, Settings, Shield } from "@/components/icons";
import { SideNav, SidebarFrame } from "@/components/layout/SideNav";
import { useProject } from "@/lib/project";

/** A project's settings: General, Alerts, API keys and Allowlist, each its own page. */
export function SettingsLayout(): ReactElement {
  const { project: p } = useProject();
  const { pathname } = useLocation();
  const base = `/projects/${p.slug}`;
  const item = (to: string, label: string, icon: typeof Settings) => ({ to, label, icon, active: pathname === to });
  return (
    <SidebarFrame
      nav={
        <SideNav
          label="Settings"
          sections={[
            {
              items: [
                item(`${base}/settings`, "General", Settings),
                item(`${base}/alerts`, "Alerts", Bell),
                item(`${base}/settings/keys`, "API keys", Key),
                item(`${base}/settings/allowlist`, "Allowlist", Shield),
              ],
            },
          ]}
        />
      }
    >
      <Outlet />
    </SidebarFrame>
  );
}
