import type { ReactElement } from "react";
import { TableCard } from "@/components/app/TableCard";
import { TableEmpty } from "@/components/list/TableEmpty";
import { Badge } from "@/components/ui/badge";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { reasonLabel, stageLabel } from "@/lib/tickets";
import { stamp } from "@/lib/time";
import type { MyTicket } from "@/lib/types";

/** A reporter's own tickets: title, number, column and when it last moved. */
export function MyTickets({ tickets }: { tickets: MyTicket[] }): ReactElement {
  return (
    <TableCard title={`Your tickets, ${tickets.length}`}>
      {tickets.length === 0 ? (
        <TableEmpty title="Your reports will show here once you send one." />
      ) : (
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Ticket</TableHead>
              <TableHead>Column</TableHead>
              <TableHead className="text-right max-sm:hidden">Updated</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {tickets.map((t) => (
              <TableRow key={t.number}>
                <TableCell className="max-w-44 whitespace-normal sm:max-w-80">
                  <span className="font-medium">{t.title}</span>
                  <div className="font-mono text-xs text-muted-foreground">#{t.number}</div>
                </TableCell>
                <TableCell>
                  <span className="flex flex-wrap gap-1.5">
                    <Badge variant="neutral">{stageLabel(t.stage)}</Badge>
                    {t.close_reason && <Badge variant="outline">{reasonLabel(t.close_reason)}</Badge>}
                  </span>
                </TableCell>
                <TableCell className="text-right font-mono text-[13px] text-muted-foreground max-sm:hidden">{stamp(t.updated_at)}</TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      )}
    </TableCard>
  );
}
