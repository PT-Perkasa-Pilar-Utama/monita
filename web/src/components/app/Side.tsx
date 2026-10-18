import type { ReactElement, ReactNode } from "react";

/** A titled block of a page's or a panel's right column, as GitHub's issue sidebar has them. */
export function Side({ title, children }: { title: ReactNode; children: ReactNode }): ReactElement {
  return (
    <section className="flex flex-col gap-2.5 border-b pb-5 last:border-b-0">
      <h3 className="text-[13px] font-semibold text-muted-foreground">{title}</h3>
      {children}
    </section>
  );
}
