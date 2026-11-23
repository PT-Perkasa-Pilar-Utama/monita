// The post-mortem's sections in reading order, as Google's example post-mortem names them
// (v1.2 Q63). The server checks the same three for a publish.
import { duration, stamp } from "@/lib/time";
import type { IncidentDetail, PostmortemText } from "@/lib/types";

export const SECTIONS: { key: keyof PostmortemText; label: string; required: boolean }[] = [
  { key: "summary", label: "Summary", required: true },
  { key: "impact", label: "Impact", required: false },
  { key: "root_cause", label: "Root cause", required: true },
  { key: "trigger", label: "Trigger", required: false },
  { key: "resolution", label: "Resolution", required: true },
  { key: "detection", label: "Detection", required: false },
  { key: "action_items", label: "Action items", required: false },
  { key: "lessons", label: "Lessons learned", required: false },
];

/** What the publish dialog says about the emails it sends. */
export function publishNote(recipients: number): string {
  if (recipients === 0) return "Nobody is on the post-mortem list under Settings, Reports, so no email goes out.";
  return recipients === 1 ? "Monita emails the PDF to the 1 address on the post-mortem list." : `Monita emails the PDF to the ${recipients} addresses on the post-mortem list.`;
}

// The prompts a template leaves for the tech lead to replace. Publish is refused while one is
// still in the text (checked in the browser only: only this form writes them).
const PROMPT = {
  summary: "[What users saw, and what fixed it, in one or two sentences.]",
  impact: "[Who was affected and how: which users or teams, how many, and what they could not do.]",
  root_cause: "[The fault underneath. Ask why until you reach something the team can fix.]",
  trigger: "[The change or event that set the fault off, such as a deploy, a config change or a traffic spike.]",
  resolution: "[What stopped the outage, who did it, and when.]",
  detection: "[How the team learned of it, and how it could be found sooner.]",
  action: "[Action] - [prevent, mitigate or process] - [owner]",
  well: "What went well: [ ]",
  wrong: "What went wrong: [ ]",
  lucky: "Where we got lucky: [ ]",
} as const;
const PROMPTS = Object.values(PROMPT).flatMap((p) => p.match(/\[[^\]]*\]/g) ?? []);

// Facts are cut so a filled section stays well under the 2 000-character cap.
function cut(s: string, n: number): string {
  return s.length > n ? `${s.slice(0, n - 3)}...` : s;
}

/** A first draft from what Monita knows about the incident, with a bracketed prompt where the
 * tech lead must write. Times are on the viewer's clock, as the page shows them. */
export function template(d: IncidentDetail): PostmortemText {
  const i = d.incident;
  const end = i.resolved_at ?? i.started_at;
  const span = `from ${stamp(i.started_at)} to ${stamp(end)}, ${duration(end - i.started_at)}`;
  const items = d.items.length === 0 ? "No monitor or heartbeat was named" : `${cut(d.items.join(", "), 400)} ${d.items.length === 1 ? "was" : "were"} affected`;
  const cause = i.cause ? cut(i.cause, 500) : null;
  return {
    summary: `INC-${i.id}: ${cut(i.title, 200)}. ${items} ${span}.\n${PROMPT.summary}`,
    impact: PROMPT.impact,
    root_cause: cause ? `Monita saw: ${cause}\n${PROMPT.root_cause}` : PROMPT.root_cause,
    trigger: PROMPT.trigger,
    resolution: `The incident was resolved at ${stamp(end)}.\n${PROMPT.resolution}`,
    detection: `${i.origin === "auto" ? `Monita opened INC-${i.id} at ${stamp(i.started_at)}${cause ? ` after: ${cause}` : ""}.` : `INC-${i.id} was reported by hand at ${stamp(i.started_at)}.`}\n${PROMPT.detection}`,
    action_items: `- ${PROMPT.action}\n- ${PROMPT.action}`,
    lessons: [PROMPT.well, PROMPT.wrong, PROMPT.lucky].join("\n"),
  };
}

/** `text` with each empty section taken from `filled`; typed text is never replaced. */
export function fillEmpty(text: PostmortemText, filled: PostmortemText): PostmortemText {
  return Object.fromEntries(SECTIONS.map(({ key }) => [key, text[key].trim() === "" ? filled[key] : text[key]])) as PostmortemText;
}

/** The sections that still hold a template prompt. */
export function unfilled(text: PostmortemText): (keyof PostmortemText)[] {
  return SECTIONS.filter(({ key }) => PROMPTS.some((p) => text[key].includes(p))).map((s) => s.key);
}

export const UNFILLED = "Replace the text in square brackets before you publish.";
