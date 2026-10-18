import { useEffect, useState } from "react";
import type { FormEvent, ReactElement, ReactNode } from "react";
import { PageHeader } from "@/components/layout/Page";
import { Card, CardAction, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from "@/components/ui/card";
import { cn } from "@/lib/utils";

export type TocItem = { id: string; label: string };

/** Which section is in view: the last one whose top has passed the bar under the header. */
function useActiveSection(ids: string[]): string {
  const [active, setActive] = useState(ids[0] ?? "");
  useEffect(() => {
    const onScroll = (): void => {
      const line = 140;
      let current = ids[0] ?? "";
      for (const id of ids) {
        const el = document.getElementById(id);
        if (el && el.getBoundingClientRect().top <= line) current = id;
      }
      if (window.innerHeight + window.scrollY >= document.body.scrollHeight - 2) current = ids.at(-1) ?? current;
      setActive(current);
    };
    onScroll();
    window.addEventListener("scroll", onScroll, { passive: true });
    return () => window.removeEventListener("scroll", onScroll);
  }, [ids.join(" ")]);
  return active;
}

/** The sections of a long settings page, beside it, as the docs show theirs. */
function OnThisPage({ items }: { items: TocItem[] }): ReactElement {
  const active = useActiveSection(items.map((i) => i.id));
  return (
    <nav aria-label="On this page" className="sticky top-[calc(var(--bar-h)+24px)] max-xl:hidden">
      <h2 className="mb-3 text-[15px] font-semibold">On this page</h2>
      <ul className="border-l">
        {items.map((i) => (
          <li key={i.id}>
            <a
              href={`#${i.id}`}
              aria-current={active === i.id ? "location" : undefined}
              className={cn(
                "-ml-px block border-l-2 border-transparent px-3 py-1.5 text-sm text-muted-foreground hover:text-foreground",
                active === i.id && "border-primary text-link",
              )}
            >
              {i.label}
            </a>
          </li>
        ))}
      </ul>
    </nav>
  );
}

/** A settings page: its title and actions, its sections in one column and, when given, On this page. */
export function SettingsPage({ title, actions, toc, children }: { title: string; actions?: ReactNode; toc?: TocItem[]; children: ReactNode }): ReactElement {
  return (
    <>
      <PageHeader title={title} actions={actions} />
      <div className={cn("grid items-start gap-10", toc && "xl:grid-cols-[minmax(0,1fr)_13rem]")}>
        <div className="flex min-w-0 flex-col gap-6">{children}</div>
        {toc && <OnThisPage items={toc} />}
      </div>
    </>
  );
}

/**
 * One settings section, the same on every settings page: a title and one sentence, an optional
 * action at the right, the fields, and a footer whose actions sit at the right. With `onSubmit`
 * the section is a form and a submit button in the footer saves it. `danger` marks the section
 * that destroys data.
 */
export function SettingsSection({
  id,
  title,
  description,
  action,
  footer,
  footerNote,
  onSubmit,
  danger = false,
  flush = false,
  children,
}: {
  // The anchor that On this page links to.
  id?: string;
  title: ReactNode;
  description?: ReactNode;
  action?: ReactNode;
  footer?: ReactNode;
  // Muted text at the footer's left, such as when the project was created.
  footerNote?: ReactNode;
  onSubmit?: (e: FormEvent) => void;
  danger?: boolean;
  // The content runs edge to edge, for a table.
  flush?: boolean;
  children?: ReactNode;
}): ReactElement {
  const body = (
    <>
      <CardHeader className="border-b py-5 [.border-b]:pb-5">
        <CardTitle className={cn("text-[15px]", danger && "text-destructive")}>{title}</CardTitle>
        {description && <CardDescription>{description}</CardDescription>}
        {action && <CardAction>{action}</CardAction>}
      </CardHeader>
      {children && <CardContent className={cn(flush ? "px-0" : "py-5")}>{children}</CardContent>}
      {(footer || footerNote) && (
        <CardFooter className="justify-between gap-3 rounded-b-xl border-t bg-paper py-3 [.border-t]:pt-3">
          <span className="text-[13px] text-muted-foreground">{footerNote}</span>
          <span className="flex items-center gap-2">{footer}</span>
        </CardFooter>
      )}
    </>
  );
  return (
    <Card id={id} className={cn("scroll-mt-[calc(var(--bar-h)+24px)] gap-0 overflow-hidden py-0", danger && "border-destructive/40")}>
      {onSubmit ? (
        <form
          onSubmit={(e) => {
            e.preventDefault();
            onSubmit(e);
          }}
        >
          {body}
        </form>
      ) : (
        body
      )}
    </Card>
  );
}
