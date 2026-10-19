import { expect, test } from "bun:test";
import { webhookFields } from "@/lib/webhookFormats";

test("each format asks for what it posts with", () => {
  expect(webhookFields("json").key).toBeNull();
  expect(webhookFields("telegram").key?.label).toBe("Chat ID");
  expect(webhookFields("pagerduty").url).toBeNull();
  expect(webhookFields("pagerduty").key?.label).toBe("Routing key");
  expect(webhookFields("ntfy").url?.label).toBe("Topic URL");
});
