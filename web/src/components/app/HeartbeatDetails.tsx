import type { ReactElement, ReactNode } from "react";
import { CodeCopy, CopyField } from "@/components/app/CopyField";
import { Facts } from "@/components/app/Facts";
import { Info } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { Sheet, SheetContent, SheetDescription, SheetHeader, SheetTitle, SheetTrigger } from "@/components/ui/sheet";
import { whenItFails } from "@/lib/checks";
import { pingSnippets } from "@/lib/pingSnippets";
import { stamp } from "@/lib/time";
import type { HeartbeatView } from "@/lib/types";

function Part({ title, children }: { title: string; children: ReactNode }): ReactElement {
  return (
    <section className="flex flex-col gap-3">
      <h3 className="text-base font-semibold">{title}</h3>
      {children}
    </section>
  );
}

/** A heartbeat's ping URL, commands to call it from a job, and its settings, in a side panel. */
export function HeartbeatDetails({ h, pingUrl, trigger }: { h: HeartbeatView; pingUrl: string; trigger?: ReactElement }): ReactElement {
  return (
    <Sheet>
      <SheetTrigger asChild>
        {trigger ?? (
          <Button variant="secondary">
            <Info data-icon="inline-start" />
            Details
          </Button>
        )}
      </SheetTrigger>
      <SheetContent size="half">
        <SheetHeader>
          <SheetTitle>{h.name}</SheetTitle>
          <SheetDescription>How the job calls Monita, and how this heartbeat is set up.</SheetDescription>
        </SheetHeader>
        <div className="flex flex-col gap-8 overflow-y-auto px-6 py-6 text-sm">
          <Part title="Ping URL">
            <p className="text-muted-foreground">Call it with GET, HEAD or POST when the job succeeds. Add /fail to the end when it fails; Monita shows the first 1 KB of the body as the cause.</p>
            <CopyField id="ping-url" value={pingUrl} />
          </Part>
          <Part title="Call it from your job">
            <p className="text-muted-foreground">Any language works: an HTTP request to the URL is enough. These commands use curl, and the script is a starting point to adapt.</p>
            {pingSnippets(pingUrl).map((s, n) => (
              <CodeCopy key={s.title} id={`ping-snippet-${n}`} title={s.title} code={s.code} />
            ))}
          </Part>
          <Part title="Settings">
            <Facts
              rows={[
                ["Runs every", h.every],
                ["Can be late by", h.grace],
                ["When it fails", whenItFails(h.alert, h.auto_incident)],
                ["Status page", h.is_public ? "Shown, with its incidents" : "Not shown"],
                ["Created", stamp(h.created_at)],
              ]}
            />
          </Part>
        </div>
      </SheetContent>
    </Sheet>
  );
}
