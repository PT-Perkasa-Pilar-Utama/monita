import type { ReactElement } from "react";
import { Outlet, useLocation } from "react-router-dom";
import { Server, Users } from "@/components/icons";
import { SideNav, SidebarFrame } from "@/components/layout/SideNav";
import { useMe } from "@/lib/session";
import { NotFound } from "@/pages/NotFound";

/** The super admin's pages: Users and System. Anyone else gets the not-found page. */
export function AdminLayout(): ReactElement {
  const me = useMe();
  const { pathname } = useLocation();
  if (!me.is_admin) return <NotFound />;
  const item = (to: string, label: string, icon: typeof Users) => ({ to, label, icon, active: pathname === to });
  return (
    <SidebarFrame nav={<SideNav label="Admin" sections={[{ items: [item("/admin/users", "Users", Users), item("/admin/settings", "System", Server)] }]} />}>
      <Outlet />
    </SidebarFrame>
  );
}
