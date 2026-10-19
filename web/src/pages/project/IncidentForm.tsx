import { useState } from "react";
import type { FormEvent, ReactElement } from "react";
import { useNavigate } from "react-router-dom";
import { SeverityField } from "@/components/app/CheckForm";
import { StateBadge } from "@/components/app/Tone";
import { FormSheet, FormSheetBody } from "@/components/app/FormSheet";
import { IncidentGuide } from "@/components/app/Guides";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Field, FieldError, FieldGroup, FieldLabel, FieldLegend, FieldSet } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { Spinner } from "@/components/ui/spinner";
import { Textarea } from "@/components/ui/textarea";
import { api, ApiError, dropField, errorText } from "@/lib/api";
import { countLabel } from "@/lib/filter";
import { pickItems } from "@/lib/incidentItems";
import { useProject } from "@/lib/project";
import type { IncidentItem, Severity } from "@/lib/types";
import { useApi } from "@/lib/useApi";

/** One kind's checkboxes under its count, scrolling past about eight rows. */
function ItemList({ items, total, noun, chosen, onToggle }: { items: IncidentItem[]; total: number; noun: string; chosen: string[]; onToggle: (value: string, on: boolean) => void }): ReactElement {
  return (
    <FieldSet className="gap-2">
      <FieldLegend variant="label">{countLabel(items.length, total, noun)}</FieldLegend>
      {items.length === 0 ? (
        <p className="text-sm text-muted-foreground">{total === 0 ? `This project has no ${noun}s.` : `No ${noun} matches.`}</p>
      ) : (
        <div className="flex max-h-64 flex-col gap-2 overflow-y-auto rounded-lg border p-3">
          {items.map((c) => {
            const id = `item-${c.value.replace(":", "-")}`;
            return (
              <Field key={c.value} orientation="horizontal">
                <Checkbox id={id} checked={chosen.includes(c.value)} onCheckedChange={(v) => onToggle(c.value, v === true)} />
                <FieldLabel htmlFor={id} className="min-w-0 font-normal">
                  <span className="truncate">{c.name}</span>
                  {c.state && (
                    <>
                      {" "}
                      <StateBadge kind={c.state} />
                    </>
                  )}
                </FieldLabel>
              </Field>
            );
          })}
        </div>
      )}
    </FieldSet>
  );
}

/** Affected items: one search over a Monitors list and a Heartbeats list, with what is already broken marked. */
function ItemPicker({ choices, chosen, onToggle, error }: { choices: IncidentItem[]; chosen: string[]; onToggle: (value: string, on: boolean) => void; error?: string | undefined }): ReactElement {
  const [q, setQ] = useState("");
  const { monitors, heartbeats } = pickItems(choices, q);
  const all = pickItems(choices, "");
  const names = choices.filter((c) => chosen.includes(c.value)).map((c) => c.name);
  return (
    <FieldSet className="gap-3" data-invalid={error ? true : undefined}>
      <FieldLegend variant="label">
        Affected monitors and heartbeats <span aria-hidden="true" className="text-destructive">*</span>
      </FieldLegend>
      {choices.length === 0 ? (
        <p className="text-sm text-muted-foreground">Add a monitor or heartbeat first.</p>
      ) : (
        <>
          <Field>
            <FieldLabel htmlFor="i-find" className="font-normal">
              Find a monitor or heartbeat by name
            </FieldLabel>
            <Input id="i-find" type="search" value={q} onChange={(e) => setQ(e.target.value)} />
          </Field>
          {names.length > 0 && <p className="text-sm text-muted-foreground">Chosen: {names.join(", ")}.</p>}
          <div className="mt-2 grid gap-4 sm:grid-cols-2">
            <ItemList items={monitors} total={all.monitors.length} noun="monitor" chosen={chosen} onToggle={onToggle} />
            <ItemList items={heartbeats} total={all.heartbeats.length} noun="heartbeat" chosen={chosen} onToggle={onToggle} />
          </div>
        </>
      )}
      {error && <FieldError>{error}</FieldError>}
    </FieldSet>
  );
}

/** Report an incident by hand: a title, the items it affects, a severity and the first update. */
function IncidentFormBody({ onClose }: { onClose: () => void }): ReactElement {
  const { project: p, reload } = useProject();
  const navigate = useNavigate();
  const base = `/projects/${p.slug}/incidents`;
  const choices = useApi<IncidentItem[]>(`/projects/${p.slug}/incident-items`);
  const [title, setTitle] = useState("");
  const [items, setItems] = useState<string[]>([]);
  const [severity, setSeverity] = useState<Severity>("warning");
  const [message, setMessage] = useState("");
  const [error, setError] = useState<ApiError | null>(null);
  const [busy, setBusy] = useState(false);
  const err = (k: string): string | undefined => error?.fields[k];

  const submit = async (_e: FormEvent): Promise<void> => {
    setBusy(true);
    setError(null);
    try {
      const made = await api<{ id: number }>(base, "POST", { title, items, severity, message });
      reload();
      navigate(`${base}/${made.id}`);
    } catch (err) {
      setError(err instanceof ApiError ? err : new ApiError(0, "error", errorText(err)));
    } finally {
      setBusy(false);
    }
  };
  const clear = (field: string): void => setError((e) => e && dropField(e, field));
  const toggle = (value: string, on: boolean): void => {
    setItems((xs) => (on ? [...xs, value] : xs.filter((x) => x !== value)));
    clear("items");
  };
  const formError = error && Object.keys(error.fields).length === 0 ? error.message : undefined;

  return (
    <FormSheetBody title="Report an incident" guide={<IncidentGuide />} onSubmit={(e) => void submit(e)} footer={<>
          <div className="ml-auto flex gap-2">
            <Button type="button" variant="ghost" onClick={onClose}>
              Cancel
            </Button>
            <Button type="submit" disabled={busy}>
              {busy && <Spinner data-icon="inline-start" />}
              Open incident
            </Button>
          </div>
        </>}>
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
                    <FieldLabel htmlFor="i-title" required>
                      Title
                    </FieldLabel>
                    <Input id="i-title" autoFocus maxLength={120} aria-required aria-invalid={err("title") ? true : undefined} value={title} onChange={(e) => {
                      setTitle(e.target.value);
                      clear("title");
                    }} />
                    {err("title") && <FieldError>{err("title")}</FieldError>}
                  </Field>
                  <ItemPicker choices={choices.data} chosen={items} onToggle={toggle} error={err("items")} />
                  <SeverityField value={severity} onChange={setSeverity} />
                  <Field data-invalid={err("message") ? true : undefined}>
                    <FieldLabel htmlFor="i-message" required>
                      First update
                    </FieldLabel>
                    <Textarea
                      id="i-message"
                      rows={5}
                      aria-required
                      aria-invalid={err("message") ? true : undefined}
                      placeholder="What users see, and what you know so far."
                      value={message}
                      onChange={(e) => {
                        setMessage(e.target.value);
                        clear("message");
                      }}
                    />
                    {err("message") && <FieldError>{err("message")}</FieldError>}
                  </Field>
                </FieldGroup>
              
      )}
    </FormSheetBody>
  );
}

/** Report an incident by hand, in the wide panel over the incident list. */
export function IncidentFormSheet({ open, onClose }: { open: boolean; onClose: () => void }): ReactElement {
  return (
    <FormSheet open={open} onClose={onClose}>
      <IncidentFormBody onClose={onClose} />
    </FormSheet>
  );
}
