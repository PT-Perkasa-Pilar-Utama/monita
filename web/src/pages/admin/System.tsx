import { useState } from "react";
import type { FormEvent, ReactElement } from "react";
import { Facts } from "@/components/app/Facts";
import { StateBadge } from "@/components/app/Tone";
import { PageHeader } from "@/components/layout/Page";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Card, CardAction, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from "@/components/ui/card";
import { Field, FieldDescription, FieldError, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { Spinner } from "@/components/ui/spinner";
import { api, ApiError, errorText } from "@/lib/api";
import { ago } from "@/lib/time";
import type { LimitRow, SystemView } from "@/lib/types";
import { useApi } from "@/lib/useApi";
import { cn } from "@/lib/utils";
import { toast } from "sonner";

/** A bar from 1 to the highest value allowed, on a log scale; the tick marks the default. */
function Range({ at, tick }: { at: number; tick?: number }): ReactElement {
  return (
    <svg viewBox="0 0 100 10" preserveAspectRatio="none" aria-hidden="true" className="h-2.5 w-full">
      <rect y={3} width={100} height={4} rx={2} className="fill-muted" />
      <rect y={3} width={at} height={4} rx={2} className="fill-neutral-strong" />
      {tick !== undefined && <rect x={tick} y={0} width={1} height={10} className="fill-foreground" />}
    </svg>
  );
}

function EmailCard({ d }: { d: SystemView }): ReactElement {
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
  return (
    <Card id="email">
      <CardHeader>
        <CardTitle className="text-[15px]">Email delivery</CardTitle>
        <CardAction>{d.mail.server ? <StateBadge kind="up" label="SMTP set" /> : <StateBadge kind="issue" label="No mail server" />}</CardAction>
      </CardHeader>
      <CardContent className="flex flex-col gap-5">
        {sentTo && (
          <Alert role="status">
            <AlertTitle>Test email queued for {sentTo}.</AlertTitle>
            <AlertDescription>Monita sends it within a few seconds, and the counts below show the result.</AlertDescription>
          </Alert>
        )}
        {!d.mail.server && (
          <p className="text-sm text-muted-foreground">
            Monita writes emails to its log instead of sending them. Set <code className="font-mono">MONITA_SMTP_URL</code> to send alerts and invites.
          </p>
        )}
        <Facts
          rows={[
            ["Mail server", <span className="font-mono text-[13px]">{d.mail.server ?? "not set"}</span>],
            ["Signs in as", d.mail.user ?? "no user name"],
            ["Sender", d.mail.from],
            ["Last sent", e.last_sent_at ? ago(e.last_sent_at) : "nothing sent yet"],
          ]}
        />
        <div className="grid grid-cols-3 gap-3" aria-label={`Emails in the last ${d.email_days} days`}>
          {(
            [
              [e.sent, `sent in ${d.email_days} days`, false],
              [e.waiting, "waiting or retrying", false],
              [e.failed, "gave up", e.failed > 0],
            ] as const
          ).map(([n, label, bad]) => (
            <div key={label} className={cn("flex flex-col gap-1 rounded-lg border bg-paper p-3", bad && "border-down/40 bg-down/8")}>
              <span className={cn("text-2xl font-semibold tabular-nums", bad && "text-down")}>{n.toLocaleString()}</span>
              <span className="text-xs text-muted-foreground">{label}</span>
            </div>
          ))}
        </div>
        {e.last_error && (
          <Alert variant="destructive">
            <AlertTitle>Last error, {ago(e.last_error[1])}</AlertTitle>
            <AlertDescription className="font-mono text-[13px] break-all">{e.last_error[0]}</AlertDescription>
          </Alert>
        )}
      </CardContent>
      <CardFooter>
        <Button variant="secondary" onClick={() => void test()} disabled={busy}>
          {busy && <Spinner data-icon="inline-start" />}
          Send a test email to me
        </Button>
      </CardFooter>
    </Card>
  );
}

function LimitsCard({ d, onSaved }: { d: SystemView; onSaved: () => void }): ReactElement {
  const [values, setValues] = useState<Record<string, string>>(() => Object.fromEntries(d.limits.map((l) => [l.key, String(l.value)])));
  const [error, setError] = useState<ApiError | null>(null);
  const [busy, setBusy] = useState(false);
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
  const row = (l: LimitRow): ReactElement => {
    const err = error?.fields[l.key];
    return (
      <Field key={l.key} data-invalid={err ? true : undefined} className="grid gap-3 sm:grid-cols-[minmax(0,1fr)_200px] sm:items-start">
        <div className="flex flex-col gap-1">
          <FieldLabel htmlFor={`limit-${l.key}`} required>
            {l.label}
          </FieldLabel>
          <FieldDescription>{l.help}</FieldDescription>
          {err && <FieldError>{err}</FieldError>}
        </div>
        <div className="flex flex-col gap-1.5">
          <Input
            id={`limit-${l.key}`}
            type="number"
            min={1}
            max={l.max}
            aria-required
            aria-invalid={err ? true : undefined}
            className="font-mono"
            value={values[l.key] ?? ""}
            onChange={(e) => setValues((v) => ({ ...v, [l.key]: e.target.value }))}
          />
          <Range at={l.at} tick={l.default_at} />
          <span className="text-xs text-muted-foreground">Default {l.default.toLocaleString()}</span>
        </div>
      </Field>
    );
  };
  return (
    <Card className="lg:col-span-2">
      <form onSubmit={(e) => void save(e)} noValidate className="flex flex-col gap-6">
        <CardHeader>
          <CardTitle className="text-[15px]">Limits</CardTitle>
          <CardDescription>The bar runs from 1 to the highest value allowed. The mark shows the default.</CardDescription>
        </CardHeader>
        <CardContent className="flex flex-col gap-8">
          {d.groups.map((g) => (
            <section key={g} className="flex flex-col gap-5">
              <h3 className="font-semibold">{g}</h3>
              {d.limits.filter((l) => l.group === g).map(row)}
            </section>
          ))}
        </CardContent>
        <CardFooter className="justify-end border-t">
          <Button type="submit" disabled={busy}>
            {busy && <Spinner data-icon="inline-start" />}
            Save limits
          </Button>
        </CardFooter>
      </form>
    </Card>
  );
}

/** Admin, System: email delivery, the server's configuration, the limits and the fixed caps. */
export function System(): ReactElement {
  const res = useApi<SystemView>("/admin/system");
  const d = res.data;
  return (
    <>
      <PageHeader title="System" />
      {!d ? (
        <Skeleton className="h-96 rounded-xl" />
      ) : (
        <div className="grid items-start gap-6 lg:grid-cols-2">
          <EmailCard d={d} />
          <Card>
            <CardHeader>
              <CardTitle className="text-[15px]">Server</CardTitle>
              <CardDescription>
                These values come from the env file. Run <code className="font-mono">monita whereis</code> to find it. After you change it, run{" "}
                <code className="font-mono">sudo systemctl restart monita</code>.
              </CardDescription>
            </CardHeader>
            <CardContent>
              <Facts rows={d.server.map(([k, v]) => [k, v] as [string, string])} />
            </CardContent>
          </Card>
          <LimitsCard key={d.limits.map((l) => l.value).join()} d={d} onSaved={res.reload} />
          <Card className="lg:col-span-2">
            <CardHeader>
              <CardTitle className="text-[15px]">Fixed caps</CardTitle>
              <CardDescription>Built into Monita. A list that reaches its cap refuses one more item.</CardDescription>
            </CardHeader>
            <CardContent className="grid grid-cols-[repeat(auto-fill,minmax(200px,1fr))] gap-3">
              {d.caps.map((c) => (
                <div key={c.label} className="flex flex-col gap-1.5 rounded-lg border bg-paper p-3">
                  <span className="text-xs text-muted-foreground">{c.label}</span>
                  {c.used === null ? (
                    <span className="text-xl font-semibold tabular-nums">{c.max.toLocaleString()}</span>
                  ) : (
                    <>
                      <span className="text-xl font-semibold tabular-nums">
                        {c.used.toLocaleString()} <span className="text-sm font-normal text-muted-foreground">of {c.max.toLocaleString()}</span>
                      </span>
                      <Range at={Math.min(100, (c.used * 100) / Math.max(1, c.max))} />
                    </>
                  )}
                </div>
              ))}
            </CardContent>
          </Card>
        </div>
      )}
    </>
  );
}
