// Alert cards in words, for the Alerts page and the project's About column (v1.1 Q34, Q37).
import type { AlertCard, CardKind } from "@/lib/types";
import { WEBHOOK_FORMATS } from "@/lib/webhookFormats";

export type Level = "critical" | "warning";
export const LEVELS: Level[] = ["critical", "warning"];

const LEVEL_WORD: Record<Level, string> = { critical: "Critical", warning: "Warning" };

/** "Critical: 3 cards." for one level, counting the cards that are on and listen to it. */
export function levelLine(cards: AlertCard[], level: Level): string {
  const n = cards.filter((c) => c.enabled && c[level]).length;
  if (n === 0) return `${LEVEL_WORD[level]}: no one is alerted.`;
  return `${LEVEL_WORD[level]}: ${n === 1 ? "1 card" : `${n} cards`}.`;
}

/** The levels that no card that is on listens to: their alerts go to no one. */
export function unheard(cards: AlertCard[]): Level[] {
  return LEVELS.filter((l) => !cards.some((c) => c.enabled && c[l]));
}

export type TicketEvent = "digest" | "new_tickets" | "ticket_moves";

const EVENT_WORD: Record<TicketEvent, string> = { digest: "Daily digest", new_tickets: "New tickets", ticket_moves: "Ticket moves" };

/** The ticket events a card listens to, in words. */
export function cardEvents(card: AlertCard): string[] {
  return (Object.keys(EVENT_WORD) as TicketEvent[]).filter((e) => card[e]).map((e) => EVENT_WORD[e]);
}

/** What sends the card's alerts, in words. */
export function kindLabel(kind: CardKind): string {
  if (kind === "owner_email") return "The owner's email";
  if (kind === "email") return "Email";
  if (kind === "json") return "JSON webhook";
  return WEBHOOK_FORMATS.find((f) => f.value === kind)?.label ?? kind;
}

/** Where the card sends: the addresses, the webhook's host, or the chat. */
export function cardTarget(card: AlertCard, ownerEmail: string): string {
  if (card.kind === "owner_email") return ownerEmail;
  if (card.kind === "email") return card.emails.join(", ");
  if (card.url === "(hidden)") return "Hidden from supervisors";
  if (card.kind === "telegram_bot" || card.kind === "telegram") return card.key ? `Chat ${card.key}` : "No chat yet";
  if (card.kind === "pagerduty") return "PagerDuty Events API";
  try {
    return new URL(card.url ?? "").host;
  } catch {
    return card.url ?? "";
  }
}
