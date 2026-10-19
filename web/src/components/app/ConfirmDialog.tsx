import { useState } from "react";
import type { ReactElement, ReactNode } from "react";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from "@/components/ui/alert-dialog";
import { errorText } from "@/lib/api";
import { useCanEdit } from "@/lib/session";
import { toast } from "sonner";

/**
 * Asks before a risky action: delete, revoke, disable. Cancel takes the focus, so Enter never
 * confirms by default. The confirm button repeats the action's label. Every use changes
 * something, so a supervisor sees neither the dialog nor its trigger.
 */
export function ConfirmDialog({
  trigger,
  title,
  description,
  action,
  onConfirm,
}: {
  trigger: ReactNode;
  title: string;
  description: string;
  action: string;
  onConfirm: () => Promise<void>;
}): ReactElement | null {
  const [open, setOpen] = useState(false);
  if (!useCanEdit()) return null;
  return (
    <AlertDialog open={open} onOpenChange={setOpen}>
      <AlertDialogTrigger asChild>{trigger}</AlertDialogTrigger>
      <AlertDialogContent onOpenAutoFocus={(e) => e.preventDefault()}>
        <AlertDialogHeader>
          <AlertDialogTitle>{title}</AlertDialogTitle>
          <AlertDialogDescription>{description}</AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel autoFocus>Cancel</AlertDialogCancel>
          <AlertDialogAction
            variant="destructive"
            onClick={(e) => {
              e.preventDefault();
              onConfirm()
                .then(() => setOpen(false))
                .catch((err: unknown) => toast.error(errorText(err)));
            }}
          >
            {action}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
