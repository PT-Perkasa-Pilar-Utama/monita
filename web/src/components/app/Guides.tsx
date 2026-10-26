import type { ReactElement, ReactNode } from "react";
import { BookOpen } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { Sheet, SheetContent, SheetHeader, SheetTitle, SheetTrigger } from "@/components/ui/sheet";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";

/** A secondary button that opens a half-width guide: what the thing does, in numbered sentences. */
export function GuideSheet({ label, title, steps, note }: { label: string; title: string; steps: ReactNode[]; note: string }): ReactElement {
  return (
    <Sheet>
      <SheetTrigger asChild>
        <Button type="button" variant="secondary">
          <BookOpen data-icon="inline-start" />
          {label}
        </Button>
      </SheetTrigger>
      <SheetContent size="half" aria-describedby={undefined}>
        <SheetHeader>
          <SheetTitle>{title}</SheetTitle>
        </SheetHeader>
        <div className="flex flex-col gap-6 overflow-y-auto px-6 py-6 text-sm">
          <ol className="flex flex-col gap-4">
            {steps.map((s, i) => (
              <li key={i} className="flex gap-4">
                <span aria-hidden="true" className="flex size-7 shrink-0 items-center justify-center rounded-full border bg-card text-[13px] font-semibold tabular-nums">
                  {i + 1}
                </span>
                <span className="pt-1">{s}</span>
              </li>
            ))}
          </ol>
          <p className="text-muted-foreground">{note}</p>
        </div>
      </SheetContent>
    </Sheet>
  );
}

const STATUS_EXAMPLES: [string, string][] = [
  ["2xx", "Any code from 200 to 299, which is the default."],
  ["200,204", "Exactly 200 or 204."],
  ["2xx,301", "Any 2xx code, or 301."],
  ["3xx", "Any redirect, which Monita does not follow, so the redirect itself is the answer."],
];

const TEXT_EXAMPLES: [string, string, string][] = [
  ['{"status": "ok"}', '"status": "ok"', "Passes."],
  ['{"status": "ok"}', '"status":"ok"', "Fails because the space is missing."],
  ['{"status": "OK"}', '"status": "ok"', "Fails because the capitals differ."],
];

/** What the "A check passes when" fields accept, with examples, so the form needs none. */
function PassRules(): ReactElement {
  return (
    <span className="flex flex-col gap-3">
      <span>A check passes when the answer comes within the timeout and the status code matches, so enter a class such as 2xx, a code, or a list separated by commas.</span>
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>You enter</TableHead>
            <TableHead>A check passes on</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {STATUS_EXAMPLES.map(([v, means]) => (
            <TableRow key={v}>
              <TableCell className="font-mono">{v}</TableCell>
              <TableCell className="whitespace-normal">{means}</TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
      <span>
        For an endpoint that needs a token, add a header such as Authorization: Bearer followed by the token, and pick POST or PUT to send a request body.
      </span>
      <span>
        The response text is optional. Monita reads the first megabyte of the body as plain text and looks for your text anywhere in it, with the same capitals and spaces, and it
        does not read JSON keys.
      </span>
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>The body is</TableHead>
            <TableHead>You enter</TableHead>
            <TableHead>The check</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {TEXT_EXAMPLES.map(([body, text, result]) => (
            <TableRow key={text}>
              <TableCell className="font-mono">{body}</TableCell>
              <TableCell className="font-mono">{text}</TableCell>
              <TableCell className="whitespace-normal">{result}</TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </span>
  );
}

export function MonitorGuide(): ReactElement {
  return (
    <GuideSheet
      label="How monitors work"
      title="How Monita checks a URL"
      steps={[
        "Monita sends a request to the URL at the interval you choose, with the method you pick (GET unless you change it).",
        <PassRules />,
        "A monitor counts as down after 2 failed checks in a row, or the count from 1 to 99 that you enter, and that opens an incident which the first passing check closes.",
        "Its alerts go to every alert card that listens to its level, critical or warning.",
        "With incidents off, a failure only sends a down alert, recovery sends an up alert, and the status page shows nothing.",
      ]}
      note="Monita keeps single checks for 7 days and daily summaries for good."
    />
  );
}

export function HeartbeatGuide(): ReactElement {
  return (
    <GuideSheet
      label="How heartbeats work"
      title="How a heartbeat works"
      steps={[
        "After you save, Monita gives the heartbeat a secret ping URL.",
        "Your job calls that URL when it finishes, and the next call must arrive before the deadline, which is its period plus the time it can be late by.",
        "For example, a job runs every 1 day and can be late by 1 hour. If it calls at 01:00, the next deadline is 02:00 the next day, so a call at 01:40 is in time and no call by 02:00 is a miss.",
        "If no call arrives by the deadline, Monita opens an incident, and the next call closes it.",
        "A call to the URL with /fail at the end opens an incident at once.",
        "With incidents off, a miss or a fail call only sends a down alert, and the next call sends an up alert.",
      ]}
      note="The first deadline counts from the moment you save, so a job that never calls still misses it."
    />
  );
}

export function IncidentGuide(): ReactElement {
  return (
    <GuideSheet
      label="When to report an incident"
      title="When to report an incident"
      steps={[
        "Report a problem that no check catches, such as a slow sign-in for some users, or when a monitor with incidents off goes down and users should know.",
        "The items you pick show as Issue on the status page, not as down, and uptime does not change.",
        "The first update appears on the status page at once, without your name.",
        "A reported incident stays open until you post an update labelled Resolved.",
      ]}
      note="A reported incident never counts as downtime."
    />
  );
}

export function MaintenanceGuide(): ReactElement {
  return (
    <GuideSheet
      label="How maintenance works"
      title="How maintenance works"
      steps={[
        "Plan a window with a start, an end and the monitors and heartbeats the work touches, in your own time, and it reads Scheduled until it starts, Active while it runs and Ended after.",
        "While a window is active Monita still checks its items, but a failure opens no incident and sends no alert, and it counts as held, so it leaves no red marks or downtime in the charts.",
        "A heartbeat that misses its deadline or reports a failure inside a window waits until the window ends, and opens an incident then only if it is still failing.",
        "The status page shows the items as Maintenance, with the message you write for visitors.",
        "End now finishes an active window early, and Delete removes one that has not started.",
        "Pause is different: a paused monitor or heartbeat is not watched at all until you resume it, and this page lists everything paused below the windows.",
      ]}
      note="If an outage was planned work after all, open the incident and reclassify it as planned work, so it leaves uptime and the status page."
    />
  );
}

export function ChangelogGuide(): ReactElement {
  return (
    <GuideSheet
      label="How the changelog works"
      title="How the changelog works"
      steps={[
        "Each release has a version such as 1.4.0, a date, a title and its changes, sorted into Added, Changed, Fixed and Removed, one change per line.",
        "A version is unique in the project, and latest is not allowed because the public address uses it for the newest release.",
        "Select a release to edit it, and a change or a delete shows on the public page at once.",
        "Visitors read the releases on the Changelog tab of the status page, one at a time with the newest first, and each release has its own address.",
        "Turn the public Changelog tab on or off in Settings, Status page.",
      ]}
      note="Write the changes for the people who use the product, because the status page shows them as written."
    />
  );
}
