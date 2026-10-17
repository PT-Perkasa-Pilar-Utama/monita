import type { ReactElement } from "react";
import { cn } from "@/lib/utils";

function Skeleton({ className, ...props }: React.HTMLAttributes<HTMLDivElement>): ReactElement {
  return <div className={cn("shimmer rounded-md bg-muted", className)} {...props} />;
}

export { Skeleton };
