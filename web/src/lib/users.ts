import type { AdminUserRow } from "@/lib/types";

/** What a super admin can do to an account from the Users page. */
export type UserAction = "make_admin" | "make_tl" | "make_supervisor" | "reset" | "disable" | "enable";

export const ACTIONS: UserAction[] = ["make_admin", "make_tl", "make_supervisor", "reset", "disable", "enable"];

export const ACTION_LABEL: Record<UserAction, string> = {
  make_admin: "Make super admin",
  make_tl: "Make tech lead",
  make_supervisor: "Make supervisor",
  reset: "Send a new invite",
  disable: "Disable",
  enable: "Enable",
};

/** The same rule as the server: nobody acts on their own account, a disabled one can only be
 * enabled, and a role changes to one of the other two. The server also refuses to make a project
 * owner a supervisor; the page shows that refusal. */
export function appliesTo(action: UserAction, u: AdminUserRow, me: string): boolean {
  if (u.email === me) return false;
  const live = u.status !== "disabled";
  switch (action) {
    case "make_admin":
      return live && u.role !== "super_admin";
    case "make_tl":
      return live && u.role !== "tl";
    case "make_supervisor":
      return live && u.role !== "supervisor";
    case "reset":
    case "disable":
      return live;
    case "enable":
      return !live;
  }
}

/** The actions that reach at least one selected account. */
export function availableActions(selected: readonly AdminUserRow[], me: string): UserAction[] {
  return ACTIONS.filter((a) => selected.some((u) => appliesTo(a, u, me)));
}

/** Who an action runs on, and how many selected it skips. */
export function planAction(action: UserAction, selected: readonly AdminUserRow[], me: string): { targets: AdminUserRow[]; skipped: number } {
  const targets = selected.filter((u) => appliesTo(action, u, me));
  return { targets, skipped: selected.length - targets.length };
}
