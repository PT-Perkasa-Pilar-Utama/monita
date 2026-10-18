import type { ReactElement } from "react";
import { useOutletContext } from "react-router-dom";
import { ShowMore, TicketBoard, TicketCard } from "@/components/app/TicketBoard";
import { PageHeader } from "@/components/layout/Page";
import { Skeleton } from "@/components/ui/skeleton";
import { PUBLIC_NOTES, withMore } from "@/lib/tickets";
import { ago, day } from "@/lib/time";
import type { PublicBoard, PublicProject, PublicTicket } from "@/lib/types";
import { useApi } from "@/lib/useApi";
import { useMorePages } from "@/lib/useMorePages";

/** The public board: tickets the team has triaged, by title and number only (SEC-14). */
export function KnownIssues(): ReactElement {
  const project = useOutletContext<PublicProject>();
  const base = `/public/${project.slug}/tickets`;
  const res = useApi<PublicBoard>(base);
  const pages = useMorePages<PublicTicket>(base);
  return (
    <>
      <PageHeader title={`${project.name} known issues`} />
      {!res.data ? (
        <Skeleton className="h-[calc(100dvh-15rem)] rounded-xl" />
      ) : (
        <TicketBoard
          stages={["backlog", "in_progress", "done"]}
          tickets={withMore(res.data.tickets, pages.more)}
          counts={res.data.counts}
          notes={PUBLIC_NOTES}
          footer={(s) => <ShowMore busy={pages.busy === s} onClick={() => void pages.load(s)} />}
          card={(t) => <TicketCard title={t.title} number={t.number} aside={<span title={`Updated ${day(t.updated_at)}`}>{ago(t.updated_at)}</span>} meta={<span>Reported {day(t.created_at)}</span>} />}
        />
      )}
    </>
  );
}
