import { useState } from "react";
import type { FormEvent, ReactElement } from "react";
import { useParams } from "react-router-dom";
import { Facts } from "@/components/app/Facts";
import { TableCard } from "@/components/app/TableCard";
import { labelTitle, LabelBadge, SeverityBadge, StateBadge } from "@/components/app/Tone";
import { Crumbs, PageHeader } from "@/components/layout/Page";
import { TableEmpty } from "@/components/list/TableEmpty";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Field, FieldError, FieldGroup, FieldLabel, FieldLegend, FieldSet } from "@/components/ui/field";
import { Skeleton } from "@/components/ui/skeleton";
import { Spinner } from "@/components/ui/spinner";
import { Textarea } from "@/components/ui/textarea";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { api, ApiError, errorText } from "@/lib/api";
import { useProject } from "@/lib/project";
import { day, duration, hhmm, nowSecs, utc } from "@/lib/time";
import type { IncidentDetail, UpdateLabel } from "@/lib/types";
import { useApi } from "@/lib/useApi";
import { NotFound } from "@/pages/NotFound";
import { toast } from "sonner";

function PostUpdate({ path, labels, closed, onPosted }: { path: string; labels: UpdateLabel[]; closed: boolean; onPosted: () => void }): ReactElement {
  const [open, setOpen] = useState(false);
  const [label, setLabel] = useState<UpdateLabel>(closed ? "resolved" : "monitoring");
  const [message, setMessage] = useState("");
  const [error, setError] = useState<ApiError | null>(null);
  const [busy, setBusy] = useState(false);
  const submit = async (e: FormEvent): Promise<void> => {
    e.preventDefault();
    setBusy(true);
    setError(null);
    try {
      await api(`${path}/updates`, "POST", { label, message });
      setMessage("");
      setOpen(false);
      onPosted();
      toast.success("Update posted.");
    } catch (err) {
      setError(err instanceof ApiError ? err : new ApiError(0, "error", errorText(err)));
    } finally {
      setBusy(false);
    }
  };
  const labelError = error?.fields.label;
  const messageError = error?.fields.message ?? (error && !labelError ? error.message : undefined);
  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <Button onClick={() => setOpen(true)}>Post update</Button>
      <DialogContent>
        <form onSubmit={(e) => void submit(e)} noValidate className="flex flex-col gap-6">
          <DialogHeader>
            <DialogTitle>Post an update</DialogTitle>
            <DialogDescription>The update appears on the status page without your name.</DialogDescription>
          </DialogHeader>
          <FieldGroup className="gap-5">
            <FieldSet data-invalid={labelError ? true : undefined}>
              <FieldLegend variant="label">Label</FieldLegend>
              <ToggleGroup type="single" variant="outline" value={label} onValueChange={(v) => v && setLabel(v as UpdateLabel)} className="flex-wrap">
                {labels.map((l) => (
                  <ToggleGroupItem key={l} value={l} className="px-3">
                    {labelTitle(l)}
                  </ToggleGroupItem>
                ))}
              </ToggleGroup>
              {labelError && <FieldError>{labelError}</FieldError>}
            </FieldSet>
            <Field data-invalid={messageError ? true : undefined}>
              <FieldLabel htmlFor="update-message" required>
                Message
              </FieldLabel>
              <Textarea
                id="update-message"
                rows={5}
                aria-required
                aria-invalid={messageError ? true : undefined}
                placeholder="What changed, and what happens next."
                value={message}
                onChange={(e) => setMessage(e.target.value)}
              />
              {messageError && <FieldError>{messageError}</FieldError>}
            </Field>
          </FieldGroup>
          <DialogFooter>
            <DialogClose asChild>
              <Button type="button" variant="ghost">
                Cancel
              </Button>
            </DialogClose>
            <Button type="submit" disabled={busy}>
              {busy && <Spinner data-icon="inline-start" />}
              Post update
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

/** One incident: its facts, its timeline of updates, and the dialog that posts the next one. */
export function Incident(): ReactElement {
  const { project: p, reload: reloadProject } = useProject();
  const { id = "" } = useParams();
  const base = `/projects/${p.slug}/incidents`;
  const path = `${base}/${encodeURIComponent(id)}`;
  const res = useApi<IncidentDetail>(path);
  if (res.error?.status === 404) return <NotFound />;
  const d = res.data;
  const i = d?.incident;
  const now = nowSecs();

  return (
    <>
      <Crumbs items={[{ label: "Incidents", to: base }, { label: i ? `INC-${i.id}` : "Incident" }]} />
      {!d || !i ? (
        <Skeleton className="h-96 rounded-xl" />
      ) : (
        <>
          <PageHeader
            title={i.title}
            meta={
              <>
                <StateBadge id="state" kind={i.resolved_at ? "up" : "down"} label={i.resolved_at ? "Resolved" : "Open"} />
                <SeverityBadge severity={i.severity} />
                <span className="font-mono text-sm">INC-{i.id}</span>
              </>
            }
            actions={
              <PostUpdate
                key={`${i.resolved_at ?? "open"}`}
                path={path}
                labels={d.labels}
                closed={i.resolved_at !== null}
                onPosted={() => {
                  res.reload();
                  reloadProject();
                }}
              />
            }
          />
          <div className="grid items-start gap-6 xl:grid-cols-[minmax(280px,1fr)_minmax(0,2fr)]">
            <Card>
              <CardHeader>
                <CardTitle className="text-[15px]">Facts</CardTitle>
              </CardHeader>
              <CardContent>
                <Facts
                  rows={[
                    ["Started", utc(i.started_at)],
                    ["Ended", i.resolved_at ? utc(i.resolved_at) : "Still open"],
                    ["Duration", duration((i.resolved_at ?? now) - i.started_at)],
                    ...(i.cause ? ([["Cause", <span className="font-mono text-[13px]">{i.cause}</span>]] as [string, ReactElement][]) : []),
                    ["Affects", d.items.join(", ")],
                    ["Origin", i.origin === "auto" ? "Detected by Monita" : "Reported by a tech lead"],
                  ]}
                />
              </CardContent>
            </Card>
            <TableCard title="Timeline">
              {d.updates.length === 0 && <TableEmpty title="No updates yet" description="Updates from you and from Monita appear here, oldest first." />}
              <ol className="flex flex-col">
                {d.updates.map((u, n) => (
                  <li key={n} className="flex flex-col gap-2 border-b px-5 py-4 last:border-0">
                    <div className="flex flex-wrap items-center gap-x-3 gap-y-1">
                      <LabelBadge label={u.label} />
                      <strong className="font-semibold">{u.author_email ?? "Monita"}</strong>
                      <span className="text-xs text-muted-foreground">
                        {day(u.created_at)} {hhmm(u.created_at)} UTC
                      </span>
                    </div>
                    <p className={u.author_email ? "whitespace-pre-wrap" : "text-muted-foreground"} data-update={u.author_email ? "tech-lead" : "monita"}>
                      {u.message}
                    </p>
                  </li>
                ))}
              </ol>
            </TableCard>
          </div>
        </>
      )}
    </>
  );
}
