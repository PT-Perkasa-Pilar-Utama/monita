import { useState } from "react";
import type { ReactElement } from "react";
import { toast } from "sonner";
import { PostmortemSheet } from "@/components/app/PostmortemSheet";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Spinner } from "@/components/ui/spinner";
import { api, errorText } from "@/lib/api";
import { SECTIONS } from "@/lib/postmortem";
import { Writable } from "@/lib/session";
import { stamp } from "@/lib/time";
import type { Postmortem } from "@/lib/types";
import { useApi } from "@/lib/useApi";

/** A resolved incident's post-mortem (v1.2 Q63): the written sections, or a button to write them. */
export function PostmortemPanel({ path }: { path: string }): ReactElement | null {
  const res = useApi<Postmortem>(`${path}/postmortem`);
  const [editing, setEditing] = useState(false);
  const [busy, setBusy] = useState(false);
  const pm = res.data;
  if (!pm) return null;

  // A published post-mortem goes back to draft before it changes.
  const edit = async (): Promise<void> => {
    setBusy(true);
    try {
      if (pm.state === "published") await api(`${path}/postmortem/edit`, "POST");
      res.reload();
      setEditing(true);
    } catch (e) {
      toast.error(errorText(e));
    } finally {
      setBusy(false);
    }
  };
  const written = SECTIONS.filter((s) => pm[s.key].trim() !== "");

  return (
    <section aria-labelledby="postmortem" className="flex flex-col gap-4">
      <div className="flex flex-wrap items-center gap-3">
        <h2 id="postmortem" className="text-[15px] font-semibold">
          Post-mortem
        </h2>
        {pm.state !== "none" && <Badge variant={pm.state === "published" ? "up" : "unknown"}>{pm.state === "published" ? "Published" : "Draft"}</Badge>}
        <span className="flex-1" />
        <Writable>
          <Button variant="outline" size="sm" disabled={busy} onClick={() => void edit()}>
            {busy && <Spinner data-icon="inline-start" />}
            {pm.state === "none" ? "Write post-mortem" : pm.state === "draft" ? "Continue the draft" : "Edit"}
          </Button>
        </Writable>
      </div>
      {pm.state === "none" ? (
        <p className="text-sm text-muted-foreground">Nobody has written the post-mortem of this incident yet.</p>
      ) : (
        <Card className="gap-4 p-5">
          {written.length === 0 ? (
            <p className="text-sm text-muted-foreground">The draft is empty.</p>
          ) : (
            written.map((s) => (
              <div key={s.key} className="flex flex-col gap-1">
                <h3 className="text-sm font-semibold">{s.label}</h3>
                <p className="text-sm whitespace-pre-wrap break-words">{pm[s.key]}</p>
              </div>
            ))
          )}
          {pm.updated_by && pm.updated_at && (
            <p className="text-[13px] text-muted-foreground">
              {pm.state === "published" && pm.published_at ? `Published ${stamp(pm.published_at)}. ` : ""}Last changed by {pm.updated_by} on {stamp(pm.updated_at)}.
            </p>
          )}
        </Card>
      )}
      {editing && res.data && <PostmortemSheet key={res.data.updated_at ?? 0} path={path} pm={res.data} open={editing} onClose={() => setEditing(false)} onSaved={res.reload} />}
    </section>
  );
}
