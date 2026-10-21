// The webhook formats an alert route can use (v1.1 Q17), and the fields each one needs.

export type WebhookFormat = "json" | "discord" | "telegram" | "telegram_bot" | "ntfy" | "pagerduty";

export const WEBHOOK_FORMATS: { value: WebhookFormat; label: string }[] = [
  { value: "json", label: "JSON (Slack, Google Chat, any receiver)" },
  { value: "discord", label: "Discord" },
  { value: "telegram_bot", label: "Global Telegram" },
  { value: "telegram", label: "Telegram" },
  { value: "ntfy", label: "ntfy" },
  { value: "pagerduty", label: "PagerDuty" },
];

export type WebhookFields = {
  /** Null when the format's URL is fixed, as PagerDuty's is. */
  url: { label: string; placeholder: string } | null;
  /** The chat ID or routing key, when the format needs one. */
  key: { label: string; placeholder: string } | null;
};

/** What the form asks for, for one format. */
export function webhookFields(format: WebhookFormat): WebhookFields {
  switch (format) {
    case "discord":
      return { url: { label: "Discord webhook URL", placeholder: "https://discord.com/api/webhooks/…" }, key: null };
    case "telegram_bot":
      return { url: null, key: { label: "Chat", placeholder: "Connect a chat, or enter its ID" } };
    case "telegram":
      return { url: { label: "Bot URL", placeholder: "https://api.telegram.org/bot<token>/sendMessage" }, key: { label: "Chat ID", placeholder: "-1001234567890 or @team_alerts" } };
    case "ntfy":
      return { url: { label: "Topic URL", placeholder: "https://ntfy.sh/team-alerts" }, key: null };
    case "pagerduty":
      return { url: null, key: { label: "Routing key", placeholder: "The integration key of an Events API v2 service" } };
    default:
      return { url: { label: "Webhook URL", placeholder: "https://hooks.slack.com/services/…" }, key: null };
  }
}
