import { useState } from "react";
import { readLocal, writeLocal } from "@/lib/store";

export type ViewMode = "chart" | "card" | "list";

const MODES: ViewMode[] = ["chart", "card", "list"];

/** A list page's view, remembered in this browser under `key`; `fallback` until one is picked. */
export function useViewMode(key: string, fallback: ViewMode = "card"): [ViewMode, (v: ViewMode) => void] {
  const [mode, setMode] = useState<ViewMode>(() => {
    const saved = readLocal(key) as ViewMode | null;
    return saved && MODES.includes(saved) ? saved : fallback;
  });
  const set = (v: ViewMode): void => {
    writeLocal(key, v);
    setMode(v);
  };
  return [mode, set];
}
