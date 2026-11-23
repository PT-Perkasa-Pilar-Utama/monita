import type { ReactElement } from "react";
import { ReportScheduleSection } from "@/components/app/ReportScheduleSection";
import { SettingsPage } from "@/components/app/SettingsSection";
import { Skeleton } from "@/components/ui/skeleton";
import { useProject } from "@/lib/project";
import type { ReportSchedule } from "@/lib/types";
import { useApi } from "@/lib/useApi";

/** Settings, Reports: the weekly and the monthly summary report by email (v1.2 Q59). */
export function ReportSettings(): ReactElement {
  const { project: p } = useProject();
  const res = useApi<ReportSchedule[]>(`/projects/${p.slug}/report-schedules`);
  return (
    <SettingsPage title="Reports">
      {!res.data ? (
        <Skeleton className="h-48 rounded-xl" />
      ) : (
        res.data.map((s) => <ReportScheduleSection key={`${s.cadence}-${String(s.enabled)}-${s.emails.join()}`} slug={p.slug} schedule={s} onSaved={res.reload} />)
      )}
    </SettingsPage>
  );
}
