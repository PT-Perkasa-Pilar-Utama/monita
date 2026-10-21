import { expect, test } from "bun:test";
import { formatsFor, webhookFields } from "@/lib/webhookFormats";

test("each format asks for what it posts with", () => {
  expect(webhookFields("json").key).toBeNull();
  expect(webhookFields("telegram").key?.label).toBe("Chat ID");
  expect(webhookFields("pagerduty").url).toBeNull();
  expect(webhookFields("pagerduty").key?.label).toBe("Routing key");
  expect(webhookFields("ntfy").url?.label).toBe("Topic URL");
});

test("Monita's bot is offered once a bot is set, or while a route still uses it", () => {
  const has = (bot: string | null, current: Parameters<typeof formatsFor>[1]): boolean => formatsFor(bot, current).some((f) => f.value === "telegram_bot");
  expect(has(null, "json")).toBe(false);
  expect(has("MonitaBot", "json")).toBe(true);
  expect(has(null, "telegram_bot")).toBe(true);
  expect(webhookFields("telegram_bot").url).toBeNull();
  expect(webhookFields("telegram_bot").key?.label).toBe("Chat");
});
