import { useEffect, useState } from "react";
import type { ReactElement } from "react";
import { Pin } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { api, errorText } from "@/lib/api";
import { cn } from "@/lib/utils";
import { toast } from "sonner";

/**
 * Pins one project, monitor or heartbeat to the top for the signed-in user (Q48). It sits on every
 * row and card, faint until pointed at, solid once pinned. A click flips it at once and the list
 * reorders on `onChange`; the toast offers Undo, and at the cap it says which one to unpin.
 * Supervisors pin too, so it never hides behind `Writable`.
 */
export function PinButton({ path, name, pinned, onChange, className }: { path: string; name: string; pinned: boolean; onChange: () => void; className?: string }): ReactElement {
  const [on, setOn] = useState(pinned);
  useEffect(() => setOn(pinned), [pinned]);
  const flip = async (to: boolean, undo = true): Promise<void> => {
    setOn(to);
    try {
      await api(`${path}/pin`, to ? "PUT" : "DELETE");
      onChange();
      if (undo) {
        toast.success(to ? `${name} is pinned to the top.` : `${name} is no longer pinned.`, {
          action: { label: "Undo", onClick: () => void flip(!to, false) },
        });
      }
    } catch (err) {
      setOn(!to);
      toast.error(errorText(err));
    }
  };
  return (
    <Button
      type="button"
      variant="ghost"
      size="icon"
      aria-pressed={on}
      aria-label={on ? `Unpin ${name}` : `Pin ${name}`}
      title={on ? "Unpin" : "Pin to the top"}
      className={cn("size-7 shrink-0", on ? "text-foreground" : "text-muted-foreground/50 hover:text-foreground", className)}
      onClick={(e) => {
        // Rows and cards open on a click; the pin is its own action.
        e.preventDefault();
        e.stopPropagation();
        void flip(!on);
      }}
    >
      <Pin weight={on ? "fill" : "regular"} />
    </Button>
  );
}
