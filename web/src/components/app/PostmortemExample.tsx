import type { ReactElement } from "react";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle, DialogTrigger } from "@/components/ui/dialog";
import { EXAMPLE, SECTIONS } from "@/lib/postmortem";

/** A finished post-mortem of a made-up incident, read beside the form and never copied into it. */
export function PostmortemExample(): ReactElement {
  return (
    <Dialog>
      <DialogTrigger asChild>
        <Button type="button" variant="outline" size="sm">
          See an example
        </Button>
      </DialogTrigger>
      <DialogContent className="max-h-[85vh] overflow-auto overscroll-contain sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle>An example post-mortem</DialogTitle>
          <DialogDescription>This incident is made up, and each section reads as a published post-mortem should.</DialogDescription>
        </DialogHeader>
        <div className="flex flex-col gap-4">
          {SECTIONS.map((s) => (
            <div key={s.key} className="flex flex-col gap-1">
              <h3 className="text-sm font-semibold">{s.label}</h3>
              <p className="text-sm whitespace-pre-wrap break-words">{EXAMPLE[s.key]}</p>
            </div>
          ))}
        </div>
      </DialogContent>
    </Dialog>
  );
}
