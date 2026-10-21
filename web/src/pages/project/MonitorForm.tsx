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
import { headerText, parseHeaderLines, watchesCert } from "@/lib/checks";
import { useProject } from "@/lib/project";
import type { CheckOptions, MonitorDetail, Secret, Severity } from "@/lib/types";
import { useApi } from "@/lib/useApi";
import { toast } from "sonner";

type Form = RequestPart & {
  name: string;
  expected_address: string;
  expected_status: string;
  keyword: string;
  skip_tls_verify: boolean;
  severity: Severity;
  is_public: boolean;
  alert: boolean;
  auto_incident: boolean;
  fails_to_open: string;
  /** `7`, `14` or `30`; empty is off. */
  cert_warn_days: string;
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
            name: m.name, kind: m.kind, url: m.url, expected_address: m.expected_address ?? "", interval_s: m.interval_s, timeout_s: String(m.timeout_s), expected_status: m.expected_status, keyword: m.keyword ?? "", skip_tls_verify: m.skip_tls_verify, severity: m.severity, is_public: m.is_public, alert: m.alert, auto_incident: m.auto_incident, method: m.method, body: m.body ?? "", fails_to_open: String(m.fails_to_open), cert_warn_days: m.cert_warn_days ? String(m.cert_warn_days) : "",
            header_rows: parseHeaderLines(m.headers).map(([name, value]) => ({ name, value })),
            auth_kind: m.auth_kind, auth_secret_id: m.auth_secret_id, auth_user: m.auth_user ?? "", auth_header: m.auth_header ?? "",
          }
        : {
            name: "", kind: "http", url: "", expected_address: "", interval_s: o.monitor.interval_s, timeout_s: String(o.monitor.timeout_s), expected_status: o.monitor.expected_status, keyword: "", skip_tls_verify: false, severity: o.monitor.severity, is_public: false, alert: true, auto_incident: true, method: "GET", body: "", fails_to_open: "2", cert_warn_days: "",
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
    const body = { ...rest, headers: headerText(header_rows), interval_s: String(form.interval_s), fails_to_open: form.fails_to_open, body: sendsBody(form.method) ? form.body : "", cert_warn_days: watchesCert(form.kind, form.url) ? form.cert_warn_days : "" };
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
                  {form.kind === "dns" && (
                    <>
                      <FieldSet className="gap-3">
                        <FieldLegend>A check passes when</FieldLegend>
                        <Field data-invalid={err("expected_address") ? true : undefined}>
                          <FieldLabel htmlFor="m-address" className="font-normal">
                            The name resolves to
                          </FieldLabel>
                          <Input id="m-address" spellCheck={false} aria-invalid={err("expected_address") ? true : undefined} className="font-mono" placeholder="Any address, or one such as 203.0.113.10" value={form.expected_address} onChange={(e) => set("expected_address", e.target.value)} />
                          {err("expected_address") && <FieldError>{err("expected_address")}</FieldError>}
                        </Field>
                      </FieldSet>
                      <FieldSeparator />
                    </>
                  )}
                  {form.kind === "http" && (
                  <>
                  <AuthFields {...parts} />
                  <FieldSeparator />
                  <HeaderFields {...parts} />
                  <FieldSeparator />
                  <FieldSet className="gap-3">
                    <FieldLegend>A check passes when</FieldLegend>
                    <div className="grid gap-5 sm:grid-cols-[minmax(0,1fr)_minmax(0,2fr)]">
                      <Field data-invalid={err("expected_status") ? true : undefined}>
                        <FieldLabel htmlFor="m-status" className="font-normal" required>
                          Status code
                        </FieldLabel>
                        <Input id="m-status" aria-required aria-invalid={err("expected_status") ? true : undefined} className="font-mono" placeholder="2xx, or 200,204" value={form.expected_status} onChange={(e) => set("expected_status", e.target.value)} />
                        {err("expected_status") && <FieldError>{err("expected_status")}</FieldError>}
                      </Field>
                      <Field data-invalid={err("keyword") ? true : undefined}>
                        <FieldLabel htmlFor="m-keyword" className="font-normal">
                          Response body contains
                        </FieldLabel>
                        <Input id="m-keyword" maxLength={200} aria-invalid={err("keyword") ? true : undefined} className="font-mono" placeholder='"status":"ok"' value={form.keyword} onChange={(e) => set("keyword", e.target.value)} />
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
                  </>
                  )}
                  <FieldSet className="gap-7">
                  <FieldLegend>Alerts and status page</FieldLegend>
                  <SeverityField value={form.severity} onChange={(v) => set("severity", v)} />
                  <WhenItFails id="m" alert={form.alert} autoIncident={form.auto_incident} onChange={set}>
                    <Field data-invalid={err("fails_to_open") ? true : undefined}>
                      <FieldLabel htmlFor="m-fails" className="font-normal">
                        It counts as down after
                      </FieldLabel>
                      <div className="flex items-center gap-3">
                        <Input
                          id="m-fails"
                          type="number"
                          inputMode="numeric"
                          min={options.data.fails.min}
                          max={options.data.fails.max}
                          className="w-24"
                          aria-invalid={err("fails_to_open") ? true : undefined}
                          value={form.fails_to_open}
                          onChange={(e) => set("fails_to_open", e.target.value)}
                          placeholder="2"
                        />
                        <span className="text-sm text-muted-foreground">{form.fails_to_open.trim() === "1" ? "failed check" : "failed checks in a row"}</span>
                      </div>
                      {err("fails_to_open") && <FieldError>{err("fails_to_open")}</FieldError>}
                    </Field>
                  </WhenItFails>
                  {watchesCert(form.kind, form.url) && (
                    <Field data-invalid={err("cert_warn_days") ? true : undefined}>
                      <FieldLabel htmlFor="m-cert" className="font-normal">
                        Warn before the certificate expires
                      </FieldLabel>
                      <Select value={form.cert_warn_days || "off"} onValueChange={(v) => set("cert_warn_days", v === "off" ? "" : v)}>
                        <SelectTrigger id="m-cert" className="w-56" aria-invalid={err("cert_warn_days") ? true : undefined}>
                          <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                          <SelectGroup>
                            <SelectItem value="off">Off</SelectItem>
                            {["7", "14", "30"].map((d) => (
                              <SelectItem key={d} value={d}>{`${d} days before`}</SelectItem>
                            ))}
                          </SelectGroup>
                        </SelectContent>
                      </Select>
                      {err("cert_warn_days") && <FieldError>{err("cert_warn_days")}</FieldError>}
                    </Field>
                  )}
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
