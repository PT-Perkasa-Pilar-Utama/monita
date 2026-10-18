import type { ReactElement } from "react";
import { Link, useSearchParams } from "react-router-dom";
import { LinkRow, ROW_LINK } from "@/components/app/LinkRow";
import { TableCard } from "@/components/app/TableCard";
import { ReasonBadge } from "@/components/app/Tone";
import { Crumbs, PageHeader } from "@/components/layout/Page";
import { Pager } from "@/components/list/Pager";
import { TableEmpty } from "@/components/list/TableEmpty";
import { Skeleton } from "@/components/ui/skeleton";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { pageParam } from "@/lib/page";
import { useProject } from "@/lib/project";
import { ago, day } from "@/lib/time";
import type { ColumnPage, TicketRow } from "@/lib/types";
import { useApi } from "@/lib/useApi";

/** Every Done ticket, newest close first, a page at a time. The board shows only the newest. */
export function DoneTickets(): ReactElement {
  const { project: p } = useProject();
  const base = `/projects/${p.slug}/tickets`;
  const [params, setParams] = useSearchParams();
  const page = pageParam(params.get("page"));
  const res = useApi<ColumnPage<TicketRow>>(`${base}?stage=done&page=${page}`);
  const d = res.data;
  return (
    <>
      <Crumbs items={[{ label: "Tickets", to: base }, { label: "Done" }]} />
      <PageHeader title="Done tickets" />
      <div className="flex flex-col gap-4">
        <TableCard title={d ? `${d.total.toLocaleString()} done ${d.total === 1 ? "ticket" : "tickets"}` : "Done tickets"}>
          {!d ? (
            <Skeleton className="m-5 h-48" />
          ) : d.tickets.length === 0 ? (
            <TableEmpty title="No ticket is done yet" description="A ticket moved to Done with a close reason shows here." />
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Ticket</TableHead>
                  <TableHead>Close reason</TableHead>
                  <TableHead className="max-md:hidden">Reported</TableHead>
                  <TableHead className="text-right">Closed</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {d.tickets.map((t) => (
                  <LinkRow key={t.number} to={`${base}/${t.number}`}>
                    <TableCell className="whitespace-normal">
                      <span className="mr-2 font-mono text-xs text-muted-foreground">#{t.number}</span>
                      <Link to={`${base}/${t.number}`} className={ROW_LINK}>
                        {t.title}
                      </Link>
                    </TableCell>
                    <TableCell>{t.close_reason && <ReasonBadge reason={t.close_reason} />}</TableCell>
                    <TableCell className="text-muted-foreground max-md:hidden">{day(t.created_at)}</TableCell>
                    <TableCell className="text-right text-muted-foreground">{ago(t.updated_at)}</TableCell>
                  </LinkRow>
                ))}
              </TableBody>
            </Table>
          )}
        </TableCard>
        {d && (
          <Pager
            page={page}
            pageSize={d.page_size}
            total={d.total}
            noun="done ticket"
            disabled={res.loading}
            onPage={(n) => setParams(n > 1 ? { page: String(n) } : {})}
          />
        )}
      </div>
    </>
  );
}
