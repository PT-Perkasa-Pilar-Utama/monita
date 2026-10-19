import { useState } from "react";
import type { ReactElement } from "react";
import { Facts } from "@/components/app/Facts";
import { SettingsSection } from "@/components/app/SettingsSection";
import { TimeZoneSection } from "@/components/app/TimeZoneSection";
import { StateBadge } from "@/components/app/Tone";
import { PageHeader } from "@/components/layout/Page";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { Spinner } from "@/components/ui/spinner";
import { api, errorText } from "@/lib/api";
import { ago } from "@/lib/time";
import type { SystemView } from "@/lib/types";
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
            ["Sends from", d.mail.from],
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

/** Admin, System information: how email goes out, and how the server is set up. */
export function System(): ReactElement {
  const res = useApi<SystemView>("/admin/system");
  const d = res.data;
  return (
    <>
      <PageHeader title="System information" />
      {!d ? (
        <Skeleton className="h-96 rounded-xl" />
      ) : (
        <div className="flex flex-col gap-6">
          <TimeZoneSection initial={d.utc_offset_min} />
          <EmailSection d={d} />
          <SettingsSection
            title="Server"
            description={
              <>
                These come from Monita's settings file on the server. To change one, edit that file and restart Monita. Run{" "}
                <code className="font-mono">monita whereis</code> to find the file.
              </>
            }
          >
            <Facts rows={d.server.map(([k, v]) => [k, v] as [string, string])} />
          </SettingsSection>
        </div>
      )}
    </>
  );
}
