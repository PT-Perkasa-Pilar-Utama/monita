import type { ReactElement } from "react";
import { useOutletContext } from "react-router-dom";
import { TicketBoard, TicketCard } from "@/components/app/TicketBoard";
import { PageHeader } from "@/components/layout/Page";
import { Skeleton } from "@/components/ui/skeleton";
import { day } from "@/lib/time";
import type { PublicBoard, PublicProject } from "@/lib/types";
import { useApi } from "@/lib/useApi";

/** The public board: tickets the team has triaged, by title and number only (SEC-14). */
export function KnownIssues(): ReactElement {
  const project = useOutletContext<PublicProject>();
  const res = useApi<PublicBoard>(`/public/${project.slug}/tickets`);
  return (
    <>
      <PageHeader title={`${project.name} known issues`} />
      {!res.data ? (
        <Skeleton className="h-64 rounded-xl" />
      ) : (
        <TicketBoard
          stages={["backlog", "in_progress", "done"]}
          tickets={res.data.tickets}
          counts={res.data.counts}
          card={(t) => <TicketCard title={t.title} number={t.number} aside={`Updated ${day(t.updated_at)}`} />}
        />
      )}
    </>
  );
}
