import type { ReactElement } from "react";
import { Outlet } from "react-router-dom";
import { PageFrame } from "@/components/layout/Page";
import { TopBar } from "@/components/layout/TopBar";
import { SessionProvider } from "@/lib/session";

/** Every signed-in page: the user loads once here, so moving between areas keeps the header. */
export function SignedInLayout(): ReactElement {
  return (
    <SessionProvider>
      <Outlet />
    </SessionProvider>
  );
}

/** Every signed-in page outside a project: the top bar, then the page. */
export function AppLayout(): ReactElement {
  return (
    <>
      <TopBar />
      <PageFrame>
        <Outlet />
      </PageFrame>
    </>
  );
}
