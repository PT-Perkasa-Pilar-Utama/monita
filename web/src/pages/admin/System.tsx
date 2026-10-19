import type { ReactElement } from "react";
import { Facts } from "@/components/app/Facts";
import { SettingsSection } from "@/components/app/SettingsSection";
import { PageHeader } from "@/components/layout/Page";
import { Skeleton } from "@/components/ui/skeleton";
import type { SystemView } from "@/lib/types";
import { useApi } from "@/lib/useApi";

/** Admin, System information: how the server is set up, from its settings file. */
export function System(): ReactElement {
  const res = useApi<SystemView>("/admin/system");
  const d = res.data;
  return (
    <>
      <PageHeader title="System information" />
      {!d ? (
        <Skeleton className="h-96 rounded-xl" />
      ) : (
        <div className="flex flex-col gap-6">
          <SettingsSection
            title="Server"
            description={
              <>
                These come from Monita's settings file on the server. To change one, edit that file and restart Monita. Run{" "}
                <code className="font-mono">monita whereis</code> to find the file.
              </>
            }
          >
            <Facts rows={d.server.map(([k, v]) => [k, v] as [string, string])} />
          </SettingsSection>
        </div>
      )}
    </>
  );
}
