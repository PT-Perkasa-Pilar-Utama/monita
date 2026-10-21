import { useCallback, useEffect, useRef, useState } from "react";
import type { ReactElement } from "react";
import { Button } from "@/components/ui/button";
import { Spinner } from "@/components/ui/spinner";
import { api, errorText } from "@/lib/api";
import { nowSecs } from "@/lib/time";

type Link = { code: string; private: string; group: string; qr: string; expires_at: number };
export type TelegramChat = { chat_id: string; title: string };

/** How often the panel asks whether the bot heard the link. */
const POLL_MS = 3000;

/**
 * Connects a chat to the global Telegram bot, in place: a QR code and two links. The person opens
 * one in Telegram and selects Start. The panel asks the server until the bot hears it.
 */
export function TelegramConnect({ slug, bot, onChat }: { slug: string; bot: string; onChat: (chat: TelegramChat) => void }): ReactElement {
  const [link, setLink] = useState<Link | null>(null);
  const [error, setError] = useState("");
  const [expired, setExpired] = useState(false);
  const base = `/projects/${slug}/telegram-link`;
  // The parent passes a new function each render. The poll should not restart for it.
  const onChatRef = useRef(onChat);
  onChatRef.current = onChat;

  const start = useCallback((): void => {
    setLink(null);
    setError("");
    setExpired(false);
    api<Link>(base, "POST")
      .then(setLink)
      .catch((e: unknown) => setError(errorText(e)));
  }, [base]);

  useEffect(start, [start]);

  useEffect(() => {
    if (!link) return;
    const timer = window.setInterval(() => {
      if (nowSecs() >= link.expires_at) {
        window.clearInterval(timer);
        setExpired(true);
        return;
      }
      api<{ chat: TelegramChat | null }>(`${base}/check`, "POST", { code: link.code })
        .then(({ chat }) => {
          if (!chat) return;
          window.clearInterval(timer);
          onChatRef.current(chat);
        })
        .catch((e: unknown) => {
          window.clearInterval(timer);
          setError(errorText(e));
        });
    }, POLL_MS);
    return () => window.clearInterval(timer);
  }, [link, base]);

  if (error) return <p className="text-sm text-destructive">{error}</p>;
  if (expired) {
    return (
      <div className="flex items-center gap-3 text-sm text-muted-foreground">
        The link expired.
        <Button type="button" variant="outline" size="sm" onClick={start}>
          Get a new link
        </Button>
      </div>
    );
  }
  if (!link) return <Spinner />;
  return (
    <div className="flex flex-col gap-4 rounded-lg border bg-paper p-4 sm:flex-row sm:items-center">
      <img id="telegram-qr" src={`data:image/svg+xml;utf8,${encodeURIComponent(link.qr)}`} alt="QR code for the Telegram link" className="w-fit shrink-0 rounded-md border" />
      <div className="flex flex-col gap-3 text-sm">
        <p>Scan the code or open the link, then select Start in Telegram.</p>
        <div className="flex flex-wrap gap-2">
          <Button asChild size="sm">
            <a href={link.private} target="_blank" rel="noreferrer">
              Open in Telegram
            </a>
          </Button>
          <Button asChild size="sm" variant="outline">
            <a href={link.group} target="_blank" rel="noreferrer">
              Add @{bot} to a group
            </a>
          </Button>
        </div>
        <p className="flex items-center gap-2 text-muted-foreground">
          <Spinner /> Waiting for Telegram.
        </p>
      </div>
    </div>
  );
}
