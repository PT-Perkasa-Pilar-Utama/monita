import { expect, test } from "bun:test";
import { cardEvents, cardTarget, kindLabel, levelLine, unheard } from "@/lib/alertCards";
import type { AlertCard } from "@/lib/types";

const card = (over: Partial<AlertCard>): AlertCard => ({ id: 1, kind: "email", name: "On call", critical: true, warning: false, new_tickets: false, ticket_moves: false, digest: false, section: "incidents", enabled: true, emails: ["a@x.co"], url: null, key: null, fixed: false, ...over });

test("a ticket card names its events and never counts for a level", () => {
  const t = card({ section: "tickets", critical: false, new_tickets: true, ticket_moves: true });
  expect(cardEvents(t)).toEqual(["New tickets", "Ticket moves"]);
  expect(cardEvents(card({ section: "tickets", critical: false, digest: true }))).toEqual(["Daily digest"]);
  expect(unheard([t])).toEqual(["critical", "warning"]);
  expect(levelLine([t], "critical")).toBe("Critical: no one is alerted.");
});

test("each level counts the cards that are on and listen to it", () => {
  const cards = [card({}), card({ id: 2, warning: true }), card({ id: 3, enabled: false, warning: true, critical: false })];
  expect(levelLine(cards, "critical")).toBe("Critical: 2 cards.");
  expect(levelLine(cards, "warning")).toBe("Warning: 1 card.");
  expect(levelLine([], "warning")).toBe("Warning: no one is alerted.");
});

test("a level no card that is on listens to is unheard", () => {
  expect(unheard([card({})])).toEqual(["warning"]);
  expect(unheard([card({ enabled: false })])).toEqual(["critical", "warning"]);
  expect(unheard([card({ warning: true })])).toEqual([]);
});

test("a card says what sends and where", () => {
  expect(kindLabel("owner_email")).toBe("The owner's email");
  expect(kindLabel("discord")).toBe("Discord");
  expect(cardTarget(card({ kind: "owner_email" }), "rina@x.co")).toBe("rina@x.co");
  expect(cardTarget(card({ emails: ["a@x.co", "b@x.co"] }), "")).toBe("a@x.co, b@x.co");
  expect(cardTarget(card({ kind: "discord", url: "https://discord.com/api/webhooks/1/x" }), "")).toBe("discord.com");
  expect(cardTarget(card({ kind: "telegram_bot", url: "https://api.telegram.org/", key: "4242" }), "")).toBe("Chat 4242");
  expect(cardTarget(card({ kind: "json", url: "(hidden)" }), "")).toBe("Hidden from supervisors");
});
