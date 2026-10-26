import { DOCS } from "@/components/layout/TopBar";
import type { ReactElement, ReactNode } from "react";
import { CodeCopy } from "@/components/app/CopyField";
import { EndpointRows } from "@/components/app/CustomStatusSection";
import { endpointDocs } from "@/components/app/customStatusDocs";
import { BookOpen } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { Sheet, SheetContent, SheetHeader, SheetTitle, SheetTrigger } from "@/components/ui/sheet";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useProject } from "@/lib/project";
import { useMe } from "@/lib/session";

/** The endpoints a key opens, in the order a ticket flows. */
const FLOW = ["Issue a ref ID", "File a ticket", "Reporter's tickets"];

function Part({ title, children }: { title: string; children: ReactNode }): ReactElement {
  return (
    <section className="flex flex-col gap-3 text-sm">
      <h3 className="text-base font-semibold">{title}</h3>
      {children}
    </section>
  );
}

/** Settings, API keys: what a key is for, how to send it, and the endpoints of the ticket flow. */
export function ApiKeysGuide(): ReactElement {
  const { project: p } = useProject();
  const { base_url: origin } = useMe();
  const docs = endpointDocs(origin, p.slug, p.name)
    .filter((d) => FLOW.includes(d.tab))
    .sort((a, b) => FLOW.indexOf(a.tab) - FLOW.indexOf(b.tab));
  return (
    <Sheet>
      <SheetTrigger asChild>
        <Button variant="secondary">
          <BookOpen data-icon="inline-start" />
          How API keys work
        </Button>
      </SheetTrigger>
      <SheetContent size="half" aria-describedby={undefined}>
        <SheetHeader>
          <SheetTitle>How API keys work</SheetTitle>
        </SheetHeader>
        <div className="flex flex-col gap-10 overflow-y-auto px-6 py-6">
          <Part title="What a key is for">
            <p className="text-muted-foreground">
              A key lets your own server or scripts call Monita for this project, and the scope you pick when you create it decides how far it reaches.
            </p>
            <EndpointRows
              rows={[
                { label: "Ref IDs only", text: "Issues the ref IDs your users file and follow bug reports with, and nothing else." },
                { label: "Read", text: "Reads monitors, heartbeats, incidents and maintenance, but never sees a ping URL, a header value or a request body." },
                { label: "Read and write", text: "Also creates, changes and pauses them, opens incidents and posts updates, plans maintenance and issues ref IDs." },
              ]}
            />
          </Part>
          <Part title="How to send it">
            <EndpointRows
              rows={[
                { label: "Header", text: "Authorization: Bearer mk_… on each call." },
                { label: "Where", text: "Only on your server, because anyone can read a key placed in a browser page or a mobile app." },
                { label: "Once", text: "Monita shows a key only once, right after you create it, so keep it in your secret store." },
                { label: "Revoke", text: "Revoke a key that leaked, and every call that sends it gets 401 from then on." },
                { label: "Limits", text: "Each key may make 120 calls a minute by default and wrong keys count against the caller's address, both of which a super admin can change in Admin, Rate limits." },
              ]}
            />
            <CodeCopy id="key-env" title="Keep it in your server's environment" code={"MONITA_API_KEY=mk_…"} />
          </Part>
          <Part title="Managing the project">
            <p className="text-muted-foreground">
              A read or write key calls the same routes as these pages, under <span className="font-mono text-[13px]">/api/v1/projects/{p.slug}</span>, and the team sees the key's name on any incident update it posts.
            </p>
            <EndpointRows
              rows={[
                { label: "Project", text: "GET the path itself, or /overview for its numbers." },
                { label: "Monitors", text: "/monitors and /monitors/{id}, with PUT /monitors/{id}/paused to pause or resume one." },
                { label: "Heartbeats", text: "/heartbeats and /heartbeats/{id}, with PUT /heartbeats/{id}/paused the same way." },
                { label: "Incidents", text: "/incidents, /incidents/{id} and POST /incidents/{id}/updates." },
                { label: "Maintenance", text: "/maintenance, /maintenance/{id} and POST /maintenance/{id}/end." },
              ]}
            />
            <CodeCopy
              id="key-pause"
              title="Pause a monitor while you deploy"
              code={`curl -s -X PUT "${origin}/api/v1/projects/${p.slug}/monitors/1/paused" \\\n  -H "Authorization: Bearer $MONITA_API_KEY" \\\n  -H "Content-Type: application/json" \\\n  -d '{"paused": true}'`}
            />
          </Part>
          <Part title="The ticket flow">
            <p className="text-muted-foreground">
              Only <strong className="font-medium text-foreground">Issue a ref ID</strong> takes the key, while the other two calls take the ref ID your server issued, and the{" "}
              <a href={`${DOCS}api/`} target="_blank" rel="noreferrer" className="text-link underline-offset-2 hover:underline">
                Public API
              </a>{" "}
              page lists every endpoint Monita has.
            </p>
            <ol className="list-decimal space-y-1 pl-5 text-muted-foreground">
              <li>Your server issues a ref ID for a signed-in user, with a Ref IDs only or a Read and write key.</li>
              <li>Your app files that user's tickets with the ref ID, without the key.</li>
              <li>Your app lists that user's tickets with the same ref ID.</li>
            </ol>
            <Tabs defaultValue={FLOW[0]} className="gap-4">
              <TabsList className="flex-wrap">
                {docs.map((d) => (
                  <TabsTrigger key={d.tab} value={d.tab}>
                    {d.tab}
                  </TabsTrigger>
                ))}
              </TabsList>
              {docs.map((d, n) => (
                <TabsContent key={d.tab} value={d.tab} className="flex flex-col gap-4">
                  <p className="font-mono text-[13px] break-all">
                    <span className="font-semibold">{d.method}</span> {d.path}
                  </p>
                  <p className="text-muted-foreground">{d.purpose}</p>
                  <EndpointRows rows={[{ label: "Who can call it", text: d.access }]} />
                  <CodeCopy id={`key-req-${n}`} title="Request" code={d.request} />
                  <CodeCopy id={`key-res-${n}`} title={d.responseTitle} code={d.response} lang="js" />
                  <EndpointRows rows={d.details} />
                </TabsContent>
              ))}
            </Tabs>
          </Part>
        </div>
      </SheetContent>
    </Sheet>
  );
}
