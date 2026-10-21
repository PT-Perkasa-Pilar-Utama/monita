import { useState } from "react";
import type { ReactElement, ReactNode } from "react";
import { SeverityBadge } from "@/components/app/Tone";
import { BookOpen } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { Sheet, SheetContent, SheetHeader, SheetTitle, SheetTrigger } from "@/components/ui/sheet";
import { Skeleton } from "@/components/ui/skeleton";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { useApi } from "@/lib/useApi";

type Preview = {
  event: string;
  label: string;
  // The settings that send it.
  route: "critical" | "warning" | "ticket";
  email: { subject: string; body: string } | null;
  webhook: unknown;
};

const CODE = "overflow-x-auto rounded-lg border bg-paper p-4 font-mono text-[13px] leading-relaxed whitespace-pre-wrap break-words";

function Part({ title, children }: { title: string; children: ReactNode }): ReactElement {
  return (
    <section className="flex flex-col gap-3 text-sm">
      <h3 className="text-base font-semibold">{title}</h3>
      {children}
    </section>
  );
}

function Severities(): ReactElement {
  return (
    <Part title="Critical and warning">
      <p className="text-muted-foreground">You choose critical or warning on each monitor and heartbeat. Its incidents alert that level's recipients.</p>
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>Alert level</TableHead>
            <TableHead>Use it for</TableHead>
            <TableHead>Sends by default</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          <TableRow>
            <TableCell>
              <SeverityBadge severity="critical" />
            </TableCell>
            <TableCell className="whitespace-normal">An outage that needs someone now, such as the checkout or the login.</TableCell>
            <TableCell>Email and webhook</TableCell>
          </TableRow>
          <TableRow>
            <TableCell>
              <SeverityBadge severity="warning" />
            </TableCell>
            <TableCell className="whitespace-normal">A problem that can wait for working hours, such as a slow report or a nightly job.</TableCell>
            <TableCell>Email</TableCell>
          </TableRow>
        </TableBody>
      </Table>
      <p className="text-muted-foreground">
        An incident opens after a monitor fails the number of checks in a row you set (2 unless you change it), a missed heartbeat or a fail ping, and closes on the next good one. Each recipient gets one alert when it opens and one
        when it closes. A monitor or heartbeat with incidents off sends a down alert and an up alert instead. One with alerts off sends nothing. A manual incident
        sends no alert; it shows on the status page. New tickets and ticket moves have their own settings in Ticket alerts.
      </p>
    </Part>
  );
}

/** Each kind of alert as Monita sends it, built by the server. */
function Samples({ slug, webhooks }: { slug: string; webhooks: Record<string, string | null> }): ReactElement {
  const previews = useApi<Preview[]>(`/projects/${slug}/alerts-preview`);
  const [kind, setKind] = useState("incident.opened");
  const p = previews.data?.find((x) => x.event === kind);
  return (
    <Part title="What recipients get">
      <p className="text-muted-foreground">Sample alerts for this project. The incident and the ticket are made up.</p>
      {!previews.data || !p ? (
        <Skeleton className="h-64" />
      ) : (
        <div className="flex flex-col gap-4">
          <ToggleGroup type="single" variant="outline" value={kind} onValueChange={(v) => v && setKind(v)} className="flex-wrap" aria-label="Alert">
            {previews.data.map((x) => (
              <ToggleGroupItem key={x.event} value={x.event} className="px-3">
                {x.label}
              </ToggleGroupItem>
            ))}
          </ToggleGroup>
          <h4 className="font-semibold">Email</h4>
          {p.email ? (
            <div className={CODE}>
              <span className="text-muted-foreground">Subject: </span>
              {p.email.subject}
              {"\n\n"}
              {p.email.body}
            </div>
          ) : (
            <p className="text-muted-foreground">
              {p.event === "ticket.created"
                ? "Each ticket sends no email of its own. The owner's daily digest lists the new ones."
                : p.event === "ticket.moved"
                  ? "A move sends no email. Only the webhook gets it, when moves are on in Ticket alerts."
                  : "This alert sends no email."}
            </p>
          )}
          <h4 className="font-semibold">Webhook</h4>
          <div className={CODE}>
            <span className="text-muted-foreground">POST </span>
            {webhooks[p.route] ?? "your webhook URL"}
            {"\n"}
            <span className="text-muted-foreground">Content-Type: application/json</span>
            {"\n\n"}
            {JSON.stringify(p.webhook, null, 2)}
          </div>
          <p className="text-muted-foreground">
            {p.event === "ticket.moved" ? "Goes to the webhook in Ticket alerts when moves are on." : p.route === "ticket" ? "Goes to the webhook in Ticket alerts when new tickets are on." : `Goes to the ${p.route} webhook when it is on.`} <code className="font-mono">at</code> is
            Unix seconds, UTC.
          </p>
        </div>
      )}
    </Part>
  );
}

function Delivery(): ReactElement {
  return (
    <Part title="Webhook delivery">
      <ul className="flex list-disc flex-col gap-2 pl-5">
        <li>Alerts leave from a queue, so a slow email server or webhook never delays a check.</li>
        <li>
          Monita sends a POST with a JSON body. Slack and Google Chat show its <code className="font-mono">text</code> field.
        </li>
        <li>A 2xx answer within 10 seconds counts as delivered. A redirect counts as a failure.</li>
        <li>A failed call is tried again after 1, 5 and 15 minutes. After the fourth failure Monita gives up and writes the error to its log.</li>
        <li>The webhook URL is a secret. Monita leaves it out of its logs and error messages.</li>
      </ul>
    </Part>
  );
}

/** The explanation of alerts, kept off the settings page: severities, samples and delivery rules. */
export function AlertsGuide({ slug, webhooks }: { slug: string; webhooks: Record<string, string | null> }): ReactElement {
  return (
    <Sheet>
      <SheetTrigger asChild>
        <Button variant="secondary">
          <BookOpen data-icon="inline-start" />
          How alerts work
        </Button>
      </SheetTrigger>
      <SheetContent size="half" aria-describedby={undefined}>
        <SheetHeader>
          <SheetTitle>How alerts work</SheetTitle>
        </SheetHeader>
        <div className="flex flex-col gap-10 overflow-y-auto px-6 py-6">
          <Severities />
          <Samples slug={slug} webhooks={webhooks} />
          <Delivery />
        </div>
      </SheetContent>
    </Sheet>
  );
}
