import type { FormEvent, ReactElement, ReactNode } from "react";
import { Sheet, SheetContent, SheetDescription, SheetHeader, SheetTitle } from "@/components/ui/sheet";

/** The half-width panel a create or edit form opens in, over the page it came from. */
export function FormSheet({ open, onClose, children }: { open: boolean; onClose: () => void; children: ReactNode }): ReactElement {
  return (
    <Sheet open={open} onOpenChange={(o) => !o && onClose()}>
      <SheetContent
        size="half"
        aria-describedby={undefined}
        onOpenAutoFocus={(e) => {
          // The first field, not the guide button in the header. A form still loading takes the panel.
          e.preventDefault();
          const panel = e.currentTarget as HTMLElement;
          (panel.querySelector<HTMLElement>("form input:not([type=hidden]), form textarea") ?? panel).focus();
        }}
      >
        {children}
      </SheetContent>
    </Sheet>
  );
}

/**
 * A form inside FormSheet: the title with an optional guide button, the fields in a column that
 * scrolls, and a footer that stays in view. Delete sits at the left of the footer, Cancel and Save
 * at the right.
 */
export function FormSheetBody({
  title,
  description,
  guide,
  onSubmit,
  footer,
  children,
}: {
  title: string;
  // One sentence under the title, when the form needs it.
  description?: string;
  guide?: ReactNode;
  onSubmit: (e: FormEvent) => void;
  footer: ReactNode;
  children: ReactNode;
}): ReactElement {
  return (
    <>
      <SheetHeader className="flex-row items-center justify-between gap-4">
        <div className="flex flex-col gap-1">
          <SheetTitle>{title}</SheetTitle>
          {description && <SheetDescription>{description}</SheetDescription>}
        </div>
        {guide}
      </SheetHeader>
      <form
        noValidate
        className="flex min-h-0 flex-1 flex-col"
        onSubmit={(e) => {
          e.preventDefault();
          onSubmit(e);
        }}
      >
        <div className="flex-1 overflow-y-auto px-6 py-6">
          {children}
        </div>
        <div className="flex flex-wrap items-center gap-2 border-t bg-paper px-6 py-3">{footer}</div>
      </form>
    </>
  );
}
