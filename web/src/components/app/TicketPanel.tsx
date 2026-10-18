import type { ReactElement } from "react";
import { ConfirmDialog } from "@/components/app/ConfirmDialog";
import { Side } from "@/components/app/Side";
import { ColumnPicker, type Move, PublicPreview, PublicTitle } from "@/components/app/TicketSide";
import { ReasonBadge, StageBadge } from "@/components/app/Tone";
import { UserRound } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { Sheet, SheetContent, SheetHeader, SheetTitle } from "@/components/ui/sheet";
import { Skeleton } from "@/components/ui/skeleton";
import { api, ApiError, errorText } from "@/lib/api";
import { stageLabel } from "@/lib/tickets";
import { ago, utc } from "@/lib/time";
import type { TicketRow } from "@/lib/types";
import { useApi } from "@/lib/useApi";
import { toast } from "sonner";

function Body({ t, path, onChanged }: { t: TicketRow; path: string; onChanged: () => void }): ReactElement {
  const move: Move = async (to, reason) => {
    try {
      await api(`${path}/stage`, "PUT", { stage: to, close_reason: reason });
      toast.success(to === t.stage ? "Close reason saved." : `Moved to ${stageLabel(to)}.`);
      onChanged();
      return null;
    } catch (err) {
      return err instanceof ApiError ? (err.fields.close_reason ?? err.fields.stage ?? err.message) : errorText(err);
    }
  };
  return (
    <>
      <SheetHeader>
        <SheetTitle className="text-2xl leading-tight break-words">
          {t.title} <span className="font-normal text-muted-foreground">#{t.number}</span>
        </SheetTitle>
        <PublicTitle t={t} path={path} onSaved={onChanged} />
        <div className="flex flex-wrap items-center gap-2 text-sm text-muted-foreground">
          <StageBadge stage={t.stage} />
          {t.close_reason && <ReasonBadge reason={t.close_reason} />}
          <span>
            {t.filed_by ? `Filed by ${t.filed_by}` : "Reported"} {ago(t.created_at)}
          </span>
        </div>
      </SheetHeader>
      <div className="grid flex-1 content-start gap-8 overflow-y-auto px-6 py-6 md:grid-cols-[minmax(0,1fr)_17rem]">
        <div className="flex min-w-0 flex-col gap-8">
          <article className="flex gap-3">
            <span aria-hidden="true" className="mt-1 flex size-8 shrink-0 items-center justify-center rounded-full bg-muted text-muted-foreground max-sm:hidden">
              <UserRound className="size-5" />
            </span>
            <div className="min-w-0 flex-1 overflow-hidden rounded-lg border bg-card">
              <header className="flex flex-wrap items-center justify-between gap-2 border-b bg-paper px-4 py-2.5 text-[13px]">
                <span>
                  <strong className="font-semibold">{t.filed_by ?? "Reporter"}</strong> <span className="text-muted-foreground">on {utc(t.created_at)}</span>
                </span>
                <span className="text-muted-foreground">Only your team sees this.</span>
              </header>
              <p className="px-4 py-4 text-sm whitespace-pre-wrap break-words">{t.description}</p>
              {t.page_url && (
                <footer className="border-t px-4 py-2.5 text-[13px]">
                  <span className="text-muted-foreground">Page </span>
                  <span className="font-mono break-all">{t.page_url}</span>
                </footer>
              )}
            </div>
          </article>
        </div>
        <aside className="flex flex-col gap-5">
          <ColumnPicker t={t} move={move} />
          <PublicPreview t={t} />
          {t.filed_by ? (
            <Side title="Filed by">
              <p className="text-sm break-all">{t.filed_by}, from the board</p>
            </Side>
          ) : (
            <Side title="Reporter's ref ID">
              <p className="text-sm">{t.ref_revoked ? "Revoked. The reporter cannot file new tickets." : "Active. The reporter can file new tickets."}</p>
              {!t.ref_revoked && (
                <ConfirmDialog
                  trigger={
                    <Button variant="danger" size="sm" className="self-start">
                      Revoke ref ID
                    </Button>
                  }
                  title="Revoke this ref ID?"
                  description="The reporter can no longer file tickets with it. Their tickets stay. You cannot undo this."
                  action="Revoke"
                  onConfirm={async () => {
                    await api(`${path}/revoke-ref`, "POST");
                    onChanged();
                  }}
                />
              )}
            </Side>
          )}
          <Side title="Dates">
            <dl className="grid grid-cols-[max-content_minmax(0,1fr)] gap-x-3 gap-y-1.5 text-sm">
              <dt className="text-muted-foreground">Reported</dt>
              <dd>{utc(t.created_at)}</dd>
              <dt className="text-muted-foreground">Updated</dt>
              <dd>{utc(t.updated_at)}</dd>
            </dl>
          </Side>
        </aside>
      </div>
    </>
  );
}

/**
 * One ticket over the board, as GitHub opens an issue from a project: the private report and
 * the public view on the left, the column and the reporter on the right. `fallback` is the
 * board's copy, shown until the ticket's own read lands.
 */
export function TicketPanel({
  slug,
  number,
  fallback,
  onClose,
  onChanged,
}: {
  slug: string;
  number: string | undefined;
  fallback: TicketRow | undefined;
  onClose: () => void;
  onChanged: () => void;
}): ReactElement {
  const path = number ? `/projects/${slug}/tickets/${encodeURIComponent(number)}` : null;
  const res = useApi<TicketRow>(path);
  const own = res.data && String(res.data.number) === number ? res.data : undefined;
  const t = own ?? fallback;
  const changed = (): void => {
    res.reload();
    onChanged();
  };
  return (
    <Sheet open={number !== undefined} onOpenChange={(o) => !o && onClose()}>
      <SheetContent
        aria-describedby={undefined}
        onOpenAutoFocus={(e) => {
          // Focus the panel, not its first field, so opening a ticket never starts an edit.
          e.preventDefault();
          (e.currentTarget as HTMLElement).focus();
        }}
      >
        {t && path ? (
          <Body t={t} path={path} onChanged={changed} />
        ) : res.error?.status === 404 ? (
          <SheetHeader>
            <SheetTitle>This ticket does not exist.</SheetTitle>
          </SheetHeader>
        ) : (
          <>
            <SheetHeader>
              <SheetTitle className="sr-only">Ticket</SheetTitle>
              <Skeleton className="h-8 w-2/3" />
              <Skeleton className="h-5 w-40" />
            </SheetHeader>
            <div className="p-6">
              <Skeleton className="h-48" />
            </div>
          </>
        )}
      </SheetContent>
    </Sheet>
  );
}
