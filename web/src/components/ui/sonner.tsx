"use client";

import type { ReactElement } from "react";
import {
  CircleCheckIcon,
  InfoIcon,
  Loader2Icon,
  OctagonXIcon,
  TriangleAlertIcon,
} from "@/components/icons";
import { useTheme } from "@/components/layout/ThemeProvider";
import { Toaster as Sonner } from "sonner";
import type { ToasterProps } from "sonner";

// Sonner reads its colours from CSS custom properties on the host element.
const TOAST_STYLE: React.CSSProperties & Record<`--${string}`, string> = {
  "--normal-bg": "var(--popover)",
  "--normal-text": "var(--popover-foreground)",
  "--normal-border": "var(--border)",
  "--border-radius": "var(--radius)",
};

const Toaster = ({ ...props }: ToasterProps): ReactElement => {
  const { theme = "system" } = useTheme();

  return (
    <Sonner
      theme={theme}
      // Bottom-center: sonner's default bottom-right corner sat over the workflow
      // editor's execution-logs panel while a toast was up. Bottom-center keeps
      // toasts clear of it. A caller can still override via props.
      position="bottom-center"
      className="toaster group"
      icons={{
        success: <CircleCheckIcon className="size-4" />,
        info: <InfoIcon className="size-4" />,
        warning: <TriangleAlertIcon className="size-4" />,
        error: <OctagonXIcon className="size-4" />,
        loading: <Loader2Icon className="size-4 animate-spin" />,
      }}
      style={TOAST_STYLE}
      {...props}
    />
  );
};

export { Toaster };
