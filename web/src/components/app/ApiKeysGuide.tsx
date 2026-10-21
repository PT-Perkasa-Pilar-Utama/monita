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
  const docs = endpointDocs(origin, p.slug, p.name).filter((d) => FLOW.includes(d.tab));
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
              A key lets your own server call Monita for this project. It issues ref IDs: the codes your users file and follow bug reports with, from inside your app. A key cannot read or change monitors, incidents or settings.
            </p>
            <ol className="list-decimal space-y-1 pl-5 text-muted-foreground">
              <li>Your server issues a ref ID for a signed-in user, with the key.</li>
              <li>Your app files that user's tickets with the ref ID. It needs no key.</li>
              <li>Your app lists that user's tickets with the same ref ID.</li>
            </ol>
          </Part>
          <Part title="How to send it">
            <EndpointRows
              rows={[
                { label: "Header", text: "Authorization: Bearer mk_… on each call." },
                { label: "Where", text: "On your server only. A key in a browser page or a mobile app is public." },
                { label: "Once", text: "Monita shows a key once, right after you create it. Keep it in your secret store." },
                { label: "Revoke", text: "Revoke a key that leaked. Every call that sends it then gets 401." },
                { label: "Limits", text: "120 calls a minute for each key by default. Wrong keys count against the caller's address. A super admin sets both in Admin, Rate limits." },
              ]}
            />
            <CodeCopy id="key-env" title="Keep it in your server's environment" code={"MONITA_API_KEY=mk_…"} />
          </Part>
          <Part title="Endpoints">
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
