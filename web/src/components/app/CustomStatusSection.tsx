import type { ReactElement } from "react";
import { CodeCopy } from "@/components/app/CopyField";
import { endpointDocs } from "@/components/app/customStatusDocs";
import { SettingsSection } from "@/components/app/SettingsSection";
import { Download } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useProject } from "@/lib/project";
import { useMe } from "@/lib/session";
import { statusTemplate } from "@/lib/statusTemplate";

function download(origin: string, slug: string): void {
  const url = URL.createObjectURL(new Blob([statusTemplate(origin, slug)], { type: "text/html" }));
  const a = document.createElement("a");
  a.href = url;
  a.download = `status-${slug}.html`;
  a.click();
  URL.revokeObjectURL(url);
}

/** How to build your own status page on Monita's public JSON: each endpoint with who may call
 * it, a request and a response, plus a template to start from. */
export function CustomStatusSection(): ReactElement {
  const { project: p } = useProject();
  const { base_url: origin } = useMe();
  const docs = endpointDocs(origin, p.slug, p.name);
  return (
    <SettingsSection
      id="custom"
      reference
      title="Custom status page"
      description="Build your own page from the same data. Start from the template, or call the endpoints below."
    >
      <div className="flex flex-col gap-6">
        <div className="flex flex-wrap items-center justify-between gap-3 rounded-lg border bg-paper px-4 py-3">
          <span className="text-sm">One HTML file that shows your systems and incidents and reloads every minute. Open it, or host it anywhere.</span>
          <Button variant="outline" onClick={() => download(origin, p.slug)}>
            <Download data-icon="inline-start" />
            Download template
          </Button>
        </div>
        <Tabs defaultValue={docs[0]!.tab} className="gap-4">
          <TabsList className="flex-wrap">
            {docs.map((d) => (
              <TabsTrigger key={d.tab} value={d.tab}>
                {d.tab}
              </TabsTrigger>
            ))}
          </TabsList>
          {docs.map((d, n) => (
            <TabsContent key={d.tab} value={d.tab} className="flex flex-col gap-4">
              <div className="flex flex-col gap-1">
                <p className="font-mono text-[13px] break-all">
                  <span className="font-semibold">{d.method}</span> {d.path}
                </p>
                <p className="text-sm">{d.purpose}</p>
              </div>
              <dl className="grid gap-x-6 gap-y-1 text-sm sm:grid-cols-[8rem_1fr]">
                <dt className="font-medium">Who can call it</dt>
                <dd className="text-muted-foreground">{d.access}</dd>
              </dl>
              <CodeCopy id={`custom-req-${n}`} title="Request" code={d.request} />
              <CodeCopy id={`custom-res-${n}`} title={d.responseTitle} code={d.response} lang="js" />
              <ul className="flex list-disc flex-col gap-1 pl-5 text-sm text-muted-foreground">
                {d.notes.map((t) => (
                  <li key={t}>{t}</li>
                ))}
                <li>An error answers {'{"error": {"code": "...", "message": "..."}}'} with its HTTP status. A 429 carries Retry-After in seconds.</li>
              </ul>
            </TabsContent>
          ))}
        </Tabs>
      </div>
    </SettingsSection>
  );
}
