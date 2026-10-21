import type { ReactElement } from "react";
import { Outlet, useLocation } from "react-router-dom";
import { Key, ShieldCheck, UserRound } from "@/components/icons";
import { SideNav, SidebarFrame } from "@/components/layout/SideNav";

/** User settings: the signed-in user's own account, one topic a page, as Admin lays out its pages. */
export function AccountLayout(): ReactElement {
  const { pathname } = useLocation();
  const item = (to: string, label: string, icon: typeof ShieldCheck) => ({ to, label, icon, active: pathname === to });
  return (
    <SidebarFrame nav={<SideNav label="User settings" sections={[{ items: [item("/account/profile", "Profile", UserRound), item("/account/password", "Password", Key), item("/account/two-factor", "Two-factor sign-in", ShieldCheck)] }]} />}>
      <Outlet />
    </SidebarFrame>
  );
}
