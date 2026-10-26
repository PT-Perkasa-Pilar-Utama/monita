import { useState } from "react";
import type { ReactElement } from "react";
import { AlertCardSheet } from "@/components/app/AlertCardSheet";
import { AlertsGuide } from "@/components/app/AlertsGuide";
import { AlertSwitch } from "@/components/app/AlertSwitch";
import { SettingsPage, SettingsSection } from "@/components/app/SettingsSection";
import { SeverityBadge } from "@/components/app/Tone";
import { Plus, Send } from "@/components/icons";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { Spinner } from "@/components/ui/spinner";
import { api, errorText } from "@/lib/api";
import { cardEvents, cardTarget, kindLabel, unheard } from "@/lib/alertCards";
import { useProject } from "@/lib/project";
import { Writable } from "@/lib/session";
import type { AlertCard, CardSection } from "@/lib/types";
import { useApi } from "@/lib/useApi";
import { toast } from "sonner";

function testNotice(n: number): string {
  if (n === 0) return "This card has no one to send to.";
  return n === 1 ? "Test sent to 1 recipient." : `Test sent to ${n} recipients.`;
}

/** One card: its name and kind, the levels it hears, where it sends, its switch and its actions. */
function CardTile({ slug, card, owner, onEdit, onToggle }: { slug: string; card: AlertCard; owner: string; onEdit: () => void; onToggle: (on: boolean) => void }): ReactElement {
  const [testing, setTesting] = useState(false);
  const test = async (): Promise<void> => {
    setTesting(true);
    try {
      const { sent_to: n } = await api<{ sent_to: number }>(`/projects/${slug}/alert-cards/${card.id}/test`, "POST");
      toast.success(testNotice(n));
    } catch (err) {
      toast.error(errorText(err));
    } finally {
      setTesting(false);
    }
  };
  return (
    <li id={`card-${card.id}`} aria-label={card.name} className="flex flex-col gap-3 rounded-lg border bg-paper p-4">
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0">
          <p className="truncate font-medium">{card.name}</p>
          {/* The kind, unless the name already says it. */}
          {!card.fixed && card.name !== kindLabel(card.kind) && <p className="text-sm text-muted-foreground">{kindLabel(card.kind)}</p>}
        </div>
        <AlertSwitch path={`/projects/${slug}/alert-cards/${card.id}/enabled`} id={`switch-${card.id}`} name={card.name} on={card.enabled} onChange={onToggle} />
      </div>
      <div className="flex flex-wrap gap-1.5">
        {card.critical && <SeverityBadge severity="critical" />}
        {card.warning && <SeverityBadge severity="warning" />}
        {cardEvents(card).map((e) => (
          <Badge key={e} variant="outline">
            {e}
          </Badge>
        ))}
      </div>
      <p className="truncate text-sm text-muted-foreground" title={cardTarget(card, owner)}>
        {card.fixed ? `${owner}, the project owner` : cardTarget(card, owner)}
      </p>
      <Writable>
        <div className="flex gap-2">
          <Button type="button" variant="outline" size="sm" disabled={testing} onClick={() => void test()}>
            {testing ? <Spinner data-icon="inline-start" /> : <Send data-icon="inline-start" />}
            Send a test
          </Button>
          {!card.fixed && (
            <Button type="button" variant="ghost" size="sm" onClick={onEdit}>
              Edit
            </Button>
          )}
        </div>
      </Writable>
    </li>
  );
}

const SECTIONS: Record<CardSection, { title: string; description: string; add: string }> = {
  incidents: { title: "Alert cards", description: "Each card sends alerts for the levels it listens to.", add: "Add alert card" },
  tickets: {
    title: "Ticket alerts",
    description: "Each card sends new bug reports from your users, each move on the board, or both, and a ticket your team files sends no new-ticket alert.",
    add: "Add ticket card",
  },
};

/** Settings, Alerts: the incident cards, then the ticket cards. The guide opens beside it. */
export function Alerts(): ReactElement {
  const { project: p } = useProject();
  const cards = useApi<AlertCard[]>(`/projects/${p.slug}/alert-cards`);
  const telegram = useApi<{ bot: string | null }>("/telegram");
  // The sheet: closed, or a section with the card to edit (null: a new card).
  const [editing, setEditing] = useState<{ section: CardSection; card: AlertCard | null } | null>(null);
  // A switch flipped on this page, until the next read of the cards.
  const [flipped, setFlipped] = useState<Record<number, boolean>>({});
  const list = (cards.data ?? []).map((c) => (c.id in flipped ? { ...c, enabled: flipped[c.id]! } : c));
  const deaf = unheard(list);

  return (
    <SettingsPage
      title="Alerts"
      actions={<AlertsGuide slug={p.slug} />}
      toc={[
        { id: "incidents", label: SECTIONS.incidents.title },
        { id: "tickets", label: SECTIONS.tickets.title },
      ]}
    >
      {cards.error && (
        <Alert variant="destructive">
          <AlertDescription>{cards.error.message}</AlertDescription>
        </Alert>
      )}
      {(["incidents", "tickets"] as const).map((section) => (
        <SettingsSection
          key={section}
          id={section}
          title={SECTIONS[section].title}
          description={SECTIONS[section].description}
          action={
            <Writable>
              <Button type="button" onClick={() => setEditing({ section, card: null })}>
                <Plus data-icon="inline-start" />
                {SECTIONS[section].add}
              </Button>
            </Writable>
          }
        >
          {!cards.data || !telegram.data ? (
            <Skeleton className="h-48 rounded-lg" />
          ) : (
            <div className="flex flex-col gap-4">
              {section === "incidents" &&
                deaf.map((l) => (
                  <Alert key={l}>
                    <AlertDescription>No card listens to {l} alerts, so nobody hears them.</AlertDescription>
                  </Alert>
                ))}
              <ul aria-label={SECTIONS[section].title} className="grid gap-4 md:grid-cols-2">
                {list
                  .filter((c) => c.section === section)
                  .map((c) => (
                    <CardTile key={c.id} slug={p.slug} card={c} owner={p.owner_email} onEdit={() => setEditing({ section, card: c })} onToggle={(on) => setFlipped((f) => ({ ...f, [c.id]: on }))} />
                  ))}
              </ul>
            </div>
          )}
        </SettingsSection>
      ))}
      {editing && (
        <AlertCardSheet
          key={editing.card?.id ?? `new-${editing.section}`}
          slug={p.slug}
          bot={telegram.data?.bot ?? null}
          section={editing.section}
          card={editing.card}
          open
          onClose={() => setEditing(null)}
          onSaved={() => {
            setEditing(null);
            setFlipped({});
            cards.reload();
          }}
        />
      )}
    </SettingsPage>
  );
}
