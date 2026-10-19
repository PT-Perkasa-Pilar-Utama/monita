import type { ReactElement } from "react";
import { SettingsPage } from "@/components/app/SettingsSection";
import { StatusPageSection } from "@/components/app/StatusPageSection";

/** Settings, Status page: which tabs visitors see, and the page's address. */
export function StatusPageSettings(): ReactElement {
  return (
    <SettingsPage title="Status page">
      <StatusPageSection />
    </SettingsPage>
  );
}
