import { useEffect, useState } from "react";
import type { FormEvent, ReactElement, ReactNode } from "react";
import { ConfirmDialog } from "@/components/app/ConfirmDialog";
import { StageRing } from "@/components/app/TicketBoard";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Field, FieldError, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";
import { Sheet, SheetContent, SheetHeader, SheetTitle } from "@/components/ui/sheet";
import { Skeleton } from "@/components/ui/skeleton";
import { Spinner } from "@/components/ui/spinner";
import { api, ApiError, errorText } from "@/lib/api";
import { REASONS, STAGES, reasonLabel, stageLabel } from "@/lib/tickets";
import { utc } from "@/lib/time";
import type { CloseReason, Stage, TicketRow } from "@/lib/types";
import { useApi } from "@/lib/useApi";
import { toast } from "sonner";

/** A titled block of the panel's right column. */
function Side({ title, children }: { title: ReactNode; children: ReactNode }): ReactElement {
  return (
    <section className="flex flex-col gap-3 border-b pb-5 last:border-b-0">
      <h3 className="text-[13px] font-semibold text-muted-foreground">{title}</h3>
      {children}
    </section>
  );
}

/**
 * The column a ticket sits in. A pick saves at once; Done waits for its close reason.
 */
function ColumnPicker({ t, move }: { t: TicketRow; move: (to: Stage, reason: CloseReason | null) => Promise<string | null> }): ReactElement {
  const [stage, setStage] = useState<Stage>(t.stage);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => setStage(t.stage), [t.stage]);
  const save = async (to: Stage, reason: CloseReason | null): Promise<void> => {
    setBusy(true);
    const refused = await move(to, reason);
    setError(refused);
    // A refused move leaves the ticket where it was, so the radios go back too.
    if (refused) setStage(t.stage);
    setBusy(false);
  };
  const pickStage = (v: string): void => {
    const to = v as Stage;
    setStage(to);
    setError(null);
    if (to !== "done") void save(to, null);
  };
  return (
    <>
      <Side title="Column">
        <RadioGroup value={stage} onValueChange={pickStage} disabled={busy} aria-label="Column" className="gap-2.5">
          {STAGES.map(([k, l]) => (
            <div key={k} className="flex items-center gap-2.5">
              <RadioGroupItem value={k} id={`col-pick-${k}`} />
              <Label htmlFor={`col-pick-${k}`} className="flex items-center gap-2 font-normal">
                <StageRing stage={k} />
                {l}
              </Label>
            </div>
          ))}
        </RadioGroup>
      </Side>
      {stage === "done" && (
        <Side
          title={
            <>
              Close reason <span aria-hidden="true" className="text-destructive">*</span>
            </>
          }
        >
          <RadioGroup
            value={t.stage === "done" ? (t.close_reason ?? "") : ""}
            onValueChange={(v) => void save("done", v as CloseReason)}
            disabled={busy}
            aria-label="Close reason"
            aria-required
            className="gap-2.5"
          >
            {REASONS.map(([k, l]) => (
              <div key={k} className="flex items-center gap-2.5">
                <RadioGroupItem value={k} id={`reason-pick-${k}`} />
                <Label htmlFor={`reason-pick-${k}`} className="font-normal">
                  {l}
                </Label>
              </div>
            ))}
          </RadioGroup>
          {t.stage !== "done" && <p className="text-[13px] text-muted-foreground">Pick a close reason to move it to Done.</p>}
        </Side>
      )}
      {busy && (
        <p className="flex items-center gap-2 text-[13px] text-muted-foreground">
          <Spinner /> Saving
        </p>
      )}
      {error && <p className="text-[13px] text-destructive">{error}</p>}
    </>
  );
}

/** The title the public board shows instead of the reporter's. */
function PublicTitle({ t, path, onSaved }: { t: TicketRow; path: string; onSaved: () => void }): ReactElement {
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
    <form onSubmit={(e) => void save(e)} noValidate className="flex flex-col gap-3">
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
      <p className="text-[13px] text-muted-foreground">
        Outside Triage the public board shows it as <strong className="font-medium text-foreground">{t.public_title ?? t.title}</strong>.
      </p>
    </form>
  );
}

function Body({ t, path, onChanged }: { t: TicketRow; path: string; onChanged: () => void }): ReactElement {
  const move = async (to: Stage, reason: CloseReason | null): Promise<string | null> => {
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
        <SheetTitle className="break-words">
          {t.title} <span className="font-normal text-muted-foreground">#{t.number}</span>
        </SheetTitle>
        <div className="flex flex-wrap items-center gap-2">
          <Badge variant="outline">
            <StageRing stage={t.stage} />
            {stageLabel(t.stage)}
          </Badge>
          {t.close_reason && <Badge variant="neutral">{reasonLabel(t.close_reason)}</Badge>}
          {t.ref_revoked && <Badge variant="outline">Ref ID revoked</Badge>}
        </div>
      </SheetHeader>
      <div className="grid flex-1 gap-8 overflow-y-auto px-6 py-6 md:grid-cols-[minmax(0,1fr)_13rem]">
        <div className="flex min-w-0 flex-col gap-6">
          <article className="overflow-hidden rounded-lg border bg-card">
            <header className="flex flex-wrap items-center justify-between gap-2 border-b bg-paper px-4 py-2.5 text-[13px] text-muted-foreground">
              <span>Reported {utc(t.created_at)}</span>
              <span>Only your team sees the report.</span>
            </header>
            <p className="px-4 py-4 text-sm whitespace-pre-wrap break-words">{t.description}</p>
            {t.page_url && (
              <footer className="border-t px-4 py-2.5 text-[13px]">
                <span className="text-muted-foreground">Page </span>
                <span className="font-mono break-all">{t.page_url}</span>
              </footer>
            )}
          </article>
          <PublicTitle t={t} path={path} onSaved={onChanged} />
        </div>
        <aside className="flex flex-col gap-5">
          <ColumnPicker t={t} move={move} />
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
          <Side title="Updated">
            <p className="text-sm">{utc(t.updated_at)}</p>
          </Side>
        </aside>
      </div>
    </>
  );
}

/**
 * One ticket over the board, as GitHub opens an issue from a project: the private report on the
 * left, the column and the reporter's ref ID on the right. `fallback` is the board's copy, shown
 * until the ticket's own read lands.
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
              <Skeleton className="h-7 w-2/3" />
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
