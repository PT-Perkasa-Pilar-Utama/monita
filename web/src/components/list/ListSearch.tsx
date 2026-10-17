import type { ReactElement, ReactNode } from "react";

// ListEmpty is what a list shows when it has nothing, the same on every list.
export function ListEmpty({ children }: { children: ReactNode }): ReactElement {
  return (
    <div className="rounded-lg border border-dashed p-8 text-center text-sm text-muted-foreground">
      {children}
    </div>
  );
}
