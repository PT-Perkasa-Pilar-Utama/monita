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
import type { CheckOptions, MonitorDetail, Severity } from "@/lib/types";
import { useApi } from "@/lib/useApi";
import { NotFound } from "@/pages/NotFound";
import { toast } from "sonner";

type Form = {
  name: string;
  url: string;
  interval_s: number;
  timeout_s: string;
  expected_status: string;
  keyword: string;
  skip_tls_verify: boolean;
  severity: Severity;
  is_public: boolean;
};

const STEPS = [
  "Monita sends a GET request to the URL at the interval you choose.",
  "A check passes when the status matches the expected status within the timeout. If you set a keyword, the body must also contain it.",
  "Two failed checks in a row open an incident. The first passing check closes it.",
  "A critical monitor alerts by email and webhook. A warning monitor alerts by email only.",
];

/** New monitor, or edit one when the URL names it. Edit also deletes. */
export function MonitorForm(): ReactElement {
  const { project: p, reload } = useProject();
  const { id } = useParams();
  const navigate = useNavigate();
  const base = `/projects/${p.slug}/monitors`;
  const options = useApi<CheckOptions>("/check-options");
  const current = useApi<MonitorDetail>(id ? `${base}/${encodeURIComponent(id)}?range=hour` : null);
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
        ? { name: m.name, url: m.url, interval_s: m.interval_s, timeout_s: String(m.timeout_s), expected_status: m.expected_status, keyword: m.keyword ?? "", skip_tls_verify: m.skip_tls_verify, severity: m.severity, is_public: m.is_public }
        : { name: "", url: "", interval_s: o.monitor.interval_s, timeout_s: String(o.monitor.timeout_s), expected_status: o.monitor.expected_status, keyword: "", skip_tls_verify: false, severity: o.monitor.severity, is_public: false },
    );
  }, [form, id, options.data, current.data]);

  if (current.error?.status === 404) return <NotFound />;
  const set = <K extends keyof Form>(k: K, v: Form[K]): void => setForm((f) => (f ? { ...f, [k]: v } : f));
  const err = (k: string): string | undefined => error?.fields[k];

  const submit = async (e: FormEvent): Promise<void> => {
    e.preventDefault();
    if (!form) return;
    setBusy(true);
    setError(null);
    const body = { ...form, interval_s: String(form.interval_s) };
    try {
      if (id) {
        await api(`${base}/${id}`, "PUT", body);
        toast.success("Monitor saved.");
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

  const title = id ? "Edit monitor" : "New monitor";
  return (
    <>
      <Crumbs items={[{ label: "Monitors", to: base }, ...(id && current.data ? [{ label: current.data.monitor.name, to: `${base}/${id}` }] : []), { label: id ? "Edit" : "New" }]} />
      <PageHeader title={title} />
      <div className="grid items-start gap-6 xl:grid-cols-[minmax(0,2fr)_minmax(280px,1fr)]">
        {!form || !options.data ? (
          <Skeleton className="h-[560px] rounded-xl" />
        ) : (
          <Card>
            <form onSubmit={(e) => void submit(e)} noValidate className="flex flex-col gap-6">
              <CardContent>
                <FieldGroup className="gap-5">
                  <Field data-invalid={err("name") ? true : undefined}>
                    <FieldLabel htmlFor="m-name" required>
                      Name
                    </FieldLabel>
                    <Input id="m-name" autoFocus={!id} maxLength={80} aria-required aria-invalid={err("name") ? true : undefined} value={form.name} onChange={(e) => set("name", e.target.value)} placeholder="Checkout API" />
                    {err("name") && <FieldError>{err("name")}</FieldError>}
                  </Field>
                  <Field data-invalid={err("url") ? true : undefined}>
                    <FieldLabel htmlFor="m-url" required>
                      URL
                    </FieldLabel>
                    <Input id="m-url" type="url" aria-required aria-invalid={err("url") ? true : undefined} className="font-mono" value={form.url} onChange={(e) => set("url", e.target.value)} placeholder="https://api.example.com/healthz" />
                    {err("url") && <FieldError>{err("url")}</FieldError>}
                  </Field>
                  <div className="grid gap-5 sm:grid-cols-3">
                    <ChoiceField id="m-interval" label="Check every" value={form.interval_s} choices={options.data.intervals} onChange={(v) => set("interval_s", v)} error={err("interval_s")} />
                    <Field data-invalid={err("timeout_s") ? true : undefined}>
                      <FieldLabel htmlFor="m-timeout" required>
                        Timeout (seconds)
                      </FieldLabel>
                      <Input id="m-timeout" inputMode="numeric" aria-required aria-invalid={err("timeout_s") ? true : undefined} value={form.timeout_s} onChange={(e) => set("timeout_s", e.target.value)} />
                      {err("timeout_s") && <FieldError>{err("timeout_s")}</FieldError>}
                    </Field>
                    <Field data-invalid={err("expected_status") ? true : undefined}>
                      <FieldLabel htmlFor="m-status" required>
                        Expected status
                      </FieldLabel>
                      <Input id="m-status" aria-required aria-invalid={err("expected_status") ? true : undefined} className="font-mono" value={form.expected_status} onChange={(e) => set("expected_status", e.target.value)} />
                      {err("expected_status") && <FieldError>{err("expected_status")}</FieldError>}
                    </Field>
                  </div>
                  <Field data-invalid={err("keyword") ? true : undefined}>
                    <FieldLabel htmlFor="m-keyword">Keyword the body must contain</FieldLabel>
                    <Input id="m-keyword" maxLength={200} aria-invalid={err("keyword") ? true : undefined} value={form.keyword} onChange={(e) => set("keyword", e.target.value)} />
                    {err("keyword") && <FieldError>{err("keyword")}</FieldError>}
                  </Field>
                  <SeverityField value={form.severity} onChange={(v) => set("severity", v)} />
                  <Field orientation="horizontal">
                    <Checkbox id="m-public" checked={form.is_public} onCheckedChange={(v) => set("is_public", v === true)} />
                    <FieldLabel htmlFor="m-public" className="font-normal">
                      Show on the public status page
                    </FieldLabel>
                  </Field>
                  <Field orientation="horizontal">
                    <Checkbox id="m-tls" checked={form.skip_tls_verify} onCheckedChange={(v) => set("skip_tls_verify", v === true)} />
                    <FieldLabel htmlFor="m-tls" className="font-normal">
                      Accept self-signed certificates
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
                  <Button asChild variant="ghost">
                    <Link to={id ? `${base}/${id}` : base}>Cancel</Link>
                  </Button>
                  <Button type="submit" disabled={busy}>
                    {busy && <Spinner data-icon="inline-start" />}
                    Save monitor
                  </Button>
                </div>
              </CardFooter>
            </form>
          </Card>
        )}
        <FormAside title="How Monita checks this URL" steps={STEPS} note="Monita keeps single checks for 7 days and daily summaries for good." />
      </div>
    </>
  );
}
