import type { ReactElement, ReactNode } from "react";
import { createContext, useContext } from "react";
import type { ProjectHead } from "@/lib/types";

type ProjectState = ProjectHead & { reload: () => void };

const ProjectContext = createContext<ProjectState | null>(null);

export function ProjectProvider({ value, children }: { value: ProjectState; children: ReactNode }): ReactElement {
  return <ProjectContext.Provider value={value}>{children}</ProjectContext.Provider>;
}

/** The project the page is in, with its tab counts; `reload` reads both again. */
export function useProject(): ProjectState {
  const p = useContext(ProjectContext);
  if (!p) throw new Error("useProject outside ProjectLayout");
  return p;
}
