import type { ReactElement } from "react";
import { EmailListSection } from "@/components/app/EmailListSection";
import { SettingsPage } from "@/components/app/SettingsSection";
import { Skeleton } from "@/components/ui/skeleton";
import { useProject } from "@/lib/project";
import type { ReportSchedule } from "@/lib/types";
import { useApi } from "@/lib/useApi";

const COPY: Record<ReportSchedule["cadence"], { title: string; description: string }> = {
  weekly: { title: "Weekly report", description: "Monita emails the summary report of last week, Monday to Sunday, as a PDF after the week ends." },
  monthly: { title: "Monthly report", description: "Monita emails the summary report of last month as a PDF after the month ends." },
};

/** Settings, Reports: the weekly and the monthly summary report by email (v1.2 Q59), and who
 * gets a post-mortem when it is published (Q63). */
export function ReportSettings(): ReactElement {
  const { project: p } = useProject();
  const base = `/projects/${p.slug}`;
  const schedules = useApi<ReportSchedule[]>(`${base}/report-schedules`);
  const pm = useApi<{ enabled: boolean; emails: string[] }>(`${base}/postmortem-recipients`);
  return (
    <SettingsPage title="Reports">
      {!schedules.data || !pm.data ? (
        <Skeleton className="h-48 rounded-xl" />
      ) : (
        <>
          {schedules.data.map((s) => (
            <EmailListSection
              key={`${s.cadence}-${String(s.enabled)}-${s.emails.join()}`}
              id={`report-${s.cadence}`}
              title={COPY[s.cadence].title}
              description={COPY[s.cadence].description}
              path={`${base}/report-schedules/${s.cadence}`}
              saved={s}
              onSaved={schedules.reload}
              switchLabel="Send this report"
            />
          ))}
          <EmailListSection
            key={`pm-${String(pm.data.enabled)}-${pm.data.emails.join()}`}
            id="report-postmortem"
            title="Post-mortems"
            description="Monita emails an incident's post-mortem as a PDF each time a tech lead publishes it."
            path={`${base}/postmortem-recipients`}
            saved={pm.data}
            onSaved={pm.reload}
            switchLabel="Email published post-mortems"
          />
        </>
      )}
    </SettingsPage>
  );
}
