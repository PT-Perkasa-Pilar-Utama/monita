import * as React from "react";
import type { ReactElement } from "react";
import { Slot } from "@radix-ui/react-slot";
import { cva } from "class-variance-authority";
import type { VariantProps } from "class-variance-authority";

import { cn } from "@/lib/utils";

// Monita's buttons are flat, with the input's 8 px corner and a 160 ms colour fade. The brand magenta marks the one
// primary action of a view; everything else is secondary, outline or ghost. In dark mode the solid fills are raised
// (a light top edge, a dark rim and a shadow), so white text reads on them and they stand off the dark page.
const RAISED =
  "dark:shadow-[inset_0_1px_0_rgb(255_255_255/0.25),0_0_0_1px_rgb(0_0_0/0.5),0_2px_4px_rgb(0_0_0/0.5)] dark:[text-shadow:0_1px_1px_rgb(0_0_0/0.3)] dark:hover:brightness-110";
const buttonVariants = cva(
  "inline-flex shrink-0 items-center justify-center gap-2 whitespace-nowrap rounded-lg text-sm font-medium transition-colors duration-150 outline-none disabled:pointer-events-none disabled:opacity-50 [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4 focus-visible:ring-[3px] focus-visible:ring-ring aria-invalid:border-destructive",
  {
    variants: {
      variant: {
        default: cn("bg-primary text-primary-foreground hover:bg-primary/90 dark:bg-(image:--primary-raised)", RAISED),
        secondary: "border border-border bg-secondary text-secondary-foreground hover:bg-accent",
        outline: "border border-input bg-card text-foreground hover:bg-accent",
        ghost: "text-muted-foreground hover:bg-accent hover:text-foreground",
        destructive: cn("bg-destructive text-destructive-foreground hover:bg-destructive/90 dark:bg-(image:--destructive-raised)", RAISED),
        danger: "border border-destructive/40 bg-secondary text-destructive hover:bg-destructive/10",
        link: "rounded-none px-0 text-link underline-offset-4 hover:underline",
      },
      size: {
        default: "h-9 px-4 has-[>svg]:px-3.5",
        sm: "h-8 gap-1.5 px-3 text-[13px] has-[>svg]:px-2.5",
        lg: "h-11 px-5 text-[15px]",
        icon: "size-9",
        "icon-sm": "size-8",
      },
    },
    defaultVariants: { variant: "default", size: "default" },
  },
);

function Button({
  className,
  variant,
  size,
  asChild = false,
  ...props
}: React.ComponentProps<"button"> &
  VariantProps<typeof buttonVariants> & {
    asChild?: boolean;
  }): ReactElement {
  const Comp = asChild ? Slot : "button";

  return (
    <Comp
      data-slot="button"
      className={cn(buttonVariants({ variant, size, className }))}
      {...props}
    />
  );
}

export { Button, buttonVariants };
