import { useState } from "react";
import type { ReactElement } from "react";
import { Link, useLocation } from "react-router-dom";
import { ShieldCheck, X } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { useMe } from "@/lib/session";
import { closeNote, noteClosed, showsNote } from "@/lib/twoFactorNote";

/** One row under the top bar while two-factor sign-in is off. Closed, it stays closed in this browser. */
export function TwoFactorNote(): ReactElement | null {
  const me = useMe();
  const { pathname } = useLocation();
  const [closed, setClosed] = useState(() => noteClosed(me.email));
  if (!showsNote(me.two_factor, closed) || pathname === "/account/two-factor") return null;
  const close = (): void => {
    closeNote(me.email);
    setClosed("1");
  };
  return (
    <div role="note" aria-label="Two-factor sign-in" className="border-b bg-muted px-4 sm:px-8 lg:px-12">
      <div className="mx-auto flex max-w-app items-center gap-3 py-1.5 text-sm">
        <ShieldCheck aria-hidden="true" className="size-4 shrink-0 text-muted-foreground" />
        <p className="min-w-0 flex-1 truncate">
          Two-factor sign-in is off for your account.{" "}
          <Link to="/account/two-factor" className="font-medium text-link underline-offset-4 hover:underline">
            Turn it on
          </Link>
        </p>
        <Button type="button" variant="ghost" size="icon" className="size-7" aria-label="Close the two-factor note" onClick={close}>
          <X />
        </Button>
      </div>
    </div>
  );
}
