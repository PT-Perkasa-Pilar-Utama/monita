import { useEffect, useState } from "react";
import { useCanEdit, Writable } from "@/lib/session";
import type { FormEvent, ReactElement } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";
import { ConnectApp } from "@/components/app/ConnectApp";
import { IssueRefId } from "@/components/app/IssueRefId";
import { NewTicketSheet } from "@/components/app/NewTicketSheet";
import { ShowMore, TicketBoard, TicketCard } from "@/components/app/TicketBoard";
import { TicketPanel } from "@/components/app/TicketPanel";
import { ReasonBadge } from "@/components/app/Tone";
import { PageHeader } from "@/components/layout/Page";
import { TableEmpty } from "@/components/list/TableEmpty";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Dialog, DialogClose, DialogContent, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { FieldLegend, FieldSet } from "@/components/ui/field";
import { Skeleton } from "@/components/ui/skeleton";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { api, ApiError, errorText } from "@/lib/api";
import { useProject } from "@/lib/project";
import { REASONS, STAGE_NOTES, dropOutcome, moveOnBoard, stageLabel, withMore } from "@/lib/tickets";
import { ago, nowSecs } from "@/lib/time";
import type { TicketBoard as Board, CloseReason, Stage, TicketRow } from "@/lib/types";
import { useApi } from "@/lib/useApi";
import { useMorePages } from "@/lib/useMorePages";
import { toast } from "sonner";

/** A card dropped on Done waits here for its close reason. */
function CloseReasonDialog({ ticket, onPick, onCancel }: { ticket: TicketRow | null; onPick: (r: CloseReason) => void; onCancel: () => void }): ReactElement {
  const [reason, setReason] = useState<CloseReason | "">("");
  useEffect(() => setReason(""), [ticket]);
  const submit = (e: FormEvent): void => {
    e.preventDefault();
    if (reason) onPick(reason);
  };
  return (
    <Dialog open={ticket !== null} onOpenChange={(o) => !o && onCancel()}>
      <DialogContent aria-describedby={undefined}>
        <form onSubmit={submit} className="flex flex-col gap-6">
          <DialogHeader>
            <DialogTitle>Move #{ticket?.number} to Done</DialogTitle>
          </DialogHeader>
          <FieldSet>
            <FieldLegend variant="label">
              Close reason <span aria-hidden="true" className="text-destructive">*</span>
            </FieldLegend>
            <ToggleGroup type="single" variant="outline" value={reason} onValueChange={(v) => setReason(v as CloseReason | "")} className="flex-wrap">
              {REASONS.map(([k, l]) => (
                <ToggleGroupItem key={k} value={k} className="px-3">
                  {l}
                </ToggleGroupItem>
              ))}
            </ToggleGroup>
          </FieldSet>
          <DialogFooter>
            <DialogClose asChild>
              <Button type="button" variant="ghost">
                Cancel
              </Button>
            </DialogClose>
            <Button type="submit" disabled={!reason}>
              Move to Done
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

/** The project's ticket board. A card opens its ticket in a panel, and drags to another column. */
export function Tickets(): ReactElement {
  const canEdit = useCanEdit();
  const { project: p, reload: reloadProject } = useProject();
  const { number } = useParams();
  const navigate = useNavigate();
  const base = `/projects/${p.slug}/tickets`;
  const res = useApi<Board>(base);
  const pages = useMorePages<TicketRow>(base);
  // The board as it looks after a drop, until the server's copy comes back.
  const [local, setLocal] = useState<Board | null>(null);
  const [closing, setClosing] = useState<TicketRow | null>(null);
  useEffect(() => setLocal(null), [res.data]);
  const server = res.data && { ...res.data, tickets: withMore(res.data.tickets, pages.more) };
  const d = local ?? server;

  const changed = (): void => {
    // A move shifts every later page, so the columns fold back to their first page.
    pages.reset();
    res.reload();
    reloadProject();
  };
  const move = async (t: TicketRow, to: Stage, reason: CloseReason | null): Promise<void> => {
    if (!d) return;
    setLocal(moveOnBoard(d, t.number, to, reason, nowSecs()));
    try {
      await api(`${base}/${t.number}/stage`, "PUT", { stage: to, close_reason: reason });
      toast.success(`Moved #${t.number} to ${stageLabel(to)}.`);
    } catch (err) {
      setLocal(null);
      toast.error(err instanceof ApiError ? err.message : errorText(err));
    }
    changed();
  };
  const drop = (t: TicketRow, to: Stage): void => {
    const outcome = dropOutcome(t.stage, to);
    if (outcome === "move") void move(t, to, null);
    if (outcome === "ask_reason") setClosing(t);
  };

  return (
    <>
      <PageHeader
        title="Tickets"
        actions={
          <>
            <Writable>
              <IssueRefId />
            </Writable>
            <ConnectApp slug={p.slug} />
            <Writable>
              <NewTicketSheet slug={p.slug} onFiled={changed} />
            </Writable>
          </>
        }
      />
      {!d ? (
        <Skeleton className="h-[calc(100dvh-15rem)] rounded-xl" />
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
          notes={STAGE_NOTES}
          onMove={canEdit ? drop : undefined}
          footer={(s, _shown, total) =>
            s === "done" ? (
              <Button asChild variant="ghost" size="sm" className="w-full text-muted-foreground">
                <Link to={`${base}/done`}>See all {total.toLocaleString()} done tickets</Link>
              </Button>
            ) : (
              <ShowMore busy={pages.busy === s} onClick={() => void pages.load(s)} />
            )
          }
          card={(t) => (
            <Link
              to={`${base}/${t.number}`}
              draggable={false}
              aria-current={String(t.number) === number ? "true" : undefined}
              className="block rounded-lg outline-none focus-visible:ring-[3px] focus-visible:ring-ring hover:[&>div]:border-foreground/25 hover:[&>div]:shadow-sm aria-[current]:[&>div]:border-foreground/40"
            >
              <TicketCard
                title={t.title}
                number={t.number}
                meta={
                  <>
                    {t.close_reason && <ReasonBadge reason={t.close_reason} />}
                    {t.public_title && <Badge variant="outline">Public title</Badge>}
                  </>
                }
                aside={ago(t.updated_at)}
              />
            </Link>
          )}
        />
      )}
      <TicketPanel
        slug={p.slug}
        number={number}
        fallback={d?.tickets.find((t) => String(t.number) === number)}
        onClose={() => navigate(base)}
        onChanged={changed}
      />
      <CloseReasonDialog
        ticket={closing}
        onCancel={() => setClosing(null)}
        onPick={(r) => {
          if (closing) void move(closing, "done", r);
          setClosing(null);
        }}
      />
    </>
  );
}
