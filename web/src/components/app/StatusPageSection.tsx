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
  { key: "uptime", title: "Uptime", description: "Shows how each public system has done over the last 90 days. This tab is always on.", locked: true },
  { key: "incidents", title: "Incidents", description: "Shows current problems and past incidents, with the updates your team posts." },
  { key: "known_issues", title: "Issues", description: "Shows the bugs your team is working on. Visitors see only the title." },
  { key: "report", title: "Ticket", description: "Lets people report a bug and follow its progress with their ref ID." },
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
      title="Public tabs"
      description="Choose what people see on your status page. Only the monitors and heartbeats you mark as public appear there."
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
        legend="Public tabs"
        cards={TABS}
        value={{ ...pages, uptime: true }}
        onChange={(k, on) => k !== "uptime" && setPages({ ...pages, [k]: on })}
      />
    </SettingsSection>
  );
}
