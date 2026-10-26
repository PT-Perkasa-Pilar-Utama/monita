import type { UpdateLabel } from "@/lib/types";

/** A starting message for each label, in the status page's voice. */
export const TEMPLATES: Record<UpdateLabel, string> = {
  investigating: "We are looking into the problem and will post an update when we know more.",
  identified: "We found the cause and are working on a fix, and we will post an update when it is out.",
  monitoring: "A fix is out and we are watching to make sure it holds.",
  resolved: "The problem is fixed and everything works again.",
};

/** The message after the label changes to `next`: its template, unless the user wrote their own. */
export function nextMessage(current: string, next: UpdateLabel): string {
  const untouched = current.trim() === "" || Object.values(TEMPLATES).includes(current);
  return untouched ? TEMPLATES[next] : current;
}
