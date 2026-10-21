import { readLocal, writeLocal } from "@/lib/store";

/** The admin pages, in side-nav order. The first is where Admin opens the first time. */
export const ADMIN_PAGES = ["/admin/users", "/admin/locations", "/admin/limits", "/admin/email", "/admin/time-zone", "/admin/system"] as const;

const KEY = "monita.admin-page";

/** The stored page when it is still an admin page; otherwise the first one. */
export function adminPage(raw: string): string {
  return (ADMIN_PAGES as readonly string[]).includes(raw) ? raw : ADMIN_PAGES[0];
}

/** Where the header's Admin link goes: the admin page opened last in this browser. */
export function lastAdminPage(): string {
  return adminPage(readLocal(KEY));
}

export function rememberAdminPage(path: string): void {
  if ((ADMIN_PAGES as readonly string[]).includes(path)) writeLocal(KEY, path);
}
