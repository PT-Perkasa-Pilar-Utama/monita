import * as React from "react";
import type { ReactElement } from "react";
import { Label as LabelPrimitive } from "radix-ui";

import { cn } from "@/lib/utils";

/** A field's label. `required` adds a red asterisk; the control itself carries `aria-required`. */
function Label({
  className,
  children,
  required,
  ...props
}: React.ComponentProps<typeof LabelPrimitive.Root> & { required?: boolean | undefined }): ReactElement {
  return (
    <LabelPrimitive.Root
      data-slot="label"
      className={cn(
        "flex items-center gap-1 text-sm leading-none font-medium select-none group-data-[disabled=true]:pointer-events-none group-data-[disabled=true]:opacity-50 peer-disabled:cursor-not-allowed peer-disabled:opacity-50",
        className,
      )}
      {...props}
    >
      {children}
      {required && (
        <span aria-hidden="true" className="text-destructive">
          *
        </span>
      )}
    </LabelPrimitive.Root>
  );
}

export { Label };
