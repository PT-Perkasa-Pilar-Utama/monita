import { useState } from "react";
import type { ReactElement } from "react";
import { CheckCards } from "@/components/app/ChoiceCards";
import type { CheckCard } from "@/components/app/ChoiceCards";
import { SettingsSection } from "@/components/app/SettingsSection";
import { ExternalLink } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { Spinner } from "@/components/ui/spinner";
import { api, errorText } from "@/lib/api";
import { useProject } from "@/lib/project";
import type { PublicPages } from "@/lib/types";
import { toast } from "sonner";

type TabKey = keyof PublicPages | "uptime";

const TABS: CheckCard<TabKey>[] = [
  { key: "uptime", title: "Uptime", description: "Each public system with 90 days of bars. Always shown.", locked: true },
  { key: "incidents", title: "Incidents", description: "What is wrong now and every past incident, with your updates." },
  { key: "known_issues", title: "Issues", description: "The tickets your team has sorted, by title only." },
  { key: "report", title: "Ticket", description: "A reporter files a bug and checks their own tickets with their ref ID. Your app can still file through the API." },
];
const KEYS: (keyof PublicPages)[] = ["incidents", "known_issues", "report"];

/** The status page's address and which tabs visitors see. Uptime always shows. */
export function StatusPageSection(): ReactElement {
  const { project: p, reload } = useProject();
  const [pages, setPages] = useState<PublicPages>(p.pages);
  const [busy, setBusy] = useState(false);
  const changed = KEYS.some((k) => pages[k] !== p.pages[k]);
  const save = async (): Promise<void> => {
    setBusy(true);
    try {
      await api(`/projects/${p.slug}/public-pages`, "PUT", pages);
      reload();
      toast.success("Status page saved.");
    } catch (err) {
      toast.error(errorText(err));
    } finally {
      setBusy(false);
    }
  };
  return (
    <SettingsSection
      id="tabs"
      title="Tabs visitors see"
      description="The page shows the monitors and heartbeats you make public. Turn off a tab your team does not use."
      footerNote={
        <a href={`/status/${p.slug}`} target="_blank" rel="noreferrer" className="inline-flex items-center gap-1 font-mono break-all text-link underline-offset-4 hover:underline">
          {window.location.origin}/status/{p.slug}
          <ExternalLink className="size-3.5 shrink-0" />
        </a>
      }
      footer={
        <Button onClick={() => void save()} disabled={busy || !changed}>
          {busy && <Spinner data-icon="inline-start" />}
          Save changes
        </Button>
      }
    >
      <CheckCards
        id="tab"
        legend="Tabs visitors see"
        cards={TABS}
        value={{ ...pages, uptime: true }}
        onChange={(k, on) => k !== "uptime" && setPages({ ...pages, [k]: on })}
      />
    </SettingsSection>
  );
}
