import { DOCS } from "@/components/layout/TopBar";
import type { ReactElement, ReactNode } from "react";
import { CodeCopy } from "@/components/app/CopyField";
import { EndpointRows } from "@/components/app/CustomStatusSection";
import { endpointDocs, type EndpointDoc } from "@/components/app/customStatusDocs";
import { KEY_ERRORS, keyApiDocs, type PrimitiveDoc } from "@/components/app/keyApiDocs";
import { BookOpen, ExternalLink } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { Sheet, SheetContent, SheetHeader, SheetTitle, SheetTrigger } from "@/components/ui/sheet";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useProject } from "@/lib/project";
import { useMe } from "@/lib/session";

/** The ticket calls, in the order a ticket flows. */
const FLOW = ["Issue a ref ID", "File a ticket", "Reporter's tickets"];

function Part({ title, children }: { title: string; children: ReactNode }): ReactElement {
  return (
    <section className="flex flex-col gap-3 text-sm">
      <h3 className="text-base font-semibold">{title}</h3>
      {children}
    </section>
  );
}

/** One tab per call: the method and path, what it does, who may call it, a request and its answer. */
function Calls({ id, docs }: { id: string; docs: EndpointDoc[] }): ReactElement {
  return (
    <Tabs defaultValue={docs[0]?.tab} className="gap-4">
      <TabsList className="h-auto flex-wrap">
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
          <CodeCopy id={`${id}-req-${n}`} title="Request" code={d.request} />
          {d.response ? (
            <CodeCopy id={`${id}-res-${n}`} title={d.responseTitle} code={d.response} lang="js" />
          ) : (
            <p className="rounded-lg border bg-paper px-3 py-2.5 text-muted-foreground">{d.responseTitle}</p>
          )}
          <EndpointRows rows={d.details} />
        </TabsContent>
      ))}
    </Tabs>
  );
}

/** One tab per primitive: every route a key may call on it, which scope each needs, then examples. */
function Primitives({ docs }: { docs: PrimitiveDoc[] }): ReactElement {
  return (
    <Tabs defaultValue={docs[0]?.tab} className="gap-4">
      <TabsList className="h-auto flex-wrap">
        {docs.map((d) => (
          <TabsTrigger key={d.tab} value={d.tab}>
            {d.tab}
          </TabsTrigger>
        ))}
      </TabsList>
      {docs.map((d) => (
        <TabsContent key={d.tab} value={d.tab} className="flex flex-col gap-4">
          <p className="text-muted-foreground">{d.purpose}</p>
          <ul className="divide-y rounded-lg border">
            {d.routes.map((r) => (
              <li key={`${r.method} ${r.path}`} className="grid gap-1 px-4 py-2.5 sm:grid-cols-[minmax(0,15rem)_1fr_auto] sm:items-baseline sm:gap-4">
                <span className="font-mono text-[13px] break-all">
                  <span className="font-semibold">{r.method}</span> {r.path}
                </span>
                <span className="text-muted-foreground">{r.text}</span>
                <span className="text-[13px] text-muted-foreground sm:text-right">{r.write ? "Read and write" : "Read"}</span>
              </li>
            ))}
          </ul>
          {d.examples.map((e, n) => (
            <div key={e.title} className="flex flex-col gap-2">
              <CodeCopy id={`key-${d.tab}-${n}`} title={e.title} code={e.request} />
              {e.response ? (
                <CodeCopy id={`key-${d.tab}-${n}-res`} title={e.responseTitle} code={e.response} lang="js" />
              ) : (
                <p className="rounded-lg border bg-paper px-3 py-2.5 text-muted-foreground">{e.responseTitle}</p>
              )}
            </div>
          ))}
          <EndpointRows rows={d.details} />
        </TabsContent>
      ))}
    </Tabs>
  );
}

/** Settings, API keys: what each scope may do, how to send a key, and every call a key makes. */
export function ApiKeysGuide(): ReactElement {
  const { project: p } = useProject();
  const { base_url: origin } = useMe();
  const tickets = endpointDocs(origin, p.slug, p.name)
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
        <SheetHeader className="flex-row items-center justify-between gap-4">
          <SheetTitle>How API keys work</SheetTitle>
          <Button variant="secondary" size="sm" asChild>
            <a href={`${DOCS}api/`} target="_blank" rel="noreferrer">
              API docs
              <ExternalLink data-icon="inline-end" />
            </a>
          </Button>
        </SheetHeader>
        <div className="flex flex-col gap-10 overflow-y-auto px-6 py-6">
          <Part title="What a key is for">
            <p className="text-muted-foreground">
              A key lets your own server or scripts call Monita for this project, and the scope you pick when you create it decides which of the calls below it may make.
            </p>
            <EndpointRows
              rows={[
                { label: "Ref IDs only", text: "Issues the ref IDs your users file and follow bug reports with, and nothing else." },
                { label: "Read", text: "Reads monitors, heartbeats, incidents and maintenance, but never sees a ping URL, a header value or a request body." },
                { label: "Read and write", text: "Also creates, changes and pauses them, posts incident updates, plans maintenance and issues ref IDs." },
              ]}
            />
          </Part>
          <Part title="How to send it">
            <EndpointRows
              rows={[
                { label: "Header", text: "Authorization: Bearer mk_… on each call." },
                { label: "Where", text: "Only from your server or your pipeline, because anyone can read a key placed in a browser page or a mobile app." },
                { label: "Once", text: "Monita shows a key only once, right after you create it, so keep it in your secret store." },
                { label: "Revoke", text: "Revoke a key that leaked, and every call that sends it gets 401 from then on." },
                { label: "Limits", text: "Each key may make 120 calls a minute by default, which a super admin can change in Admin, Rate limits." },
              ]}
            />
            <CodeCopy id="key-env" title="Keep it in your server's environment" code={"MONITA_API_KEY=mk_…"} />
          </Part>
          <Part title="Managing the project">
            <p className="text-muted-foreground">
              A Read or a Read and write key calls the paths below after <span className="font-mono text-[13px]">/api/v1/projects/{p.slug}</span>, the same routes these pages use, and it reaches this project only. A GET on that path itself reads the project.
            </p>
            <Primitives docs={keyApiDocs(origin, p.slug)} />
          </Part>
          <Part title="The ticket flow">
            <p className="text-muted-foreground">Your users file and follow bug reports from inside your app with a ref ID, and only the first call sends a key.</p>
            <ol className="list-decimal space-y-1 pl-5 text-muted-foreground">
              <li>Your server issues a ref ID for a signed-in user, with a Ref IDs only or a Read and write key.</li>
              <li>Your app files that user's tickets with the ref ID, without the key.</li>
              <li>Your app lists that user's tickets with the same ref ID.</li>
            </ol>
            <Calls id="key-tickets" docs={tickets} />
          </Part>
          <Part title="When a call fails">
            <EndpointRows rows={KEY_ERRORS} />
          </Part>
        </div>
      </SheetContent>
    </Sheet>
  );
}
