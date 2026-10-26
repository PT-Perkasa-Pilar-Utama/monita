import type { ReactElement } from "react";
import { cn } from "@/lib/utils";

/** A project's icon, or its first letter in the brand colour when it has none. Decorative. */
export function ProjectBadge({
  name,
  iconUrl,
  className,
}: {
  name: string;
  iconUrl?: string | null | undefined;
  className?: string;
}): ReactElement {
  const base = cn("grid size-6 shrink-0 place-items-center rounded-md text-[11px] font-semibold", className);
  if (iconUrl) return <img src={iconUrl} alt="" className={cn(base, "bg-muted object-cover")} />;
  return (
    <span aria-hidden="true" className={cn(base, "bg-primary text-primary-foreground dark:bg-(image:--primary-raised) dark:shadow-[inset_0_1px_0_rgb(255_255_255/0.25)]")}>
      {name.charAt(0).toUpperCase() || "?"}
    </span>
  );
}
