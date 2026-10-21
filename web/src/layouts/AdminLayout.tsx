import { useEffect } from "react";
import type { ReactElement } from "react";
import { Outlet, useLocation } from "react-router-dom";
import { Clock, Globe, Mail, Server, Timer, Users } from "@/components/icons";
import { SideNav, SidebarFrame } from "@/components/layout/SideNav";
import { rememberAdminPage } from "@/lib/adminPage";
import { useMe } from "@/lib/session";
import { NotFound } from "@/pages/NotFound";

/** The super admin's pages: User management, Locations, Rate limits, Email, Time zone and System information. Anyone else gets the not-found page. */
export function AdminLayout(): ReactElement {
  const me = useMe();
  const { pathname } = useLocation();
  useEffect(() => {
    if (me.sees_all) rememberAdminPage(pathname);
  }, [me.sees_all, pathname]);
  if (!me.sees_all) return <NotFound />;
  const item = (to: string, label: string, icon: typeof Users) => ({ to, label, icon, active: pathname === to });
  return (
    <SidebarFrame nav={<SideNav
          label="Admin"
          sections={[{ items: [item("/admin/users", "User management", Users), item("/admin/locations", "Locations", Globe), item("/admin/limits", "Rate limits", Timer), item("/admin/email", "Outgoing email", Mail), item("/admin/time-zone", "Time zone", Clock), item("/admin/system", "System information", Server)] }]}
        />}>
      <Outlet />
    </SidebarFrame>
  );
}
