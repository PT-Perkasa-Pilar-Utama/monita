import type { ReactElement } from "react";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import type { AdminUserRow } from "@/lib/types";
import { ACTION_LABEL, type UserAction } from "@/lib/users";

const WHAT: Record<UserAction, string> = {
  make_admin: "A super admin manages every project, the users and the system settings.",
  make_tl: "A tech lead keeps the projects they own and loses the admin pages from their next click.",
  make_supervisor: "A supervisor sees every project, the users and the system settings without changing anything, and someone who owns a project cannot become one.",
  reset: "Monita emails a new sign-up link, and their password and sessions stop working until they use it.",
  two_factor_off: "They sign in with their password alone and can turn two-factor on again, so use it when they lost their phone and their recovery codes.",
  disable: "Monita signs them out now and they can no longer sign in, but their projects stay and you can enable them again.",
  enable: "They sign in with their old password, though someone who never set one needs a new invite.",
};

/** Asks before an action on one account or a selection. Cancel takes the focus. */
export function ActionDialog({
  action,
  targets,
  skipped,
  onCancel,
  onConfirm,
}: {
  action: UserAction;
  targets: AdminUserRow[];
  skipped: number;
  onCancel: () => void;
  onConfirm: () => void;
}): ReactElement {
  const who = targets.length === 1 ? targets[0]!.email : `${targets.length} users`;
  return (
    <AlertDialog open onOpenChange={(o) => !o && onCancel()}>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>
            {ACTION_LABEL[action]}: {who}?
          </AlertDialogTitle>
          <AlertDialogDescription>
            {WHAT[action]}
            {skipped > 0 && ` ${skipped} selected ${skipped === 1 ? "user is" : "users are"} left out because the action does not apply to them, or one is you.`}
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel>Cancel</AlertDialogCancel>
          <AlertDialogAction variant={action === "disable" ? "danger" : undefined} onClick={onConfirm}>
            {ACTION_LABEL[action]}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
