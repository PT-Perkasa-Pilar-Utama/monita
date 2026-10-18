import type { ReactElement } from "react";
import { SettingsSection } from "@/components/app/SettingsSection";
import { SeverityBadge } from "@/components/app/Tone";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";

/** Critical against warning, and when either fires. */
export function HowAlertsWork(): ReactElement {
  return (
    <SettingsSection
      id="how"
      title="How alerts work"
      description="You choose critical or warning on each monitor and heartbeat. Its incidents alert that severity's recipients."
    >
      <div className="flex flex-col gap-4 text-sm">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Severity</TableHead>
              <TableHead>Use it for</TableHead>
              <TableHead>Sends by default</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            <TableRow>
              <TableCell><SeverityBadge severity="critical" /></TableCell>
              <TableCell className="whitespace-normal">An outage that needs someone now, such as the checkout or the login.</TableCell>
              <TableCell>Email and webhook</TableCell>
            </TableRow>
            <TableRow>
              <TableCell><SeverityBadge severity="warning" /></TableCell>
              <TableCell className="whitespace-normal">A problem that can wait for working hours, such as a slow report or a nightly job.</TableCell>
              <TableCell>Email</TableCell>
            </TableRow>
          </TableBody>
        </Table>
        <p className="text-muted-foreground">
          An incident opens after 2 failed checks in a row, a missed heartbeat or a fail ping, and closes on the next good one. Each recipient gets one alert when it opens and
          one when it closes. A manual incident sends no alert; it shows on the status page. New tickets have their own switches in Ticket alerts.
        </p>
      </div>
    </SettingsSection>
  );
}

/** How webhook calls leave and retry. */
export function Delivery(): ReactElement {
  return (
    <SettingsSection id="delivery" title="Webhook delivery" description="Alerts leave from a queue, so a slow email server or webhook never delays a check.">
      <ul className="flex list-disc flex-col gap-2 pl-5 text-sm">
        <li>Monita sends a POST with a JSON body. Slack and Google Chat show its <code className="font-mono">text</code> field.</li>
        <li>A 2xx answer within 10 seconds counts as delivered. A redirect counts as a failure.</li>
        <li>A failed call is tried again after 1, 5 and 15 minutes. After the fourth failure Monita gives up and writes the error to its log.</li>
        <li>The webhook URL is a secret. Monita leaves it out of its logs and error messages.</li>
      </ul>
    </SettingsSection>
  );
}
