import type { ReactElement } from "react";
import { SettingsPage } from "@/components/app/SettingsSection";
import { CustomStatusSection } from "@/components/app/CustomStatusSection";
import { StatusPageSection } from "@/components/app/StatusPageSection";

/** Settings, Status page: which tabs visitors see, the page's address, and how to build your own. */
export function StatusPageSettings(): ReactElement {
  return (
    <SettingsPage title="Status page">
      <StatusPageSection />
      <CustomStatusSection />
    </SettingsPage>
  );
}
