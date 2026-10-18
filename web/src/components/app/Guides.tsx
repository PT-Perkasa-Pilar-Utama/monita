import type { ReactElement, ReactNode } from "react";
import { BookOpen } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { Sheet, SheetContent, SheetHeader, SheetTitle, SheetTrigger } from "@/components/ui/sheet";

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

export function MonitorGuide(): ReactElement {
  return (
    <GuideSheet
      label="How monitors work"
      title="How Monita checks a URL"
      steps={[
        "Monita sends a GET request to the URL at the interval you choose.",
        "A check passes when the status matches the expected status within the timeout. If you set a keyword, the body must also contain it.",
        "Two failed checks in a row open an incident. The first passing check closes it.",
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
        "Your job calls that URL when it finishes. Each call moves the deadline to the time of the call plus the period and the grace time.",
        "If no call arrives by the deadline, Monita opens an incident. The next call closes it.",
        "A call to the URL with /fail at the end opens an incident at once.",
        "With incidents off, a miss or a fail call only sends a down alert, and the next call sends an up alert.",
      ]}
      note="The first deadline is the period plus the grace time from the moment you save."
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
