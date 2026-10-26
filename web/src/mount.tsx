import { StrictMode, type ReactElement } from "react";
import { createRoot } from "react-dom/client";
import { BrowserRouter } from "react-router-dom";
import { IconContext } from "@/components/icons";
import { ThemeProvider } from "@/components/layout/ThemeProvider";
import { Toaster } from "@/components/ui/sonner";

/** Renders an app into the page's #root with the providers every page needs. */
export function mount(app: ReactElement): void {
  const root = document.getElementById("root");
  if (!root) throw new Error("#root is missing from the page");
  createRoot(root).render(
    <StrictMode>
      <IconContext.Provider value={{ weight: "regular" }}>
        <ThemeProvider>
          <BrowserRouter>{app}</BrowserRouter>
          <Toaster />
        </ThemeProvider>
      </IconContext.Provider>
    </StrictMode>,
  );
}
