import { useEffect, useState } from "react";
import type { FormEvent, ReactElement, ReactNode } from "react";
import { ConfirmDialog } from "@/components/app/ConfirmDialog";
import { StageRing, TicketCard } from "@/components/app/TicketBoard";
import { ReasonBadge, StageBadge } from "@/components/app/Tone";
import { UserRound } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { Field, FieldError, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Select, SelectContent, SelectGroup, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Sheet, SheetContent, SheetHeader, SheetTitle } from "@/components/ui/sheet";
import { Skeleton } from "@/components/ui/skeleton";
import { Spinner } from "@/components/ui/spinner";
import { api, ApiError, errorText } from "@/lib/api";
import { REASONS, STAGES, stageLabel } from "@/lib/tickets";
import { ago, day, utc } from "@/lib/time";
import type { CloseReason, Stage, TicketRow } from "@/lib/types";
import { useApi } from "@/lib/useApi";
import { toast } from "sonner";

type Move = (to: Stage, reason: CloseReason | null) => Promise<string | null>;

/** A titled block of the panel's right column, as GitHub's issue sidebar has them. */
function Side({ title, children }: { title: ReactNode; children: ReactNode }): ReactElement {
  return (
    <section className="flex flex-col gap-2.5 border-b pb-5 last:border-b-0">
      <h3 className="text-[13px] font-semibold text-muted-foreground">{title}</h3>
      {children}
    </section>
  );
}

/** The column, saved on pick. Done waits for its close reason, picked just under it. */
function ColumnPicker({ t, move }: { t: TicketRow; move: Move }): ReactElement {
  const [stage, setStage] = useState<Stage>(t.stage);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => setStage(t.stage), [t.stage]);
  const save = async (to: Stage, reason: CloseReason | null): Promise<void> => {
    setBusy(true);
    const refused = await move(to, reason);
    setError(refused);
    // A refused move leaves the ticket where it was, so the picker goes back too.
    if (refused) setStage(t.stage);
    setBusy(false);
  };
  const pick = (v: string): void => {
    const to = v as Stage;
    setStage(to);
    setError(null);
    if (to !== "done") void save(to, null);
  };
  return (
    <Side title="Column">
      <Select value={stage} onValueChange={pick} disabled={busy}>
        <SelectTrigger aria-label="Column">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          <SelectGroup>
            {STAGES.map(([k, l]) => (
              <SelectItem key={k} value={k}>
                <StageRing stage={k} />
                {l}
              </SelectItem>
            ))}
          </SelectGroup>
        </SelectContent>
      </Select>
      {stage === "done" && (
        <Field>
          <FieldLabel htmlFor="close-reason" required>
            Close reason
          </FieldLabel>
          <Select value={t.stage === "done" ? (t.close_reason ?? undefined) : undefined} onValueChange={(v) => void save("done", v as CloseReason)} disabled={busy}>
            <SelectTrigger id="close-reason" aria-required>
              <SelectValue placeholder="Pick a reason to close it" />
            </SelectTrigger>
            <SelectContent>
              <SelectGroup>
                {REASONS.map(([k, l]) => (
                  <SelectItem key={k} value={k}>
                    {l}
                  </SelectItem>
                ))}
              </SelectGroup>
            </SelectContent>
          </Select>
        </Field>
      )}
      {busy && (
        <p className="flex items-center gap-2 text-[13px] text-muted-foreground">
          <Spinner /> Saving
        </p>
      )}
      {error && <p className="text-[13px] text-destructive">{error}</p>}
    </Side>
  );
}

/** The ticket as a visitor sees it, and the title field that changes it. */
function PublicView({ t, path, onSaved }: { t: TicketRow; path: string; onSaved: () => void }): ReactElement {
  const [title, setTitle] = useState(t.public_title ?? "");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  useEffect(() => setTitle(t.public_title ?? ""), [t.public_title]);
  const save = async (e: FormEvent): Promise<void> => {
    e.preventDefault();
    setBusy(true);
    setError(null);
    try {
      await api(`${path}/public-title`, "PUT", { public_title: title });
      toast.success("Public title saved.");
      onSaved();
    } catch (err) {
      setError(err instanceof ApiError ? (err.fields.public_title ?? err.message) : errorText(err));
    } finally {
      setBusy(false);
    }
  };
  return (
    <section className="flex flex-col gap-4">
      <h3 className="text-sm font-semibold">What the public sees</h3>
      {t.stage === "triage" ? (
        <p className="rounded-lg border border-dashed px-4 py-3 text-sm text-muted-foreground">Nothing yet. The public board shows this ticket once it leaves Triage.</p>
      ) : (
        <div className="flex flex-col gap-2 rounded-xl border bg-paper p-2 sm:max-w-sm">
          <span className="flex items-center gap-2 px-2 pt-1 text-[13px] font-semibold">
            <StageRing stage={t.stage} />
            {stageLabel(t.stage)}
          </span>
          <TicketCard title={t.public_title ?? t.title} number={t.number} aside={ago(t.updated_at)} meta={<span>Reported {day(t.created_at)}</span>} />
        </div>
      )}
      <form onSubmit={(e) => void save(e)} noValidate>
        <Field data-invalid={error ? true : undefined}>
          <FieldLabel htmlFor="public-title">Public title, when the reporter's title says too much</FieldLabel>
          <div className="flex flex-col gap-2 sm:flex-row">
            <Input id="public-title" maxLength={120} aria-invalid={error ? true : undefined} value={title} onChange={(e) => setTitle(e.target.value)} />
            <Button type="submit" variant="secondary" disabled={busy || title.trim() === (t.public_title ?? "")}>
              {busy && <Spinner data-icon="inline-start" />}
              Save public title
            </Button>
          </div>
          {error && <FieldError>{error}</FieldError>}
        </Field>
      </form>
    </section>
  );
}

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
        <div className="flex flex-wrap items-center gap-2 text-sm text-muted-foreground">
          <StageBadge stage={t.stage} />
          {t.close_reason && <ReasonBadge reason={t.close_reason} />}
          <span>
            {t.filed_by ? `Filed by ${t.filed_by}` : "Reported"} {ago(t.created_at)}
          </span>
        </div>
      </SheetHeader>
      <div className="grid flex-1 content-start gap-8 overflow-y-auto px-6 py-6 md:grid-cols-[minmax(0,1fr)_15rem]">
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
          <PublicView t={t} path={path} onSaved={onChanged} />
        </div>
        <aside className="flex flex-col gap-5">
          <ColumnPicker t={t} move={move} />
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
