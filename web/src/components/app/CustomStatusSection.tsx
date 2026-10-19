import type { ReactElement } from "react";
import { CodeCopy } from "@/components/app/CopyField";
import { SettingsSection } from "@/components/app/SettingsSection";
import { Download } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { useProject } from "@/lib/project";
import { statusTemplate } from "@/lib/statusTemplate";

const ROUTES: { method: string; path: string; access: string; use: string }[] = [
  { method: "GET", path: "/api/v1/public/status/{slug}", access: "None", use: "Systems, their last 90 days and incidents. Add ?days=1 to 90 for a shorter window." },
  { method: "POST", path: "/api/v1/tickets", access: "A ref ID in the body", use: "Files a bug report from your page. Body: ref_id, title, description, page_url." },
  { method: "GET", path: "/api/v1/tickets/mine", access: "A ref ID in the X-Ref-Id header", use: "Lists the reporter's own tickets and their columns." },
  { method: "POST", path: "/api/v1/ref-ids", access: "An API key, from your server only", use: "Gives one of your users a ref ID. Never put the API key in a browser page." },
];

const FIELDS: [string, string][] = [
  ["overall", "up, down or issue, for the whole page"],
  ["items[]", "Each public system: name, state, uptime_90d and days[] (day, uptime, downtime_s)"],
  ["incidents[]", "Open incidents and those that started in the window: title, items, started_at, resolved_at and updates[] (label, message, at)"],
];

function download(origin: string, slug: string): void {
  const url = URL.createObjectURL(new Blob([statusTemplate(origin, slug)], { type: "text/html" }));
  const a = document.createElement("a");
  a.href = url;
  a.download = `status-${slug}.html`;
  a.click();
  URL.revokeObjectURL(url);
}

/** How to build your own status page on Monita's public JSON, with a template to start from. */
export function CustomStatusSection(): ReactElement {
  const { project: p } = useProject();
  const origin = window.location.origin;
  return (
    <SettingsSection
      id="custom"
      reference
      title="Custom status page"
      description="Build your own page from the same data. The status JSON needs no API key, and a browser can call it from any website."
    >
      <div className="flex flex-col gap-6">
        <div className="overflow-hidden rounded-lg border">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Endpoint</TableHead>
                <TableHead>Key</TableHead>
                <TableHead className="max-md:hidden">What it does</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {ROUTES.map((r) => (
                <TableRow key={r.path}>
                  <TableCell className="font-mono text-[13px] whitespace-normal">
                    <span className="text-muted-foreground">{r.method}</span> {r.path.replace("{slug}", p.slug)}
                  </TableCell>
                  <TableCell className="whitespace-normal">{r.access}</TableCell>
                  <TableCell className="whitespace-normal text-muted-foreground max-md:hidden">{r.use}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>
        <CodeCopy id="custom-status-curl" title="Try it" code={`curl -s ${origin}/api/v1/public/status/${p.slug}`} />
        <dl className="grid gap-x-6 gap-y-2 text-sm sm:grid-cols-[9rem_1fr]">
          {FIELDS.map(([k, v]) => (
            <div key={k} className="contents">
              <dt className="font-mono text-[13px]">{k}</dt>
              <dd className="text-muted-foreground">{v}</dd>
            </div>
          ))}
        </dl>
        <ul className="flex list-disc flex-col gap-1 pl-5 text-sm text-muted-foreground">
          <li>Only the monitors and heartbeats you mark as public appear.</li>
          <li>While the Incidents tab is off, incidents comes back empty.</li>
          <li>Each visitor address may call 60 times a minute. An answer can be up to 30 seconds old.</li>
        </ul>
        <div className="flex flex-wrap items-center justify-between gap-3 rounded-lg border bg-paper px-4 py-3">
          <span className="text-sm">A single HTML file that shows your systems and incidents and refreshes every minute.</span>
          <Button variant="outline" onClick={() => download(origin, p.slug)}>
            <Download data-icon="inline-start" />
            Download template
          </Button>
        </div>
      </div>
    </SettingsSection>
  );
}
