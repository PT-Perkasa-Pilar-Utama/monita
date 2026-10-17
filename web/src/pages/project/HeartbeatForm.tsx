import { useEffect, useState } from "react";
import type { FormEvent, ReactElement } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";
import { ChoiceField, FormAside, SeverityField } from "@/components/app/CheckForm";
import { ConfirmDialog } from "@/components/app/ConfirmDialog";
import { Trash2 } from "@/components/icons";
import { Crumbs, PageHeader } from "@/components/layout/Page";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardFooter } from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
import { Field, FieldError, FieldGroup, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { Spinner } from "@/components/ui/spinner";
import { api, ApiError, errorText } from "@/lib/api";
import { useProject } from "@/lib/project";
import type { CheckOptions, HeartbeatDetail, Severity } from "@/lib/types";
import { useApi } from "@/lib/useApi";
import { NotFound } from "@/pages/NotFound";
import { toast } from "sonner";

type Form = { name: string; period_s: number; grace_s: number; severity: Severity; is_public: boolean };

const STEPS = [
  "After you save, Monita gives the heartbeat a secret ping URL.",
  "Your job calls that URL when it finishes. Each call moves the deadline to the time of the call plus the period and the grace time.",
  "If no call arrives by the deadline, Monita opens an incident. The next call closes it.",
  "A call to the URL with /fail at the end opens an incident at once.",
];

/** New heartbeat, or edit one when the URL names it. Edit also deletes. */
export function HeartbeatForm(): ReactElement {
  const { project: p, reload } = useProject();
  const { id } = useParams();
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
    setForm(h ? { name: h.name, period_s: h.period_s, grace_s: h.grace_s, severity: h.severity, is_public: h.is_public } : { name: "", ...o.heartbeat, is_public: false });
  }, [form, id, options.data, current.data]);

  if (current.error?.status === 404) return <NotFound />;
  const set = <K extends keyof Form>(k: K, v: Form[K]): void => setForm((f) => (f ? { ...f, [k]: v } : f));
  const err = (k: string): string | undefined => error?.fields[k];

  const submit = async (e: FormEvent): Promise<void> => {
    e.preventDefault();
    if (!form) return;
    setBusy(true);
    setError(null);
    const body = { ...form, period_s: String(form.period_s), grace_s: String(form.grace_s) };
    try {
      if (id) {
        await api(`${base}/${id}`, "PUT", body);
        toast.success("Heartbeat saved.");
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
    <>
      <Crumbs items={[{ label: "Heartbeats", to: base }, ...(id && current.data ? [{ label: current.data.heartbeat.name, to: `${base}/${id}` }] : []), { label: id ? "Edit" : "New" }]} />
      <PageHeader title={id ? "Edit heartbeat" : "New heartbeat"} />
      <div className="grid items-start gap-6 xl:grid-cols-[minmax(0,2fr)_minmax(280px,1fr)]">
        {!form || !options.data ? (
          <Skeleton className="h-[420px] rounded-xl" />
        ) : (
          <Card>
            <form onSubmit={(e) => void submit(e)} noValidate className="flex flex-col gap-6">
              <CardContent>
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
                    <ChoiceField id="h-grace" label="Grace time" value={form.grace_s} choices={options.data.graces} onChange={(v) => set("grace_s", v)} error={err("grace_s")} />
                  </div>
                  <SeverityField value={form.severity} onChange={(v) => set("severity", v)} />
                  <Field orientation="horizontal">
                    <Checkbox id="h-public" checked={form.is_public} onCheckedChange={(v) => set("is_public", v === true)} />
                    <FieldLabel htmlFor="h-public" className="font-normal">
                      Show on the public status page
                    </FieldLabel>
                  </Field>
                </FieldGroup>
              </CardContent>
              <CardFooter className="flex-wrap gap-2 border-t">
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
                  <Button asChild variant="ghost">
                    <Link to={id ? `${base}/${id}` : base}>Cancel</Link>
                  </Button>
                  <Button type="submit" disabled={busy}>
                    {busy && <Spinner data-icon="inline-start" />}
                    Save heartbeat
                  </Button>
                </div>
              </CardFooter>
            </form>
          </Card>
        )}
        <FormAside title="How a heartbeat works" steps={STEPS} note="The first deadline is the period plus the grace time from the moment you save." />
      </div>
    </>
  );
}
