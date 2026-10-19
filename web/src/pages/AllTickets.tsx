import type { ReactElement } from "react";
import { Link, useSearchParams } from "react-router-dom";
import { LinkRow, ROW_LINK } from "@/components/app/LinkRow";
import { ProjectBadge } from "@/components/app/ProjectBadge";
import { RefreshButton } from "@/components/app/RefreshButton";
import { TableCard } from "@/components/app/TableCard";
import { PageHeader } from "@/components/layout/Page";
import { Pager } from "@/components/list/Pager";
import { TableEmpty } from "@/components/list/TableEmpty";
import { Skeleton } from "@/components/ui/skeleton";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { pageParam } from "@/lib/page";
import { ago } from "@/lib/time";
import type { HomeList, HomeTicket } from "@/lib/types";
import { LIVE_MS, useApi } from "@/lib/useApi";

/** New tickets in Triage across the projects you can open, newest first, so you can sort them in one place. */
export function AllTickets(): ReactElement {
  const [params, setParams] = useSearchParams();
  const page = pageParam(params.get("page"));
  const res = useApi<HomeList<"tickets", HomeTicket>>(`/tickets?page=${page}`, LIVE_MS);
  const d = res.data;
  return (
    <>
      <PageHeader title="Tickets" actions={<RefreshButton onClick={res.refresh} busy={res.refreshing} />} />
      <div className="flex flex-col gap-6 pb-4">
        <TableCard title={d ? `${d.total} in Triage` : "In Triage"}>
          {!d ? (
            <Skeleton className="m-4 h-40" />
          ) : d.tickets.length === 0 ? (
            <TableEmpty title="Triage is empty" description="New bug reports from every project land here until someone sorts them." />
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Ticket</TableHead>
                  <TableHead className="max-md:hidden">Project</TableHead>
                  <TableHead className="text-right">Reported</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {d.tickets.map((t) => {
                  const to = `/projects/${t.project.slug}/tickets/${t.number}`;
                  return (
                    <LinkRow key={`${t.project.slug}-${t.number}`} to={to}>
                      <TableCell className="max-w-44 whitespace-normal sm:max-w-96">
                        <Link to={to} className={ROW_LINK}>
                          {t.title}
                        </Link>
                        <div className="font-mono text-xs text-muted-foreground">#{t.number}</div>
                      </TableCell>
                      <TableCell className="max-md:hidden">
                        <span className="flex items-center gap-2">
                          <ProjectBadge name={t.project.name} iconUrl={t.project.icon_url} className="size-6 rounded-md text-[11px]" />
                          <span className="truncate">{t.project.name}</span>
                        </span>
                      </TableCell>
                      <TableCell className="text-right text-muted-foreground">{ago(t.created_at)}</TableCell>
                    </LinkRow>
                  );
                })}
              </TableBody>
            </Table>
          )}
        </TableCard>
        {d && <Pager page={page} pageSize={d.page_size} total={d.total} noun="ticket" onPage={(p) => setParams({ page: String(p) })} disabled={res.loading} />}
      </div>
    </>
  );
}
