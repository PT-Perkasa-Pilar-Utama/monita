import { useState } from "react";
import { readLocal, writeLocal } from "@/lib/store";

export type ViewMode = "card" | "list";

/** A list page's card or list view, remembered in this browser under `key`. */
export function useViewMode(key: string): [ViewMode, (v: ViewMode) => void] {
  const [mode, setMode] = useState<ViewMode>(() => (readLocal(key) === "list" ? "list" : "card"));
  const set = (v: ViewMode): void => {
    writeLocal(key, v);
    setMode(v);
  };
  return [mode, set];
}
