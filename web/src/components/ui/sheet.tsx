import * as React from "react";
import type { ReactElement } from "react";
import { Dialog as SheetPrimitive } from "radix-ui";
import { XIcon } from "@/components/icons";

import { cn } from "@/lib/utils";

function Sheet({ ...props }: React.ComponentProps<typeof SheetPrimitive.Root>): ReactElement {
  return <SheetPrimitive.Root data-slot="sheet" {...props} />;
}

function SheetTrigger({ ...props }: React.ComponentProps<typeof SheetPrimitive.Trigger>): ReactElement {
  return <SheetPrimitive.Trigger data-slot="sheet-trigger" {...props} />;
}

function SheetClose({ ...props }: React.ComponentProps<typeof SheetPrimitive.Close>): ReactElement {
  return <SheetPrimitive.Close data-slot="sheet-close" {...props} />;
}

function SheetOverlay({ className, ...props }: React.ComponentProps<typeof SheetPrimitive.Overlay>): ReactElement {
  return (
    <SheetPrimitive.Overlay
      data-slot="sheet-overlay"
      className={cn(
        "data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 fixed inset-0 z-50 bg-black/40",
        className,
      )}
      {...props}
    />
  );
}

// Wide holds a record with a sidebar, such as a ticket, at three quarters of the screen; half holds a form or a guide.
const SIZE = { wide: "sm:w-3/4 sm:max-w-none", half: "sm:w-1/2 sm:min-w-[36rem] sm:max-w-none" } as const;

/** A panel that slides in from the right edge: `wide` for a record with a sidebar, `half` for a form or a guide. */
function SheetContent({ className, children, size = "wide", ...props }: React.ComponentProps<typeof SheetPrimitive.Content> & { size?: keyof typeof SIZE }): ReactElement {
  return (
    <SheetPrimitive.Portal>
      <SheetOverlay />
      <SheetPrimitive.Content
        data-slot="sheet-content"
        className={cn(
          "bg-background data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:slide-out-to-right data-[state=open]:slide-in-from-right fixed inset-y-0 right-0 z-50 flex h-full w-full flex-col border-l shadow-lg outline-none duration-200",
          SIZE[size],
          className,
        )}
        {...props}
      >
        {children}
        <SheetPrimitive.Close className="absolute top-4 right-4 inline-flex size-8 items-center justify-center rounded-lg text-muted-foreground hover:bg-accent hover:text-foreground focus-visible:ring-[3px] focus-visible:ring-ring focus-visible:outline-none [&_svg]:size-4">
          <XIcon />
          <span className="sr-only">Close</span>
        </SheetPrimitive.Close>
      </SheetPrimitive.Content>
    </SheetPrimitive.Portal>
  );
}

function SheetHeader({ className, ...props }: React.ComponentProps<"div">): ReactElement {
  return <div data-slot="sheet-header" className={cn("flex flex-col gap-2 border-b px-6 py-5 pr-14", className)} {...props} />;
}

function SheetTitle({ className, ...props }: React.ComponentProps<typeof SheetPrimitive.Title>): ReactElement {
  return <SheetPrimitive.Title data-slot="sheet-title" className={cn("text-xl font-semibold", className)} {...props} />;
}

function SheetDescription({ className, ...props }: React.ComponentProps<typeof SheetPrimitive.Description>): ReactElement {
  return <SheetPrimitive.Description data-slot="sheet-description" className={cn("text-sm text-muted-foreground", className)} {...props} />;
}

export { Sheet, SheetTrigger, SheetClose, SheetContent, SheetDescription, SheetHeader, SheetTitle };
