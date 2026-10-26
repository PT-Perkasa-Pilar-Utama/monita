import { useState } from "react";
import type { ReactElement } from "react";
import { ExternalLink, Sparkle, X } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { useMe } from "@/lib/session";
import { markSeen, RELEASES, releaseUrl, seenVersion, showsWhatsNew } from "@/lib/whatsNew";

/** One row under the top bar after Monita is upgraded: what is new, in a dialog of plain
 * highlights with a link to the full notes. Closed or read, it stays away until the next release. */
export function WhatsNewNote(): ReactElement | null {
  const me = useMe();
  const latest = RELEASES[0];
  const [seen, setSeen] = useState(() => seenVersion(me.email));
  const [open, setOpen] = useState(false);
  if (!latest || (!open && !showsWhatsNew(latest.version, seen))) return null;
  const done = (): void => {
    markSeen(me.email, latest.version);
    setSeen(latest.version);
  };
  return (
    <>
      {showsWhatsNew(latest.version, seen) && (
        <div role="note" aria-label="What's new" className="border-b bg-muted px-4 sm:px-8 lg:px-12">
          <div className="mx-auto flex max-w-app items-center gap-3 py-1.5 text-sm">
            <Sparkle aria-hidden="true" className="size-4 shrink-0 text-muted-foreground" />
            <p className="min-w-0 flex-1 truncate">
              {latest.summary}{" "}
              <button type="button" className="font-medium text-link underline-offset-4 hover:underline" onClick={() => setOpen(true)}>
                See what's new
              </button>
            </p>
            <Button type="button" variant="ghost" size="icon" className="size-7" aria-label="Close the what's new note" onClick={done}>
              <X />
            </Button>
          </div>
        </div>
      )}
      <Dialog
        open={open}
        onOpenChange={(o) => {
          setOpen(o);
          if (!o) done();
        }}
      >
        <DialogContent className="sm:max-w-xl">
          <DialogHeader>
            <DialogTitle>What's new in Monita {latest.version}</DialogTitle>
            <DialogDescription>{latest.summary}</DialogDescription>
          </DialogHeader>
          <ul className="flex max-h-[60vh] flex-col gap-4 overflow-y-auto text-sm">
            {latest.highlights.map((h) => (
              <li key={h.title} className="flex gap-3">
                <Sparkle aria-hidden="true" weight="fill" className="mt-0.5 size-4 shrink-0 text-primary" />
                <span>
                  <span className="font-medium">{h.title}</span>
                  <span className="block text-muted-foreground">{h.text}</span>
                </span>
              </li>
            ))}
          </ul>
          <DialogFooter className="sm:justify-between">
            <Button asChild variant="link" className="px-0">
              <a href={releaseUrl(latest.version)} target="_blank" rel="noreferrer">
                Full release notes on GitHub
                <ExternalLink data-icon="inline-end" />
              </a>
            </Button>
            <Button type="button" onClick={() => (setOpen(false), done())}>
              Got it
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}
