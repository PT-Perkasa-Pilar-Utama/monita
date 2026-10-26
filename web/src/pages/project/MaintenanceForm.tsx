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
import { clockDiffers, fromLocalInputValue, localInputValue, nowSecs, windowHint } from "@/lib/time";
import type { IncidentItem } from "@/lib/types";
import { useApi } from "@/lib/useApi";
import { ItemPicker } from "@/pages/project/IncidentForm";
import { toast } from "sonner";

/** Plan a window: a title, a start and an end on the user's own clock, the items, and a note for visitors. */
export function WindowForm({ base, onClose, onSaved }: { base: string; onClose: () => void; onSaved: () => void }): ReactElement {
  const { project: p } = useProject();
  const choices = useApi<IncidentItem[]>(`/projects/${p.slug}/incident-items`);
  const [title, setTitle] = useState("");
  // People type times on their own clock; the hint says how Monita's time zone shows them.
  const [starts, setStarts] = useState(() => localInputValue(nowSecs()));
  const [ends, setEnds] = useState(() => localInputValue(nowSecs() + 3600));
  // "Plan maintenance" on a monitor or heartbeat opens the form with it picked: ?item=monitor:3.
  const [params] = useSearchParams();
  const [items, setItems] = useState<string[]>(() => params.getAll("item"));
  const [message, setMessage] = useState("");
  const [error, setError] = useState<ApiError | null>(null);
  const [busy, setBusy] = useState(false);
  const err = (k: string): string | undefined => error?.fields[k];
  const clear = (k: string): void => setError((e) => e && dropField(e, k));

  const submit = async (_e: FormEvent): Promise<void> => {
    setBusy(true);
    setError(null);
    try {
      await api(base, "POST", { title, message, starts_at: fromLocalInputValue(starts), ends_at: fromLocalInputValue(ends), items });
      toast.success("Maintenance planned.");
      onSaved();
    } catch (e) {
      setError(e instanceof ApiError ? e : new ApiError(0, "error", errorText(e)));
    } finally {
      setBusy(false);
    }
  };
  const formError = error && Object.keys(error.fields).length === 0 ? error.message : undefined;
  const [from, to] = [fromLocalInputValue(starts), fromLocalInputValue(ends)];
  const hint = windowHint(from, to, nowSecs(), from !== null && to !== null && clockDiffers(from) ? between(from, to) : null);

  return (
    <FormSheetBody
      title="Plan maintenance"
      onSubmit={(e) => void submit(e)}
      footer={
        <div className="ml-auto flex gap-2">
          <Button type="button" variant="ghost" onClick={onClose}>
            Cancel
          </Button>
          <Button type="submit" disabled={busy}>
            {busy && <Spinner data-icon="inline-start" />}
            Plan maintenance
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
              <Input id="w-starts" type="datetime-local" aria-required aria-invalid={err("starts_at") ? true : undefined} value={starts} onChange={(e) => {
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
          <ItemPicker
            choices={choices.data}
            chosen={items}
            onToggle={(value, on) => {
              setItems((xs) => (on ? [...xs, value] : xs.filter((x) => x !== value)));
              clear("items");
            }}
            error={err("items")}
          />
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
