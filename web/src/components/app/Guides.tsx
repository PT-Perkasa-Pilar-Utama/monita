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
  ["2xx", "Any code from 200 to 299. This is the default."],
  ["200,204", "Exactly 200 or 204."],
  ["2xx,301", "Any 2xx code, or 301."],
  ["3xx", "Any redirect. Monita does not follow it, so the redirect itself is the answer."],
];

const TEXT_EXAMPLES: [string, string, string][] = [
  ['{"status": "ok"}', '"status": "ok"', "Passes."],
  ['{"status": "ok"}', '"status":"ok"', "Fails. The space is missing."],
  ['{"status": "OK"}', '"status": "ok"', "Fails. The capitals differ."],
];

/** What the "A check passes when" fields accept, with examples, so the form needs none. */
function PassRules(): ReactElement {
  return (
    <span className="flex flex-col gap-3">
      <span>A check passes when the answer comes within the timeout and the status code matches. Enter a class such as 2xx, a code, or a list separated by commas.</span>
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
        For an endpoint that needs a token, add a header such as Authorization: Bearer followed by the token. Pick POST or PUT to send a request body.
      </span>
      <span>
        The response text is optional. Monita reads the first megabyte of the body as plain text and looks for your text anywhere in it, with the same capitals and spaces. It
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
        "Monita sends a GET request to the URL at the interval you choose.",
        <PassRules />,
        "A monitor counts as down after 2 failed checks in a row, or the count you pick: 1, 2, 3 or 5. That opens an incident. The first passing check closes it.",
        "A critical monitor alerts by email and webhook. A warning monitor alerts by email only.",
        "With incidents off, a failure only sends a down alert, and recovery sends an up alert. The status page shows nothing.",
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
        "Your job calls that URL when it finishes. The next call must come within the time the job runs every, plus the time it can be late by. That moment is the deadline.",
        "For example: a job runs every 1 day and can be late by 1 hour. It calls at 01:00. The next deadline is 02:00 the next day. A call at 01:40 is in time; no call by 02:00 is a miss.",
        "If no call arrives by the deadline, Monita opens an incident. The next call closes it.",
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
        "Report a problem that no check catches, such as a slow sign-in for some users. Report one too when a monitor with incidents off goes down and users should know.",
        "The items you pick show as Issue on the status page, not as down. Uptime does not change.",
        "The first update appears on the status page at once, without your name.",
        "A reported incident stays open until you post an update labelled Resolved.",
      ]}
      note="A reported incident never counts as downtime."
    />
  );
}
