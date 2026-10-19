import { useEffect, useState } from "react";
import type { FormEvent, ReactElement } from "react";
import { useNavigate } from "react-router-dom";
import { PublicField, SeverityField, WhenItFails } from "@/components/app/CheckForm";
import { FormSheet, FormSheetBody } from "@/components/app/FormSheet";
import { MonitorGuide } from "@/components/app/Guides";
import { ConfirmDialog } from "@/components/app/ConfirmDialog";
import { AuthFields, HeaderFields, RequestFields, sendsBody } from "@/components/app/MonitorRequest";
import type { RequestPart } from "@/components/app/MonitorRequest";
import { Trash2 } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Field, FieldError, FieldGroup, FieldLabel, FieldLegend, FieldSeparator, FieldSet } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Select, SelectContent, SelectGroup, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Skeleton } from "@/components/ui/skeleton";
import { Spinner } from "@/components/ui/spinner";
import { api, ApiError, dropField, errorText } from "@/lib/api";
import { headerText, parseHeaderLines } from "@/lib/checks";
import { useProject } from "@/lib/project";
import type { CheckOptions, MonitorDetail, Secret, Severity } from "@/lib/types";
import { useApi } from "@/lib/useApi";
import { toast } from "sonner";

type Form = RequestPart & {
  name: string;
  expected_status: string;
  keyword: string;
  skip_tls_verify: boolean;
  severity: Severity;
  is_public: boolean;
  alert: boolean;
  auto_incident: boolean;
  fails_to_open: number;
};

