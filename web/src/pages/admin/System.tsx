import { useState } from "react";
import type { FormEvent, ReactElement } from "react";
import { useSearchParams } from "react-router-dom";
import { Facts } from "@/components/app/Facts";
import { SettingsSection } from "@/components/app/SettingsSection";
import { StateBadge } from "@/components/app/Tone";
import { PageHeader } from "@/components/layout/Page";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Field, FieldError, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { Spinner } from "@/components/ui/spinner";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { api, ApiError, errorText } from "@/lib/api";
import { ago } from "@/lib/time";
import type { LimitRow, SystemView } from "@/lib/types";
import { useApi } from "@/lib/useApi";
import { cn } from "@/lib/utils";
import { toast } from "sonner";

function EmailSection({ d }: { d: SystemView }): ReactElement {
  const [busy, setBusy] = useState(false);
  const [sentTo, setSentTo] = useState<string | null>(null);
  const e = d.email;
  const test = async (): Promise<void> => {
    setBusy(true);
    try {
      setSentTo((await api<{ sent_to: string }>("/admin/test-email", "POST")).sent_to);
    } catch (err) {
      toast.error(errorText(err));
    } finally {
      setBusy(false);
    }
  };
  const counts = [
    [e.sent, `Sent in ${d.email_days} days`, false],
    [e.waiting, "Waiting or retrying", false],
    [e.failed, "Gave up", e.failed > 0],
  ] as const;
  return (
    <SettingsSection
      title="Email delivery"
      description={
        d.mail.server ? (
          "Alerts, digests and invites go out through this mail server."
        ) : (
          <>
            No mail server is set, so Monita writes each email to its log. Set <code className="font-mono">MONITA_SMTP_URL</code> to send them.
          </>
        )
      }
      action={d.mail.server ? <StateBadge kind="up" label="Sending" /> : <StateBadge kind="issue" label="Log only" />}
      footerNote={d.mail.server ? undefined : "Set a mail server to send a test email."}
      footer={
        <Button variant="secondary" onClick={() => void test()} disabled={busy || !d.mail.server}>
          {busy && <Spinner data-icon="inline-start" />}
          Send a test email to me
        </Button>
      }
    >
      <div className="flex flex-col gap-5">
        {sentTo && (
          <Alert role="status">
            <AlertTitle>Test email queued for {sentTo}.</AlertTitle>
            <AlertDescription>Monita sends it within a few seconds, and the counts below show the result.</AlertDescription>
          </Alert>
        )}
        <Facts
          rows={[
            ["Mail server", <span className="font-mono text-[13px]">{d.mail.server ?? "Not set"}</span>],
            ["Signs in as", d.mail.user ?? "No user name"],
            ["Sender", d.mail.from],
            ["Last sent", e.last_sent_at ? ago(e.last_sent_at) : "Nothing sent yet"],
          ]}
        />
        <dl className="grid grid-cols-3 divide-x rounded-lg border">
          {counts.map(([n, label, bad]) => (
            <div key={label} className="flex flex-col gap-0.5 px-4 py-3">
              <dt className="text-xs text-muted-foreground">{label}</dt>
              <dd className={cn("text-lg font-semibold tabular-nums", bad && "text-down")}>{n.toLocaleString()}</dd>
            </div>
          ))}
        </dl>
        {e.last_error && (
          <Alert variant="destructive">
            <AlertTitle>Last error, {ago(e.last_error[1])}</AlertTitle>
            <AlertDescription className="font-mono text-[13px] break-all">{e.last_error[0]}</AlertDescription>
          </Alert>
        )}
      </div>
    </SettingsSection>
  );
}

/** One limit as a GitHub settings row: what it is on the left, the number on the right. */
function LimitField({ l, value, error, onChange }: { l: LimitRow; value: string; error: string | undefined; onChange: (v: string) => void }): ReactElement {
  return (
    <Field data-invalid={error ? true : undefined} className="grid gap-x-8 gap-y-2 px-6 py-4 sm:grid-cols-[minmax(0,1fr)_auto] sm:items-center">
      <div className="flex min-w-0 flex-col gap-0.5">
        <FieldLabel htmlFor={`limit-${l.key}`}>{l.label}</FieldLabel>
        <p className="text-[13px] text-muted-foreground">{l.help}</p>
        {error && <FieldError>{error}</FieldError>}
      </div>
      <div className="flex items-center gap-3 sm:flex-col sm:items-end sm:gap-1">
        <Input
          id={`limit-${l.key}`}
          type="number"
          min={1}
          max={l.max}
          aria-required
          aria-invalid={error ? true : undefined}
          className="w-28 text-right font-mono tabular-nums"
          value={value}
          onChange={(e) => onChange(e.target.value)}
        />
        <span className={cn("text-xs text-muted-foreground", value !== String(l.default) && "text-foreground")}>
          Default {l.default.toLocaleString()}
        </span>
      </div>
    </Field>
  );
}

