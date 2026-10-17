import type { ReactElement } from "react";
import { Moon, Sun } from "@/components/icons";
import { useTheme } from "@/components/layout/ThemeProvider";
import { Button } from "@/components/ui/button";

/** One button that switches between the light and the dark theme. */
export function ThemeToggle({ className }: { className?: string }): ReactElement {
  const { resolved, setTheme } = useTheme();
  const next = resolved === "dark" ? "light" : "dark";
  return (
    <Button
      variant="ghost"
      size="icon"
      className={className}
      onClick={() => setTheme(next)}
      aria-label={`Switch to the ${next} theme`}
      title={`Switch to the ${next} theme`}
    >
      {resolved === "dark" ? <Sun /> : <Moon />}
    </Button>
  );
}
