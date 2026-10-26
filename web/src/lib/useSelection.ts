import { useState } from "react";

/** Rows picked on a list by id, with the header box's state over the rows shown now. */
export function useSelection(shown: { id: number }[]): {
  selected: ReadonlySet<number>;
  toggle: (id: number) => void;
  toggleShown: (on: boolean) => void;
  headerBox: boolean | "indeterminate";
  clear: () => void;
} {
  const [selected, setSelected] = useState<Set<number>>(new Set());
  const picked = shown.filter((r) => selected.has(r.id)).length;
  return {
    selected,
    toggle: (id) =>
      setSelected((s) => {
        const n = new Set(s);
        n.has(id) ? n.delete(id) : n.add(id);
        return n;
      }),
    toggleShown: (on) =>
      setSelected((s) => {
        const n = new Set(s);
        for (const r of shown) on ? n.add(r.id) : n.delete(r.id);
        return n;
      }),
    headerBox: picked === 0 ? false : picked === shown.length ? true : "indeterminate",
    clear: () => setSelected(new Set()),
  };
}