function LimitsForm({ d, onSaved }: { d: SystemView; onSaved: () => void }): ReactElement {
  const initial = Object.fromEntries(d.limits.map((l) => [l.key, String(l.value)]));
  const [values, setValues] = useState<Record<string, string>>(initial);
  const [error, setError] = useState<ApiError | null>(null);
  const [busy, setBusy] = useState(false);
  const changed = d.limits.some((l) => values[l.key] !== initial[l.key]);
  const save = async (e: FormEvent): Promise<void> => {
    e.preventDefault();
    setBusy(true);
    setError(null);
    try {
      await api("/admin/limits", "PUT", values);
      toast.success("Limits saved.");
      onSaved();
    } catch (err) {
      setError(err instanceof ApiError ? err : new ApiError(0, "error", errorText(err)));
    } finally {
      setBusy(false);
    }
  };
  return (
    <form onSubmit={(e) => void save(e)} noValidate className="flex flex-col gap-6">
      {d.groups.map((g) => (
        <SettingsSection key={g} title={g} flush>
          <div className="divide-y">
            {d.limits
              .filter((l) => l.group === g)
              .map((l) => (
                <LimitField key={l.key} l={l} value={values[l.key] ?? ""} error={error?.fields[l.key]} onChange={(v) => setValues((s) => ({ ...s, [l.key]: v }))} />
              ))}
          </div>
        </SettingsSection>
      ))}
      <div className="flex items-center justify-end gap-3">
        {error && Object.keys(error.fields).length === 0 && <p className="mr-auto text-sm text-destructive">{error.message}</p>}
        <Button type="button" variant="ghost" disabled={!changed || busy} onClick={() => setValues(initial)}>
          Discard changes
        </Button>
        <Button type="submit" disabled={!changed || busy}>
          {busy && <Spinner data-icon="inline-start" />}
          Save limits
        </Button>
      </div>
    </form>
  );
}

/** Admin, System: what the server runs with, and the limits a super admin can change. */
export function System(): ReactElement {
  const res = useApi<SystemView>("/admin/system");
  const [params, setParams] = useSearchParams();
  const tab = params.get("tab") === "limits" ? "limits" : "info";
  const d = res.data;
  return (
    <>
      <PageHeader title="System" />
      <Tabs value={tab} onValueChange={(v) => setParams(v === "limits" ? { tab: "limits" } : {}, { replace: true })}>
        <TabsList>
          <TabsTrigger value="info">Information</TabsTrigger>
          <TabsTrigger value="limits">Limits</TabsTrigger>
        </TabsList>
        {!d ? (
          <Skeleton className="h-96 rounded-xl" />
        ) : (
          <>
            <TabsContent value="info" className="flex flex-col gap-6">
              <EmailSection d={d} />
              <SettingsSection
                title="Server"
                description={
                  <>
                    These values come from the env file. Run <code className="font-mono">monita whereis</code> to find it, and{" "}
                    <code className="font-mono">sudo systemctl restart monita</code> after a change.
                  </>
                }
              >
                <Facts rows={d.server.map(([k, v]) => [k, v] as [string, string])} />
              </SettingsSection>
            </TabsContent>
            <TabsContent value="limits" className="flex flex-col gap-6">
              <LimitsForm key={d.limits.map((l) => l.value).join()} d={d} onSaved={res.reload} />
              <SettingsSection title="Fixed caps" description="Built into Monita. A list that reaches its cap refuses one more item." flush>
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead className="pl-6">Cap</TableHead>
                      <TableHead className="text-right">In use</TableHead>
                      <TableHead className="pr-6 text-right">Most allowed</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {d.caps.map((c) => (
                      <TableRow key={c.label}>
                        <TableCell className="pl-6">{c.label}</TableCell>
                        <TableCell className="text-right tabular-nums">{c.used === null ? "–" : c.used.toLocaleString()}</TableCell>
                        <TableCell className="pr-6 text-right tabular-nums">{c.max.toLocaleString()}</TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              </SettingsSection>
            </TabsContent>
          </>
        )}
      </Tabs>
    </>
  );
}
