import { useState } from "react";
import type { FormEvent, ReactElement } from "react";
import { useSearchParams } from "react-router-dom";
import { FormSheetBody } from "@/components/app/FormSheet";
import { Button } from "@/components/ui/button";
import { Field, FieldError, FieldGroup, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { Spinner } from "@/components/ui/spinner";
import { Textarea } from "@/components/ui/textarea";
import { api, ApiError, dropField, errorText } from "@/lib/api";
import { useProject } from "@/lib/project";
import { between } from "@/lib/status";
import { clockDiffers, duration, fromLocalInputValue, localInputValue, nowSecs, windowHint } from "@/lib/time";
import type { IncidentItem, MaintenanceWindow } from "@/lib/types";
import { useApi } from "@/lib/useApi";
import { ItemPicker } from "@/pages/project/IncidentForm";
import { toast } from "sonner";

/** Edit a window: loads it, then opens the form on it. */
export function EditWindow({ base, id, onClose, onSaved }: { base: string; id: string; onClose: () => void; onSaved: () => void }): ReactElement {
  const w = useApi<MaintenanceWindow>(`${base}/${id}`);
  if (w.error) return <p className="p-6 text-sm text-destructive">{w.error.message}</p>;
  if (!w.data) return <Skeleton className="m-6 h-[480px] rounded-xl" />;
  return <WindowForm key={w.data.id} base={base} initial={w.data} onClose={onClose} onSaved={onSaved} />;
}

/** Plan a window: a title, a start and an end on the user's own clock, the items, and a note for visitors.
 * With `initial` it edits that window (Q50); an active one keeps its start and items. */
export function WindowForm({ base, initial, onClose, onSaved }: { base: string; initial?: MaintenanceWindow; onClose: () => void; onSaved: () => void }): ReactElement {
  const { project: p } = useProject();
  const choices = useApi<IncidentItem[]>(`/projects/${p.slug}/incident-items`);
  const active = initial?.state === "active";
  const [title, setTitle] = useState(initial?.title ?? "");
  // People type times on their own clock; the hint says how Monita's time zone shows them.
  const [starts, setStarts] = useState(() => localInputValue(initial?.starts_at ?? nowSecs()));
  const [ends, setEnds] = useState(() => localInputValue(initial?.ends_at ?? nowSecs() + 3600));
  // "Plan maintenance" on a monitor or heartbeat opens the form with it picked: ?item=monitor:3.
  const [params] = useSearchParams();
  const [items, setItems] = useState<string[]>(() => initial?.items.map((i) => i.value) ?? params.getAll("item"));
  const [message, setMessage] = useState(initial?.message ?? "");
  const [error, setError] = useState<ApiError | null>(null);
  const [busy, setBusy] = useState(false);
  const err = (k: string): string | undefined => error?.fields[k];
  const clear = (k: string): void => setError((e) => e && dropField(e, k));

  const submit = async (_e: FormEvent): Promise<void> => {
    setBusy(true);
    setError(null);
    try {
      const body = { title, message, starts_at: fromLocalInputValue(starts), ends_at: fromLocalInputValue(ends), items };
      await (initial ? api(`${base}/${initial.id}`, "PUT", body) : api(base, "POST", body));
      toast.success(initial ? "Window saved." : "Maintenance planned.");
      onSaved();
    } catch (e) {
      setError(e instanceof ApiError ? e : new ApiError(0, "error", errorText(e)));
    } finally {
      setBusy(false);
    }
  };
  const formError = error && Object.keys(error.fields).length === 0 ? error.message : undefined;
  const [from, to] = [fromLocalInputValue(starts), fromLocalInputValue(ends)];
  const hint =
    active && to !== null && to > nowSecs()
      ? `It started already, so only its end moves, and it ends in ${duration(Math.round((to - nowSecs()) / 60) * 60)}.`
      : windowHint(from, to, nowSecs(), from !== null && to !== null && clockDiffers(from) ? between(from, to) : null);

  return (
    <FormSheetBody
      title={initial ? "Edit maintenance" : "Plan maintenance"}
      onSubmit={(e) => void submit(e)}
      footer={
        <div className="ml-auto flex gap-2">
          <Button type="button" variant="ghost" onClick={onClose}>
            Cancel
          </Button>
          <Button type="submit" disabled={busy}>
            {busy && <Spinner data-icon="inline-start" />}
            {initial ? "Save window" : "Plan maintenance"}
          </Button>
        </div>
      }
    >
      {!choices.data ? (
        <Skeleton className="h-[480px] rounded-xl" />
      ) : (
        <FieldGroup className="gap-5">
          {formError && (
            <p role="alert" className="text-sm text-destructive">
              {formError}
            </p>
          )}
          <Field data-invalid={err("title") ? true : undefined}>
            <FieldLabel htmlFor="w-title" required>
              Title
            </FieldLabel>
            <Input id="w-title" autoFocus maxLength={120} aria-required aria-invalid={err("title") ? true : undefined} placeholder="Database upgrade" value={title} onChange={(e) => {
              setTitle(e.target.value);
              clear("title");
            }} />
            {err("title") && <FieldError>{err("title")}</FieldError>}
          </Field>
          <div className="grid gap-5 sm:grid-cols-2">
            <Field data-invalid={err("starts_at") ? true : undefined}>
              <FieldLabel htmlFor="w-starts" required>
                Starts
              </FieldLabel>
              <Input id="w-starts" type="datetime-local" disabled={active} aria-required aria-invalid={err("starts_at") ? true : undefined} value={starts} onChange={(e) => {
                setStarts(e.target.value);
                clear("starts_at");
              }} />
              {err("starts_at") && <FieldError>{err("starts_at")}</FieldError>}
            </Field>
            <Field data-invalid={err("ends_at") ? true : undefined}>
              <FieldLabel htmlFor="w-ends" required>
                Ends
              </FieldLabel>
              <Input id="w-ends" type="datetime-local" aria-required aria-invalid={err("ends_at") ? true : undefined} value={ends} onChange={(e) => {
                setEnds(e.target.value);
                clear("ends_at");
              }} />
              {err("ends_at") && <FieldError>{err("ends_at")}</FieldError>}
            </Field>
            {hint && <p className="text-sm text-muted-foreground sm:col-span-2">{hint}</p>}
          </div>
          {active ? (
            <div className="flex flex-col gap-1.5 text-sm">
              <span className="font-medium">Affected monitors and heartbeats</span>
              <span>{initial.items.map((i) => i.name).join(", ") || "Its items were deleted."}</span>
              <span className="text-muted-foreground">An active window keeps its items, because the checks it already holds count as maintenance.</span>
            </div>
          ) : (
            <ItemPicker
              choices={choices.data}
              chosen={items}
              onToggle={(value, on) => {
                setItems((xs) => (on ? [...xs, value] : xs.filter((x) => x !== value)));
                clear("items");
              }}
              error={err("items")}
            />
          )}
          <Field data-invalid={err("message") ? true : undefined}>
            <FieldLabel htmlFor="w-message">Message on the status page</FieldLabel>
            <Textarea id="w-message" rows={4} maxLength={1000} aria-invalid={err("message") ? true : undefined} placeholder="What visitors may notice, and when it is back." value={message} onChange={(e) => {
              setMessage(e.target.value);
              clear("message");
            }} />
            {err("message") && <FieldError>{err("message")}</FieldError>}
          </Field>
        </FieldGroup>
      )}
    </FormSheetBody>
  );
}
