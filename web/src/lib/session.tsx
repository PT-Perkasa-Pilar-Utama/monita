import type { ReactElement, ReactNode } from "react";
import { createContext, useContext } from "react";
import { useApi } from "@/lib/useApi";

export type Me = { email: string; role: "super_admin" | "tl"; is_admin: boolean };

const SessionContext = createContext<Me | null>(null);

/** Loads the signed-in user; renders nothing until it is known. A 401 has already moved to /login. */
export function SessionProvider({ children }: { children: ReactNode }): ReactElement | null {
  const me = useApi<Me>("/me");
  if (!me.data) return null;
  return <SessionContext.Provider value={me.data}>{children}</SessionContext.Provider>;
}

export function useMe(): Me {
  const me = useContext(SessionContext);
  if (!me) throw new Error("useMe outside SessionProvider");
  return me;
}
