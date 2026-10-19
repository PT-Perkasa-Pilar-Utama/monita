import type { ReactElement, ReactNode } from "react";
import { Info } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { Sheet, SheetContent, SheetDescription, SheetHeader, SheetTitle, SheetTrigger } from "@/components/ui/sheet";

/** One part of a Details panel: a heading and what belongs under it. */
export function DetailPart({ title, children }: { title: string; children: ReactNode }): ReactElement {
  return (
    <section className="flex flex-col gap-3">
      <h3 className="text-base font-semibold">{title}</h3>
      {children}
    </section>
  );
}

/** The Details button beside Edit on a monitor or heartbeat, and its half-width panel. */
export function DetailsSheet({ title, description, trigger, children }: { title: string; description: string; trigger?: ReactElement | undefined; children: ReactNode }): ReactElement {
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
          <SheetTitle>{title}</SheetTitle>
          <SheetDescription>{description}</SheetDescription>
        </SheetHeader>
        <div className="flex flex-col gap-8 overflow-y-auto px-6 py-6 text-sm">{children}</div>
      </SheetContent>
    </Sheet>
  );
}
