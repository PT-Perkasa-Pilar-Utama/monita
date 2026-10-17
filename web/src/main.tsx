import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { BrowserRouter } from "react-router-dom";
import { IconContext } from "@/components/icons";
import { ThemeProvider } from "@/components/layout/ThemeProvider";
import { Toaster } from "@/components/ui/sonner";
import App from "@/App";

const root = document.getElementById("root");
if (!root) throw new Error("#root is missing from index.html");

createRoot(root).render(
  <StrictMode>
    <IconContext.Provider value={{ weight: "regular" }}>
      <ThemeProvider>
        <BrowserRouter>
          <App />
        </BrowserRouter>
        <Toaster />
      </ThemeProvider>
    </IconContext.Provider>
  </StrictMode>,
);
