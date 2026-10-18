import { useEffect, useState } from "react";
import type { FormEvent, ReactElement, ReactNode } from "react";
import { StageRing, TicketCard } from "@/components/app/TicketBoard";
import { Pencil } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { Field, FieldError, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Select, SelectContent, SelectGroup, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Spinner } from "@/components/ui/spinner";
import { api, ApiError, errorText } from "@/lib/api";
import { REASONS, STAGES } from "@/lib/tickets";
import { ago, day } from "@/lib/time";
import type { CloseReason, Stage, TicketRow } from "@/lib/types";
import { toast } from "sonner";

export type Move = (to: Stage, reason: CloseReason | null) => Promise<string | null>;

/** A titled block of the panel's right column, as GitHub's issue sidebar has them. */
export function Side({ title, children }: { title: ReactNode; children: ReactNode }): ReactElement {
  return (
    <section className="flex flex-col gap-2.5 border-b pb-5 last:border-b-0">
      <h3 className="text-[13px] font-semibold text-muted-foreground">{title}</h3>
      {children}
    </section>
  );
}

/** The column, saved on pick. Done waits for its close reason, picked just under it. */
export function ColumnPicker({ t, move }: { t: TicketRow; move: Move }): ReactElement {
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

/** How the public board shows the ticket, and the title that board uses. The title edits in place. */
export function PublicSide({ t, path, onSaved }: { t: TicketRow; path: string; onSaved: () => void }): ReactElement {
  const [editing, setEditing] = useState(false);
  const [title, setTitle] = useState(t.public_title ?? "");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  useEffect(() => setTitle(t.public_title ?? ""), [t.public_title]);
  const cancel = (): void => {
    setEditing(false);
    setTitle(t.public_title ?? "");
    setError(null);
  };
  const save = async (e: FormEvent): Promise<void> => {
    e.preventDefault();
    setBusy(true);
    setError(null);
    try {
      await api(`${path}/public-title`, "PUT", { public_title: title });
      toast.success("Public title saved.");
      setEditing(false);
      onSaved();
    } catch (err) {
      setError(err instanceof ApiError ? (err.fields.public_title ?? err.message) : errorText(err));
    } finally {
      setBusy(false);
    }
  };
  return (
    <>
      <Side title="On the public board">
        {t.stage === "triage" ? (
          <p className="text-sm text-muted-foreground">Hidden until it leaves Triage.</p>
        ) : (
          <TicketCard title={t.public_title ?? t.title} number={t.number} aside={ago(t.updated_at)} meta={<span>Reported {day(t.created_at)}</span>} />
        )}
      </Side>
      <Side
        title={
          <span className="flex items-center justify-between gap-2">
            Public title
            {!editing && (
              <Button type="button" variant="ghost" size="icon" className="size-7" aria-label="Edit the public title" onClick={() => setEditing(true)}>
                <Pencil />
              </Button>
            )}
          </span>
        }
      >
        {editing ? (
          <form onSubmit={(e) => void save(e)} noValidate className="flex flex-col gap-2">
            <Field data-invalid={error ? true : undefined}>
              <FieldLabel htmlFor="public-title" className="sr-only">
                Public title, when the reporter's title says too much
              </FieldLabel>
              <Input id="public-title" autoFocus maxLength={120} placeholder={t.title} aria-invalid={error ? true : undefined} value={title} onChange={(e) => setTitle(e.target.value)} />
              {error && <FieldError>{error}</FieldError>}
            </Field>
            <p className="text-[13px] text-muted-foreground">Leave it empty to show the reporter's title.</p>
            <div className="flex justify-end gap-2">
              <Button type="button" variant="ghost" size="sm" onClick={cancel}>
                Cancel
              </Button>
              <Button type="submit" size="sm" disabled={busy || title.trim() === (t.public_title ?? "")}>
                {busy && <Spinner data-icon="inline-start" />}
                Save public title
              </Button>
            </div>
          </form>
        ) : (
          <p className="text-sm break-words">{t.public_title ?? <span className="text-muted-foreground">Same as the reporter's title</span>}</p>
        )}
      </Side>
    </>
  );
}
