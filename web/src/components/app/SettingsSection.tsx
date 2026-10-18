import type { FormEvent, ReactElement, ReactNode } from "react";
import { PageHeader } from "@/components/layout/Page";
import { Card, CardAction, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from "@/components/ui/card";
import { cn } from "@/lib/utils";

/** A settings page: its title, then its sections in one column. */
export function SettingsPage({ title, children }: { title: string; children: ReactNode }): ReactElement {
  return (
    <>
      <PageHeader title={title} />
      <div className="flex max-w-3xl flex-col gap-6">{children}</div>
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
    <Card className={cn("gap-0 overflow-hidden py-0", danger && "border-destructive/40")}>
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
