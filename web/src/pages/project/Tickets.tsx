import type { ReactElement } from "react";
import { Link } from "react-router-dom";
import { TicketBoard, TicketCard } from "@/components/app/TicketBoard";
import { Globe } from "@/components/icons";
import { PageHeader } from "@/components/layout/Page";
import { TableEmpty } from "@/components/list/TableEmpty";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { useProject } from "@/lib/project";
import { ago } from "@/lib/time";
import { reasonLabel } from "@/lib/tickets";
import type { TicketBoard as Board } from "@/lib/types";
import { useApi } from "@/lib/useApi";

/** The project's ticket board: Triage, Backlog, In progress and Done. */
export function Tickets(): ReactElement {
  const { project: p } = useProject();
  const base = `/projects/${p.slug}/tickets`;
  const res = useApi<Board>(base);
  const d = res.data;
  return (
    <>
      <PageHeader
        title="Tickets"
        actions={
          <Button asChild variant="secondary">
            <Link to={`/status/${p.slug}/tickets`}>
              <Globe data-icon="inline-start" />
              Public board
            </Link>
          </Button>
        }
      />
      {!d ? (
        <Skeleton className="h-80 rounded-xl" />
      ) : d.tickets.length === 0 ? (
        <Card className="py-0">
          <TableEmpty
            title="Bug reports arrive here"
            description={`Project users report bugs with a ref ID from your system, through the API or the form at /status/${p.slug}/report.`}
          />
        </Card>
      ) : (
        <TicketBoard
          stages={["triage", "backlog", "in_progress", "done"]}
          tickets={d.tickets}
          counts={d.counts}
          card={(t) => (
            <Link to={`${base}/${t.number}`} className="block rounded-lg outline-none hover:[&>div]:border-input focus-visible:ring-[3px] focus-visible:ring-ring">
              <TicketCard
                title={t.title}
                number={t.number}
                meta={
                  <>
                    {t.close_reason && <Badge variant="neutral">{reasonLabel(t.close_reason)}</Badge>}
                    {t.public_title && <Badge variant="outline">Public title</Badge>}
                  </>
                }
                aside={ago(t.updated_at)}
              />
            </Link>
          )}
        />
      )}
    </>
  );
}
