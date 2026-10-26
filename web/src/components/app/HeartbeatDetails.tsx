import { useState } from "react";
import type { ReactElement } from "react";
import { CodeCopy, CopyField } from "@/components/app/CopyField";
import { DetailPart, DetailsSheet } from "@/components/app/DetailsSheet";
import { Facts } from "@/components/app/Facts";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { whenItFails } from "@/lib/checks";
import type { Lang } from "@/lib/highlight";
import { LANGS, pingSnippets } from "@/lib/pingSnippets";
import { readLocal, writeLocal } from "@/lib/store";
import { stamp } from "@/lib/time";
import type { HeartbeatView } from "@/lib/types";

/** The snippet language a viewer last picked, in this browser. */
const LANG_KEY = "monita.ping-lang";


/** A heartbeat's ping URL, commands to call it from a job, and its settings, in a side panel. */
export function HeartbeatDetails({ h, pingUrl, trigger }: { h: HeartbeatView; pingUrl: string; trigger?: ReactElement }): ReactElement {
  const [lang, setLang] = useState<Lang>(() => (LANGS.some((l) => l.lang === readLocal(LANG_KEY)) ? (readLocal(LANG_KEY) as Lang) : "sh"));
  const pick = (l: Lang): void => {
    setLang(l);
    writeLocal(LANG_KEY, l);
  };
  return (
    <DetailsSheet title={h.name} description="How the job calls Monita, and how this heartbeat is set up." trigger={trigger}>
          <DetailPart title="Ping URL">
            <p className="text-muted-foreground">Call it with GET, HEAD or POST when the job succeeds, and Monita ignores that body. When it fails, add /fail to the end and send the reason as plain text in the body, and Monita keeps the first 1 KB as the cause.</p>
            <CopyField id="ping-url" value={pingUrl} />
          </DetailPart>
          <DetailPart title="Call it from your job">
            <p className="text-muted-foreground">Any language works, since an HTTP request to the URL is enough, and the last example is a starting point to adapt.</p>
            <ToggleGroup type="single" variant="outline" size="sm" aria-label="Language" value={lang} onValueChange={(v) => v && pick(v as Lang)} className="self-start">
              {LANGS.map((l) => (
                <ToggleGroupItem key={l.lang} value={l.lang} className="px-3">
                  {l.label}
                </ToggleGroupItem>
              ))}
            </ToggleGroup>
            {pingSnippets(pingUrl, lang).map((s, n) => (
              <CodeCopy key={`${lang}${s.title}`} id={`ping-snippet-${n}`} title={s.title} code={s.code} lang={lang} />
            ))}
          </DetailPart>
          <DetailPart title="Settings">
            <Facts
              rows={[
                ["Runs every", h.every],
                ["Can be late by", h.grace],
                ["When it fails", whenItFails(h.alert, h.auto_incident)],
                ["Status page", h.is_public ? "Shown, with its incidents" : "Not shown"],
                ["Created", stamp(h.created_at)],
              ]}
            />
          </DetailPart>
    </DetailsSheet>
  );
}
