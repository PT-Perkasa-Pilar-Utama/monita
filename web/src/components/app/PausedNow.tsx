import { useState } from "react";
import type { ReactElement } from "react";
import { Link } from "react-router-dom";
import { runBatch } from "@/components/app/SourceBatch";
import { rowTape } from "@/components/app/StateStamp";
import { TableCard } from "@/components/app/TableCard";
import { Play } from "@/components/icons";
import { TableEmpty } from "@/components/list/TableEmpty";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { Spinner } from "@/components/ui/spinner";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { errorText } from "@/lib/api";
import { Writable } from "@/lib/session";
import type { SourceKind } from "@/lib/sources";
import { ago } from "@/lib/time";
import type { HeartbeatList, MonitorList } from "@/lib/types";
import { useApi } from "@/lib/useApi";
import { toast } from "sonner";

type Row = { kind: SourceKind; id: number; name: string; pausedAt: number };

/** Every paused monitor and heartbeat of the project, read from the two capped lists, so the
 * Maintenance tab shows everything Monita is not watching in one place. */
export function PausedNow({ slug }: { slug: string }): ReactElement {
  const monitors = useApi<MonitorList>(`/projects/${slug}/monitors`);
  const heartbeats = useApi<HeartbeatList>(`/projects/${slug}/heartbeats`);
  const [busy, setBusy] = useState<string | null>(null);
  const ready = monitors.data && heartbeats.data;
  const rows: Row[] = [
    ...(monitors.data?.monitors ?? []).flatMap((m) => (m.paused_at === null ? [] : [{ kind: "monitor" as const, id: m.id, name: m.name, pausedAt: m.paused_at }])),
    ...(heartbeats.data?.heartbeats ?? []).flatMap((h) => (h.paused_at === null ? [] : [{ kind: "heartbeat" as const, id: h.id, name: h.name, pausedAt: h.paused_at }])),
  ].sort((a, b) => a.pausedAt - b.pausedAt);
  const resume = async (r: Row): Promise<void> => {
    setBusy(`${r.kind}:${r.id}`);
    try {
      await runBatch(slug, r.kind, "resume", [r.id]);
      toast.success(`${r.name} runs again.`);
      (r.kind === "monitor" ? monitors : heartbeats).reload();
    } catch (err) {
      toast.error(errorText(err));
    } finally {
      setBusy(null);
    }
  };
  return (
    <TableCard title={ready ? `${rows.length} paused` : "Paused now"}>
      {!ready ? (
        <Skeleton className="m-5 h-16" />
      ) : rows.length === 0 ? (
        <TableEmpty title="Nothing is paused" description="A paused monitor or heartbeat shows here until you resume it, so you can see at a glance what Monita is not watching." />
      ) : (
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Paused</TableHead>
              <TableHead className="max-sm:hidden">Kind</TableHead>
              <TableHead>Since</TableHead>
              <TableHead className="text-right">
                <span className="sr-only">Action</span>
              </TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {rows.map((r) => (
              <TableRow key={`${r.kind}:${r.id}`} style={rowTape("paused")}>
                <TableCell className="font-medium">
                  <Link to={`/projects/${slug}/${r.kind}s/${r.id}`} className="hover:underline">
                    {r.name}
                  </Link>
                </TableCell>
                <TableCell className="text-muted-foreground capitalize max-sm:hidden">{r.kind}</TableCell>
                <TableCell className="text-muted-foreground">{ago(r.pausedAt)}</TableCell>
                <TableCell className="text-right">
                  <Writable>
                    <Button size="sm" variant="secondary" disabled={busy !== null} onClick={() => void resume(r)}>
                      {busy === `${r.kind}:${r.id}` ? <Spinner data-icon="inline-start" /> : <Play data-icon="inline-start" />}
                      Resume
                    </Button>
                  </Writable>
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      )}
    </TableCard>
  );
}
