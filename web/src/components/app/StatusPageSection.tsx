import { useState } from "react";
import type { ReactElement } from "react";
import { SettingsSection } from "@/components/app/SettingsSection";
import { ExternalLink } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Field, FieldLabel, FieldLegend, FieldSet } from "@/components/ui/field";
import { Spinner } from "@/components/ui/spinner";
import { api, errorText } from "@/lib/api";
import { useProject } from "@/lib/project";
import type { PublicPages } from "@/lib/types";
import { toast } from "sonner";

const TABS: { key: keyof PublicPages; label: string }[] = [
  { key: "known_issues", label: "Known issues: the tickets your team has sorted, by title only" },
  { key: "report", label: "Report a bug: the form on the status page. Your app can still file tickets through the API." },
  { key: "mine", label: "My tickets: a reporter looks up their own tickets by ref ID" },
];

/** The status page's address and which tabs visitors see. Status always shows. */
export function StatusPageSection(): ReactElement {
  const { project: p, reload } = useProject();
  const [pages, setPages] = useState<PublicPages>(p.pages);
  const [busy, setBusy] = useState(false);
  const changed = TABS.some((t) => pages[t.key] !== p.pages[t.key]);
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
      <FieldSet className="gap-3">
        <FieldLegend className="sr-only">Tabs visitors see</FieldLegend>
        <Field orientation="horizontal" data-disabled>
          <Checkbox id="tab-status" checked disabled />
          <FieldLabel htmlFor="tab-status" className="font-normal">
            Status: always shown
          </FieldLabel>
        </Field>
        {TABS.map((t) => (
          <Field key={t.key} orientation="horizontal">
            <Checkbox id={`tab-${t.key}`} checked={pages[t.key]} onCheckedChange={(v) => setPages({ ...pages, [t.key]: v === true })} />
            <FieldLabel htmlFor={`tab-${t.key}`} className="font-normal">
              {t.label}
            </FieldLabel>
          </Field>
        ))}
      </FieldSet>
    </SettingsSection>
  );
}
