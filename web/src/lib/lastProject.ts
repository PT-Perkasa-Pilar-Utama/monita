import { readLocal, writeLocal } from "@/lib/store";

/** The project a user last opened in this browser, for the way back from Projects and Admin. */
export type LastProject = { slug: string; name: string; icon_url: string | null };

const key = (email: string): string => `monita.last-project:${email}`;

/** The stored value, or null when it is missing or not a project. */
export function parseLast(raw: string): LastProject | null {
  try {
    const v: unknown = JSON.parse(raw);
    if (typeof v !== "object" || v === null) return null;
    const { slug, name, icon_url } = v as Record<string, unknown>;
    if (typeof slug !== "string" || !slug || typeof name !== "string") return null;
    return { slug, name, icon_url: typeof icon_url === "string" ? icon_url : null };
  } catch {
    return null;
  }
}

export function lastProject(email: string): LastProject | null {
  return parseLast(readLocal(key(email)));
}

export function rememberProject(email: string, p: LastProject): void {
  writeLocal(key(email), JSON.stringify({ slug: p.slug, name: p.name, icon_url: p.icon_url }));
}

/** Forgets `slug` when it is the one stored, as when it no longer opens. */
export function forgetProject(email: string, slug: string): void {
  if (lastProject(email)?.slug === slug) writeLocal(key(email), "");
}
