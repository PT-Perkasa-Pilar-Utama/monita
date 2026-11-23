// The post-mortem's sections in reading order, as Google's example post-mortem names them
// (v1.2 Q63). The server checks the same three for a publish.
import type { PostmortemText } from "@/lib/types";

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
