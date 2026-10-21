import type { ReactElement } from "react";
import { TableCard } from "@/components/app/TableCard";
import { StateBadge } from "@/components/app/Tone";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { fresh } from "@/lib/locations";
import { ago, nowSecs } from "@/lib/time";
import type { PlaceView } from "@/lib/types";

/** Each place a monitor checks from, with its newest result. A stale one no longer counts (technical-specs 14.7). */
export function PlacesCard({ places, intervalS, quorum }: { places: PlaceView[]; intervalS: number; quorum: number }): ReactElement {
  const now = nowSecs();
  return (
    <TableCard title={`Places: a check fails when ${quorum} of ${places.length} fail`}>
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>Place</TableHead>
            <TableHead>Newest result</TableHead>
            <TableHead>When</TableHead>
            <TableHead className="max-md:hidden">Error</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {places.map((p) => {
            const last = p.last;
            const counts = last !== null && fresh(last[0], now, intervalS);
            return (
              <TableRow key={p.location_id ?? 0}>
                <TableCell className="font-medium">{p.name}</TableCell>
                <TableCell>
                  {last === null ? <StateBadge kind="unknown" label="No data yet" /> : <StateBadge kind={last[1] ? "up" : "down"} label={last[1] ? "OK" : "Failed"} />}
                </TableCell>
                <TableCell className={last !== null && !counts ? "text-down" : undefined}>
                  {last === null ? "–" : counts ? ago(last[0], now) : `${ago(last[0], now)}, too old to count`}
                </TableCell>
                <TableCell className="max-w-96 truncate text-muted-foreground max-md:hidden">{last?.[2] ?? ""}</TableCell>
              </TableRow>
            );
          })}
        </TableBody>
      </Table>
    </TableCard>
  );
}
