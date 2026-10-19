import { useState } from "react";
import { useMe } from "@/lib/session";
import type { ReactElement, ReactNode } from "react";
import { Link } from "react-router-dom";
import { Check, Copy, Plug } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { Sheet, SheetContent, SheetHeader, SheetTitle, SheetTrigger } from "@/components/ui/sheet";

/** A command to copy, in the docs' code style. */
function Code({ children }: { children: string }): ReactElement {
  const [copied, setCopied] = useState(false);
  const copy = async (): Promise<void> => {
    await navigator.clipboard.writeText(children);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };
  return (
    <div className="relative">
      <pre className="overflow-x-auto rounded-lg border bg-paper p-4 pr-24 font-mono text-[13px] leading-relaxed">{children}</pre>
      <Button type="button" variant="secondary" size="sm" className="absolute top-2 right-2" onClick={() => void copy()}>
        {copied ? <Check data-icon="inline-start" /> : <Copy data-icon="inline-start" />}
        {copied ? "Copied" : "Copy"}
      </Button>
    </div>
  );
}

function Step({ n, title, children }: { n: number; title: string; children: ReactNode }): ReactElement {
  return (
    <li className="flex gap-4">
      <span aria-hidden="true" className="flex size-7 shrink-0 items-center justify-center rounded-full border bg-card text-[13px] font-semibold tabular-nums">
        {n}
      </span>
      <div className="flex min-w-0 flex-1 flex-col gap-3 pt-0.5 text-sm">
        <h3 className="font-semibold">{title}</h3>
        {children}
      </div>
    </li>
  );
}

/** How a project's own app files tickets: the three calls, written for this project. */
export function ConnectApp({ slug }: { slug: string }): ReactElement {
  const { base_url: origin } = useMe();
  const settings = `/projects/${slug}/settings`;
  return (
    <Sheet>
      <SheetTrigger asChild>
        <Button variant="secondary">
          <Plug data-icon="inline-start" />
          Connect your app
        </Button>
      </SheetTrigger>
      <SheetContent size="half" aria-describedby={undefined}>
        <SheetHeader>
          <SheetTitle>Connect your app</SheetTitle>
          <p className="text-sm text-muted-foreground">Your users report bugs from inside your app. Monita never learns who they are.</p>
        </SheetHeader>
        <ol className="flex flex-col gap-8 overflow-y-auto px-6 py-6">
          <Step n={1} title="Make an API key">
            <p>
              Open <Link className="text-link underline-offset-2 hover:underline" to={`${settings}/keys`}>API keys</Link> and select New API key. Keep the key on your server. Never put it in a browser.
            </p>
          </Step>
          <Step n={2} title="Your server issues a ref ID for each user">
            <Code>{`curl -X POST ${origin}/api/v1/ref-ids \\
  -H "Authorization: Bearer $MONITA_KEY" \\
  -H "Content-Type: application/json" \\
  -d '{"single_use": false}'`}</Code>
            <p className="text-muted-foreground">
              The answer holds <code className="font-mono text-foreground">ref_id</code>. Store it on the user's record. With <code className="font-mono text-foreground">"single_use": true</code> the ref ID takes one ticket only.
            </p>
          </Step>
          <Step n={3} title="The user files a ticket with it">
            <Code>{`curl -X POST ${origin}/api/v1/tickets \\
  -H "Content-Type: application/json" \\
  -d '{"ref_id": "rf_...", "title": "Checkout button does nothing",
       "description": "I click Pay and nothing happens.",
       "page_url": "https://shop.example.com/cart"}'`}</Code>
            <p className="text-muted-foreground">
              This call needs no key, so the user's browser can send it. The ticket lands in Triage. A ref ID files up to 5 tickets a day, and the project up to 100.
            </p>
          </Step>
          <Step n={4} title="Show the user their tickets">
            <Code>{`curl ${origin}/api/v1/tickets/mine \\
  -H "X-Ref-Id: rf_..."`}</Code>
            <p className="text-muted-foreground">The answer lists their newest 100 tickets with the column, never the description.</p>
          </Step>
          <Step n={5} title="Or link to the form Monita hosts">
            <Code>{`${origin}/status/${slug}/ticket`}</Code>
          </Step>
          <Step n={6} title="Hear about new tickets">
            <p>
              Choose in{" "}
              <Link className="text-link underline-offset-2 hover:underline" to={`${settings}/alerts#tickets`}>
                Ticket alerts
              </Link>
              : a daily email to the project owner that lists new tickets in Triage, a webhook call for each new ticket, both or neither.
            </p>
          </Step>
        </ol>
      </SheetContent>
    </Sheet>
  );
}
