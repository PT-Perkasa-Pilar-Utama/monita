import * as React from "react";
import type { ReactElement } from "react";
import { cva } from "class-variance-authority";
import type { VariantProps } from "class-variance-authority";

import { cn } from "@/lib/utils";

// Monita's tone pill: a tinted fill and a coloured word. State colours carry data (up, down,
// issue); the brand magenta never marks a state.
const badgeVariants = cva(
  "inline-flex w-fit shrink-0 items-center gap-1 whitespace-nowrap rounded-md border border-transparent px-1.5 py-0.5 text-xs font-semibold [&>svg]:size-3",
  {
    variants: {
      variant: {
        default: "bg-primary text-primary-foreground",
        secondary: "bg-secondary text-secondary-foreground",
        outline: "border-border text-muted-foreground",
        destructive: "bg-destructive/12 text-destructive",
        up: "bg-up/12 text-up",
        down: "bg-down/12 text-down",
        issue: "bg-issue/14 text-issue",
        unknown: "bg-muted text-muted-foreground",
        neutral: "bg-neutral-strong/12 text-neutral-strong",
        maintenance: "bg-neutral-strong/12 text-neutral-strong",
      },
    },
    defaultVariants: { variant: "secondary" },
  },
);

export type BadgeProps = React.HTMLAttributes<HTMLSpanElement> & VariantProps<typeof badgeVariants>;

function Badge({ className, variant, ...props }: BadgeProps): ReactElement {
  return <span data-slot="badge" className={cn(badgeVariants({ variant }), className)} {...props} />;
}

export { Badge, badgeVariants };
