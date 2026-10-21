import type { ReactElement, ReactNode } from "react";
import { createContext, useContext } from "react";
import { setOffset } from "@/lib/time";
import { useApi } from "@/lib/useApi";

export type Role = "super_admin" | "tl" | "supervisor";

export type Me = {
  email: string;
  role: Role;
  /** Changes anything: a super admin. */
  is_admin: boolean;
  /** Sees every project and the admin pages: a super admin or a supervisor. */
  sees_all: boolean;
  /** Reads only: a supervisor. The server refuses their writes; pages hide the controls. */
  read_only: boolean;
  /** Monita's public address (`MONITA_BASE_URL`). Links and examples for people outside use it,
   * not the address this browser happened to use. */
  base_url: string;
  utc_offset_min: number;
  /** Two-factor sign-in is on. While it is off, the top bar notes it until the user closes the note. */
  two_factor: boolean;
};

export const ROLE_LABEL: Record<Role, string> = { super_admin: "Super admin", tl: "Tech lead", supervisor: "Supervisor" };

const SessionContext = createContext<Me | null>(null);
const ReloadContext = createContext<() => void>(() => {});

/** Loads the signed-in user; renders nothing until it is known. A 401 has already moved to /login. */
export function SessionProvider({ children }: { children: ReactNode }): ReactElement | null {
  const me = useApi<Me>("/me");
  if (!me.data) return null;
  setOffset(me.data.utc_offset_min);
  return (
    <ReloadContext.Provider value={me.reload}>
      <SessionContext.Provider value={me.data}>{children}</SessionContext.Provider>
    </ReloadContext.Provider>
  );
}

/** Reads the signed-in user again, after a page changes something the header shows. */
export function useReloadMe(): () => void {
  return useContext(ReloadContext);
}

export function useMe(): Me {
  const me = useContext(SessionContext);
  if (!me) throw new Error("useMe outside SessionProvider");
  return me;
}

/** False for a supervisor. Outside a signed-in page (the public pages) it is true. */
export function useCanEdit(): boolean {
  return useContext(SessionContext)?.read_only !== true;
}

/** Renders its children only for someone who may change things: a create, edit or delete control. */
export function Writable({ children }: { children: ReactNode }): ReactElement | null {
  return useCanEdit() ? <>{children}</> : null;
}
