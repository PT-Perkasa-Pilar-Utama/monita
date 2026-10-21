import { useEffect, useRef, useState } from "react";
import type { ReactElement } from "react";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle, DialogTrigger } from "@/components/ui/dialog";
import { Spinner } from "@/components/ui/spinner";
import { api, errorText } from "@/lib/api";
import { nowSecs } from "@/lib/time";

type Link = { code: string; private: string; group: string; qr: string; expires_at: number };
export type TelegramChat = { chat_id: string; title: string };

/** How often the dialog asks whether the bot heard the link. */
const POLL_MS = 3000;

/**
 * Connects a chat to Monita's bot: a link and its QR code. The person opens it in Telegram, or
 * adds the bot to a group with it; the dialog asks the server until the bot hears it.
 */
export function TelegramConnect({ slug, bot, onChat }: { slug: string; bot: string; onChat: (chat: TelegramChat) => void }): ReactElement {
  const [open, setOpen] = useState(false);
  const [link, setLink] = useState<Link | null>(null);
  const [error, setError] = useState("");
  const base = `/projects/${slug}/telegram-link`;
  // The parent passes a new function each render; the poll should not restart for it.
  const onChatRef = useRef(onChat);
  onChatRef.current = onChat;

  useEffect(() => {
    if (!open) return;
    let live = true;
    setLink(null);
    setError("");
    api<Link>(base, "POST")
      .then((l) => live && setLink(l))
      .catch((e: unknown) => live && setError(errorText(e)));
    return () => {
      live = false;
    };
  }, [open, base]);

  useEffect(() => {
    if (!open || !link) return;
    const timer = window.setInterval(() => {
      if (nowSecs() >= link.expires_at) {
        window.clearInterval(timer);
        setError("This link expired. Close this and connect again.");
        return;
      }
      api<{ chat: TelegramChat | null }>(`${base}/check`, "POST", { code: link.code })
        .then(({ chat }) => {
          if (!chat) return;
          window.clearInterval(timer);
          setOpen(false);
          onChatRef.current(chat);
        })
        .catch((e: unknown) => {
          window.clearInterval(timer);
          setError(errorText(e));
        });
    }, POLL_MS);
    return () => window.clearInterval(timer);
  }, [open, link, base]);

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        <Button type="button" variant="outline" className="w-fit">
          Connect a chat
        </Button>
      </DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Connect a Telegram chat</DialogTitle>
          <DialogDescription>Scan the code with your phone, or open the link, then select Start. To alert a group, add @{bot} to it with the group link. The link works for 10 minutes.</DialogDescription>
        </DialogHeader>
        {error ? (
          <p className="text-sm text-destructive">{error}</p>
        ) : !link ? (
          <Spinner />
        ) : (
          <div className="flex flex-col items-start gap-4">
            <img id="telegram-qr" src={`data:image/svg+xml;utf8,${encodeURIComponent(link.qr)}`} alt="QR code for the Telegram link" className="w-fit rounded-md border" />
            <div className="flex flex-wrap gap-2">
              <Button asChild>
                <a href={link.private} target="_blank" rel="noreferrer">
                  Open in Telegram
                </a>
              </Button>
              <Button asChild variant="outline">
                <a href={link.group} target="_blank" rel="noreferrer">
                  Add to a group
                </a>
              </Button>
            </div>
            <p className="flex items-center gap-2 text-sm text-muted-foreground">
              <Spinner /> Waiting for Telegram.
            </p>
          </div>
        )}
      </DialogContent>
    </Dialog>
  );
}
