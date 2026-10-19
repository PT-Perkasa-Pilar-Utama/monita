import { useEffect, useState } from "react";
import type { FormEvent, ReactElement } from "react";
import { useNavigate } from "react-router-dom";
import { ChoiceField, PublicField, SeverityField, WhenItFails } from "@/components/app/CheckForm";
import { FormSheet, FormSheetBody } from "@/components/app/FormSheet";
import { HeartbeatGuide } from "@/components/app/Guides";
import { ConfirmDialog } from "@/components/app/ConfirmDialog";
import { Trash2 } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { Field, FieldError, FieldGroup, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { Spinner } from "@/components/ui/spinner";
import { api, ApiError, errorText } from "@/lib/api";
import { useProject } from "@/lib/project";
import type { CheckOptions, HeartbeatDetail, Severity } from "@/lib/types";
import { useApi } from "@/lib/useApi";
import { toast } from "sonner";

type Form = { name: string; period_s: number; grace_s: number; severity: Severity; is_public: boolean; alert: boolean; auto_incident: boolean };


/** New heartbeat, or edit one when the URL names it. Edit also deletes. */
function HeartbeatFormBody({ id, onClose, onSaved }: { id: string | undefined; onClose: () => void; onSaved: () => void }): ReactElement {
  const { project: p, reload } = useProject();
  const navigate = useNavigate();
  const base = `/projects/${p.slug}/heartbeats`;
  const options = useApi<CheckOptions>("/check-options");
  const current = useApi<HeartbeatDetail>(id ? `${base}/${encodeURIComponent(id)}?range=hour` : null);
  const [form, setForm] = useState<Form | null>(null);
  const [error, setError] = useState<ApiError | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (form) return;
    const o = options.data;
    if (!o) return;
    const h = current.data?.heartbeat;
    if (id && !h) return;
    setForm(h ? { name: h.name, period_s: h.period_s, grace_s: h.grace_s, severity: h.severity, is_public: h.is_public, alert: h.alert, auto_incident: h.auto_incident } : { name: "", ...o.heartbeat, is_public: false, alert: true, auto_incident: true });
  }, [form, id, options.data, current.data]);

  const set = <K extends keyof Form>(k: K, v: Form[K]): void => setForm((f) => (f ? { ...f, [k]: v } : f));
  const err = (k: string): string | undefined => error?.fields[k];

  const submit = async (_e: FormEvent): Promise<void> => {
    if (!form) return;
    setBusy(true);
    setError(null);
    const body = { ...form, period_s: String(form.period_s), grace_s: String(form.grace_s) };
    try {
      if (id) {
        await api(`${base}/${id}`, "PUT", body);
        toast.success("Heartbeat saved.");
        onSaved();
        navigate(`${base}/${id}`);
      } else {
        const made = await api<{ id: number }>(base, "POST", body);
        reload();
        navigate(`${base}/${made.id}`);
      }
    } catch (err) {
      setError(err instanceof ApiError ? err : new ApiError(0, "error", errorText(err)));
    } finally {
      setBusy(false);
    }
  };

  return (
    <FormSheetBody
      title={id ? "Edit heartbeat" : "New heartbeat"}
      guide={<HeartbeatGuide />}
      onSubmit={(e) => void submit(e)}
      footer={<>
                {id && (
                  <ConfirmDialog
                    trigger={
                      <Button type="button" variant="danger">
                        <Trash2 data-icon="inline-start" />
                        Delete heartbeat and its pings
                      </Button>
                    }
                    title="Delete this heartbeat?"
                    description="Monita deletes the heartbeat and every ping it got. A job that still calls its URL gets 404. You cannot undo this."
                    action="Delete heartbeat"
                    onConfirm={async () => {
                      await api(`${base}/${id}`, "DELETE");
                      reload();
                      navigate(base);
                    }}
                  />
                )}
                <div className="ml-auto flex gap-2">
                  <Button type="button" variant="ghost" onClick={onClose}>
                    Cancel
                  </Button>
                  <Button type="submit" disabled={busy}>
                    {busy && <Spinner data-icon="inline-start" />}
                    Save heartbeat
                  </Button>
                </div>
              </>}
    >
      {current.error?.status === 404 ? (
        <p className="text-sm text-muted-foreground">This heartbeat does not exist.</p>
      ) : !form || !options.data ? (
        <Skeleton className="h-[420px] rounded-xl" />
      ) : (
                <FieldGroup className="gap-5">
                  <Field data-invalid={err("name") ? true : undefined}>
                    <FieldLabel htmlFor="h-name" required>
                      Name
                    </FieldLabel>
                    <Input id="h-name" autoFocus={!id} maxLength={80} aria-required aria-invalid={err("name") ? true : undefined} value={form.name} onChange={(e) => set("name", e.target.value)} placeholder="nightly-backup" />
                    {err("name") && <FieldError>{err("name")}</FieldError>}
                  </Field>
                  <div className="grid gap-5 sm:grid-cols-2">
                    <ChoiceField id="h-period" label="Runs every" value={form.period_s} choices={options.data.periods} onChange={(v) => set("period_s", v)} error={err("period_s")} />
                    <ChoiceField id="h-grace" label="Can be late by" value={form.grace_s} choices={options.data.graces} onChange={(v) => set("grace_s", v)} error={err("grace_s")} />
                  </div>
                  <SeverityField value={form.severity} onChange={(v) => set("severity", v)} />
                  <WhenItFails id="h" alert={form.alert} autoIncident={form.auto_incident} onChange={set} />
                  <PublicField id="h" noun="heartbeat" checked={form.is_public} onChange={(v) => set("is_public", v)} />
                </FieldGroup>
              
      )}
    </FormSheetBody>
  );
}

/** New heartbeat, or edit one when `id` names it, in the wide panel. Edit also deletes. */
export function HeartbeatFormSheet({ open, id, onClose, onSaved = () => undefined }: { open: boolean; id?: string | undefined; onClose: () => void; onSaved?: () => void }): ReactElement {
  return (
    <FormSheet open={open} onClose={onClose}>
      <HeartbeatFormBody id={id} onClose={onClose} onSaved={onSaved} />
    </FormSheet>
  );
}
