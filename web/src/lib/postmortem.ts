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

/** A finished post-mortem of a made-up incident, shown beside the form so a tech lead sees how
 * each section reads. Never written into the form, so it cannot be published by mistake. */
export const EXAMPLE: PostmortemText = {
  summary:
    "INC-42: Checkout API is down. Card payments failed for 38 minutes, from 14:02 to 14:40, because the payment service ran out of database connections after a release. Rolling the release back restored payments.",
  impact: "About 1 900 customers could not pay by card, and 312 orders failed. Bank transfers and the rest of the shop kept working. Support received 46 tickets.",
  root_cause:
    "Release 3.8 opened a new database connection for each fraud check and never returned it to the pool. At the normal afternoon load the pool of 50 connections was used up in 9 minutes, and every later request waited until it timed out.",
  trigger: "Release 3.8 was deployed at 13:53. The load test before it ran at a tenth of real traffic, so the leak did not show.",
  resolution:
    "The on-call engineer rolled back to release 3.7 at 14:36, and payments recovered at 14:40 when the old connections timed out. Release 3.8.1, which returns each connection after the check, shipped the next day.",
  detection:
    "Monita opened INC-42 at 14:02 after two failed checks of the Checkout API, and the critical alert reached the on-call engineer by email and Telegram. Linking the failure to the release took 34 minutes, because the deploy was not posted in the incident channel.",
  action_items:
    "- Return each connection after the fraud check - prevent - payments team (done in 3.8.1)\n- Alert when the connection pool is over 80% used - mitigate - platform team\n- Load test at full afternoon traffic before a release - prevent - QA\n- Post every deploy in the incident channel - process - release manager",
  lessons:
    "What went well: Monita alerted within 2 minutes, and the rollback took 4 minutes.\nWhat went wrong: nobody linked the failure to the release for half an hour.\nWhere we got lucky: the release went out on a weekday afternoon, with the whole team at work.",
};
