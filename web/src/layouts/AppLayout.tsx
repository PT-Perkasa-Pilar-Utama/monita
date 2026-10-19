import type { ReactElement } from "react";
import { Outlet } from "react-router-dom";
import { PageFrame } from "@/components/layout/Page";
import { TopBar } from "@/components/layout/TopBar";
import { SessionProvider } from "@/lib/session";

/** Every signed-in page outside a project: the top bar, then the page. */
export function AppLayout(): ReactElement {
  return (
    <SessionProvider>
      <TopBar />
      <PageFrame>
        <Outlet />
      </PageFrame>
    </SessionProvider>
  );
}
