import { useState } from "react";
import type { ReactElement } from "react";
import { ListFilters } from "@/components/app/ListFilters";
import { ProjectBadge } from "@/components/app/ProjectBadge";
import { RefreshButton } from "@/components/app/RefreshButton";
import { TableCard } from "@/components/app/TableCard";
import { TicketPanel } from "@/components/app/TicketPanel";
import { PageHeader } from "@/components/layout/Page";
import { Pager } from "@/components/list/Pager";
import { SortHead } from "@/components/list/SortHead";
import { TableEmpty } from "@/components/list/TableEmpty";
import { Skeleton } from "@/components/ui/skeleton";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { dateHead, nextDate } from "@/lib/dateSort";
import { ago } from "@/lib/time";
import type { HomeList, HomeTicket } from "@/lib/types";
import { LIVE_MS, useApi } from "@/lib/useApi";
import { useServerList } from "@/lib/useServerList";

/** New tickets in Triage across the projects you can open, newest first. A ticket opens in the
 * side panel over this list, so closing it lands back on the same rows. */
export function AllTickets(): ReactElement {
  const list = useServerList("monita:tickets:query");
  const { params } = list;
  const res = useApi<HomeList<"tickets", HomeTicket>>(`/tickets${list.query}`, LIVE_MS);
  const [open, setOpen] = useState<{ slug: string; number: string } | null>(null);
  const d = res.data;
  const sort = dateHead("reported", params.sort, true);
  return (
    <>
      <PageHeader title="Tickets" filtered={list.filtered} actions={<RefreshButton onClick={res.refresh} busy={res.refreshing} />} />
      <div className="flex flex-col gap-6 pb-4">
        <TableCard
          title={d ? `${d.total} in Triage` : "In Triage"}
          onClear={list.filtered ? list.clear : undefined}
          tools={<ListFilters noun="tickets" filter={{ q: list.draft, state: "", severity: "" }} onChange={(p) => p.q !== undefined && list.setDraft(p.q)} selects={false} />}
        >
          {!d ? (
            <Skeleton className="m-5 h-24" />
          ) : d.tickets.length === 0 && list.filtered ? (
            <TableEmpty title="" filtered noun="tickets" onClear={list.clear} />
          ) : d.tickets.length === 0 ? (
            <TableEmpty title="Triage is empty" description="New bug reports from every project land here until someone sorts them." />
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Ticket</TableHead>
                  <TableHead className="max-md:hidden">Project</TableHead>
                  <SortHead by="reported" label="Reported" sort={sort} onSort={() => list.set({ sort: nextDate(params.sort, true) })} className="text-right" />
                </TableRow>
              </TableHeader>
              <TableBody>
                {d.tickets.map((t) => {
                  const show = (): void => setOpen({ slug: t.project.slug, number: String(t.number) });
                  return (
                    <TableRow key={`${t.project.slug}-${t.number}`} className="cursor-pointer hover:bg-accent" onClick={show}>
                      <TableCell className="max-w-44 whitespace-normal sm:max-w-96">
                        <button type="button" onClick={show} className="text-left font-medium outline-none focus-visible:underline">
                          {t.title}
                        </button>
                        <div className="font-mono text-xs text-muted-foreground">#{t.number}</div>
                      </TableCell>
                      <TableCell className="max-md:hidden">
                        <span className="flex items-center gap-2">
                          <ProjectBadge name={t.project.name} iconUrl={t.project.icon_url} className="size-6 rounded-md text-[11px]" />
                          <span className="truncate">{t.project.name}</span>
                        </span>
                      </TableCell>
                      <TableCell className="text-right text-muted-foreground">{ago(t.created_at)}</TableCell>
                    </TableRow>
                  );
                })}
              </TableBody>
            </Table>
          )}
        </TableCard>
        {d && <Pager page={params.page} pageSize={d.page_size} total={d.total} noun="ticket" onPage={(page) => list.set({ page })} disabled={res.loading} />}
      </div>
      <TicketPanel slug={open?.slug ?? ""} number={open?.number} fallback={undefined} onClose={() => setOpen(null)} onChanged={res.refresh} />
    </>
  );
}