/** New monitor, or edit one when the URL names it. Edit also deletes. */
function MonitorFormBody({ id, onClose, onSaved }: { id: string | undefined; onClose: () => void; onSaved: () => void }): ReactElement {
  const { project: p, reload } = useProject();
  const navigate = useNavigate();
  const base = `/projects/${p.slug}/monitors`;
  const options = useApi<CheckOptions>("/check-options");
  const current = useApi<MonitorDetail>(id ? `${base}/${encodeURIComponent(id)}?range=hour` : null);
  const secrets = useApi<Secret[]>(`/projects/${p.slug}/secrets`);
  const [form, setForm] = useState<Form | null>(null);
  const [error, setError] = useState<ApiError | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (form) return;
    const o = options.data;
    if (!o) return;
    const m = current.data?.monitor;
    if (id && !m) return;
    setForm(
      m
        ? {
            name: m.name, url: m.url, interval_s: m.interval_s, timeout_s: String(m.timeout_s), expected_status: m.expected_status, keyword: m.keyword ?? "", skip_tls_verify: m.skip_tls_verify, severity: m.severity, is_public: m.is_public, alert: m.alert, auto_incident: m.auto_incident, method: m.method, body: m.body ?? "", fails_to_open: m.fails_to_open,
            header_rows: parseHeaderLines(m.headers).map(([name, value]) => ({ name, value })),
            auth_kind: m.auth_kind, auth_secret_id: m.auth_secret_id, auth_user: m.auth_user ?? "", auth_header: m.auth_header ?? "",
          }
        : {
            name: "", url: "", interval_s: o.monitor.interval_s, timeout_s: String(o.monitor.timeout_s), expected_status: o.monitor.expected_status, keyword: "", skip_tls_verify: false, severity: o.monitor.severity, is_public: false, alert: true, auto_incident: true, method: "GET", body: "", fails_to_open: 2,
            header_rows: [], auth_kind: "none", auth_secret_id: null, auth_user: "", auth_header: "",
          },
    );
  }, [form, id, options.data, current.data]);

  const set = <K extends keyof Form>(k: K, v: Form[K]): void => {
    setForm((f) => (f ? { ...f, [k]: v } : f));
    setError((e) => e && dropField(e, k === "header_rows" ? "headers" : String(k)));
  };
  const err = (k: string): string | undefined => error?.fields[k];

  const submit = async (_e: FormEvent): Promise<void> => {
    if (!form) return;
    setBusy(true);
    setError(null);
    // A body typed for POST stays in the box after a switch to GET, but only POST and PUT send it.
    const { header_rows, ...rest } = form;
    const body = { ...rest, headers: headerText(header_rows), interval_s: String(form.interval_s), fails_to_open: String(form.fails_to_open), body: sendsBody(form.method) ? form.body : "" };
    try {
      if (id) {
        await api(`${base}/${id}`, "PUT", body);
        toast.success("Monitor saved.");
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

  const parts = form && options.data ? { form, set, err, intervals: options.data.intervals, secrets: secrets.data ?? null, secretsHref: `/projects/${p.slug}/settings/secrets` } : null;

  return (
    <FormSheetBody
      title={id ? "Edit monitor" : "New monitor"}
      guide={<MonitorGuide />}
      onSubmit={(e) => void submit(e)}
      footer={<>
                {id && (
                  <ConfirmDialog
                    trigger={
                      <Button type="button" variant="danger">
                        <Trash2 data-icon="inline-start" />
                        Delete monitor and its checks
                      </Button>
                    }
                    title="Delete this monitor?"
                    description="Monita deletes the monitor and every check it ran. You cannot undo this."
                    action="Delete monitor"
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
                    Save monitor
                  </Button>
                </div>
              </>}
    >
      {current.error?.status === 404 ? (
        <p className="text-sm text-muted-foreground">This monitor does not exist.</p>
      ) : !form || !options.data || !parts ? (
        <Skeleton className="h-[420px] rounded-xl" />
      ) : (
                <FieldGroup className="gap-5">
                  <Field data-invalid={err("name") ? true : undefined}>
                    <FieldLabel htmlFor="m-name" required>
                      Name
                    </FieldLabel>
                    <Input id="m-name" autoFocus={!id} maxLength={80} aria-required aria-invalid={err("name") ? true : undefined} value={form.name} onChange={(e) => set("name", e.target.value)} placeholder="Checkout API" />
                    {err("name") && <FieldError>{err("name")}</FieldError>}
                  </Field>
                  <FieldSeparator />
                  <RequestFields {...parts} />
                  <FieldSeparator />
                  <AuthFields {...parts} />
                  <FieldSeparator />
                  <HeaderFields {...parts} />
                  <FieldSeparator />
                  <FieldSet className="gap-3">
                    <FieldLegend>A check passes when</FieldLegend>
                    <div className="grid gap-5 sm:grid-cols-[minmax(0,1fr)_minmax(0,2fr)]">
                      <Field data-invalid={err("expected_status") ? true : undefined}>
                        <FieldLabel htmlFor="m-status" className="font-normal" required>
                          The status code matches
                        </FieldLabel>
                        <Input id="m-status" aria-required aria-invalid={err("expected_status") ? true : undefined} className="font-mono" value={form.expected_status} onChange={(e) => set("expected_status", e.target.value)} />
                        {err("expected_status") && <FieldError>{err("expected_status")}</FieldError>}
                      </Field>
                      <Field data-invalid={err("keyword") ? true : undefined}>
                        <FieldLabel htmlFor="m-keyword" className="font-normal">
                          The response body has this exact text somewhere
                        </FieldLabel>
                        <Input id="m-keyword" maxLength={200} aria-invalid={err("keyword") ? true : undefined} className="font-mono" value={form.keyword} onChange={(e) => set("keyword", e.target.value)} />
                        {err("keyword") && <FieldError>{err("keyword")}</FieldError>}
                      </Field>
                    </div>
                    <Field orientation="horizontal">
                      <Checkbox id="m-tls" checked={form.skip_tls_verify} onCheckedChange={(v) => set("skip_tls_verify", v === true)} />
                      <FieldLabel htmlFor="m-tls" className="font-normal">
                        The certificate may be self-signed
                      </FieldLabel>
                    </Field>
                  </FieldSet>
                  <FieldSeparator />
                  <FieldSet className="gap-7">
                  <FieldLegend>Alerts and status page</FieldLegend>
                  <SeverityField value={form.severity} onChange={(v) => set("severity", v)} />
                  <WhenItFails id="m" alert={form.alert} autoIncident={form.auto_incident} onChange={set}>
                    <Field data-invalid={err("fails_to_open") ? true : undefined} className="max-w-sm">
                      <FieldLabel htmlFor="m-fails" className="font-normal">
                        It counts as down after
                      </FieldLabel>
                      <Select value={String(form.fails_to_open)} onValueChange={(v) => set("fails_to_open", Number(v))}>
                        <SelectTrigger id="m-fails">
                          <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                          <SelectGroup>
                            {options.data.fails.map((n) => (
                              <SelectItem key={n} value={String(n)}>
                                {n === 1 ? "1 failed check" : `${n} failed checks in a row`}
                              </SelectItem>
                            ))}
                          </SelectGroup>
                        </SelectContent>
                      </Select>
                      {err("fails_to_open") && <FieldError>{err("fails_to_open")}</FieldError>}
                    </Field>
                  </WhenItFails>
                  <PublicField id="m" noun="monitor" checked={form.is_public} onChange={(v) => set("is_public", v)} />
                  </FieldSet>
                </FieldGroup>
              
      )}
    </FormSheetBody>
  );
}

/** New monitor, or edit one when `id` names it, in the wide panel. Edit also deletes. */
export function MonitorFormSheet({ open, id, onClose, onSaved = () => undefined }: { open: boolean; id?: string | undefined; onClose: () => void; onSaved?: () => void }): ReactElement {
  return (
    <FormSheet open={open} onClose={onClose}>
      <MonitorFormBody id={id} onClose={onClose} onSaved={onSaved} />
    </FormSheet>
  );
}
