import * as React from "react";

import { Loader2 as Loader2Icon } from "@/components/icons";
import type { IconProps } from "@/components/icons";
import { cn } from "@/lib/utils";

function Spinner({ className, ...props }: IconProps): React.ReactElement {
  return (
    <Loader2Icon
      role="status"
      aria-label="Loading"
      className={cn("size-4 animate-spin", className)}
      {...props}
    />
  );
}

export { Spinner };
