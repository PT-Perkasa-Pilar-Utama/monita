import { useState } from "react";
import type { FormEvent, ReactElement } from "react";
import { useParams } from "react-router-dom";
import { ChoiceCards } from "@/components/app/ChoiceCards";
import { FormSheet, FormSheetBody } from "@/components/app/FormSheet";
import { Side } from "@/components/app/Side";
import { LabelBadge, SeverityBadge, StateBadge } from "@/components/app/Tone";
import { Crumbs, PageHeader } from "@/components/layout/Page";
import { TableEmpty } from "@/components/list/TableEmpty";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Field, FieldError, FieldGroup, FieldLabel } from "@/components/ui/field";
import { Skeleton } from "@/components/ui/skeleton";
import { Spinner } from "@/components/ui/spinner";
import { Textarea } from "@/components/ui/textarea";
import { api, ApiError, dropField, errorText } from "@/lib/api";
import { useProject } from "@/lib/project";
import { ago, duration, nowSecs, stamp } from "@/lib/time";
import type { IncidentDetail, UpdateLabel } from "@/lib/types";
import { nextMessage, TEMPLATES } from "@/lib/updates";
import { useApi } from "@/lib/useApi";
import { cn } from "@/lib/utils";
import { NotFound } from "@/pages/NotFound";
import { toast } from "sonner";

const LABELS: { value: UpdateLabel; title: string; description: string }[] = [
  { value: "investigating", title: "Investigating", description: "Something is wrong and you are looking into it." },
  { value: "identified", title: "Identified", description: "You found the cause and are working on a fix." },
  { value: "monitoring", title: "Monitoring", description: "A fix is out. You are watching to be sure it holds." },
  { value: "resolved", title: "Resolved", description: "It is over. A reported incident closes with this update." },
];

/** Why a label is off for this incident, in the words the server's rule uses. */
function unavailable(label: UpdateLabel, allowed: UpdateLabel[]): string | undefined {
  return allowed.includes(label) ? undefined : "The incident is over. Only a Resolved note can follow it.";
}

/** Resolved on an incident Monita opened closes it now; a check that still fails opens a new one. */
const RESOLVE_AUTO = "Closes it now and tells the alert recipients. If it still fails, Monita opens a new incident.";

/** Post update, in the half-width panel: a label as cards, and the message the status page shows. */
function PostUpdate({ path, labels, auto, onPosted }: { path: string; labels: UpdateLabel[]; auto: boolean; onPosted: () => void }): ReactElement {
  const [open, setOpen] = useState(false);
  const first = labels.includes("monitoring") ? "monitoring" : (labels[0] ?? "resolved");
  const [label, setLabel] = useState<UpdateLabel>(first);
  const [message, setMessage] = useState(TEMPLATES[first]);
  const [error, setError] = useState<ApiError | null>(null);
  const show = (): void => {
    setLabel(first);
    setMessage(TEMPLATES[first]);
    setError(null);
    setOpen(true);
  };
  const pick = (l: UpdateLabel): void => {
    setLabel(l);
    setMessage((m) => nextMessage(m, l));
    setError((e) => e && dropField(e, "message"));
  };
  const [busy, setBusy] = useState(false);
  const submit = async (_e: FormEvent): Promise<void> => {
    setBusy(true);
    setError(null);
    try {
      await api(`${path}/updates`, "POST", { label, message });
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
    <>
      <Button onClick={show}>Post update</Button>
      <FormSheet open={open} onClose={() => setOpen(false)}>
        <FormSheetBody
          title="Post an update"
          description="The update appears on the status page at once, without your name."
          onSubmit={(e) => void submit(e)}
          footer={
            <div className="ml-auto flex gap-2">
              <Button type="button" variant="ghost" onClick={() => setOpen(false)}>
                Cancel
              </Button>
              <Button type="submit" disabled={busy}>
                {busy && <Spinner data-icon="inline-start" />}
                Post update
              </Button>
            </div>
          }
        >
          <FieldGroup className="gap-5">
            <ChoiceCards id="label" legend="Label" value={label} onChange={pick} choices={LABELS.map((l) => ({ ...l, description: auto && l.value === "resolved" ? RESOLVE_AUTO : l.description, unavailable: unavailable(l.value, labels) }))} />
            {labelError && <FieldError>{labelError}</FieldError>}
            <Field data-invalid={messageError ? true : undefined}>
              <FieldLabel htmlFor="update-message" required>
                Message
              </FieldLabel>
              <Textarea
                id="update-message"
                aria-required
                aria-invalid={messageError ? true : undefined}
                placeholder="What changed, and what happens next."
                className="min-h-40"
                value={message}
                onChange={(e) => {
                  setMessage(e.target.value);
                  setError((er) => er && dropField(er, "message"));
                }}
              />
              {messageError && <FieldError>{messageError}</FieldError>}
            </Field>
          </FieldGroup>
        </FormSheetBody>
      </FormSheet>
    </>
  );
}

/** One incident: its updates as a thread on the left, its facts in a sidebar on the right. */
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
                <span className="text-sm text-muted-foreground">
                  INC-{i.id}, {i.origin === "auto" ? "detected by Monita" : "reported by hand"} {ago(i.started_at)}
                </span>
              </>
            }
            actions={
              <PostUpdate
                key={`${i.resolved_at ?? "open"}`}
                path={path}
                labels={d.labels}
                auto={i.origin === "auto" && i.resolved_at === null}
                onPosted={() => {
                  res.reload();
                  reloadProject();
                }}
              />
            }
          />
          <div className="grid items-start gap-8 lg:grid-cols-[minmax(0,1fr)_18rem]">
            <section aria-labelledby="timeline" className="flex min-w-0 flex-col gap-4">
              <h2 id="timeline" className="text-[15px] font-semibold">
                Updates, newest first
              </h2>
              {d.updates.length === 0 ? (
                <Card className="py-0">
                  <TableEmpty title="No updates yet" description="Updates from you and from Monita appear here." />
                </Card>
              ) : (
                <ol className="relative flex flex-col gap-4 before:absolute before:top-4 before:bottom-4 before:left-4 before:w-px before:bg-border">
                  {d.updates.map((u, n) => (
                    <li key={n} className="relative flex gap-4">
                      <span
                        aria-hidden="true"
                        className={cn(
                          "z-10 mt-2 flex size-8 shrink-0 items-center justify-center rounded-full border bg-background text-xs font-semibold text-muted-foreground",
                          !u.author_email && "bg-muted",
                        )}
                      >
                        {(u.author_email ?? "M")[0]?.toUpperCase()}
                      </span>
                      <article className="min-w-0 flex-1 overflow-hidden rounded-lg border bg-card">
                        <header className="flex flex-wrap items-center gap-x-2 gap-y-1 border-b bg-paper px-4 py-2.5 text-[13px]">
                          <LabelBadge label={u.label} />
                          <strong className="font-semibold">{u.author_email ?? "Monita"}</strong>
                          <span className="text-muted-foreground" title={stamp(u.created_at)}>
                            {ago(u.created_at)}
                          </span>
                        </header>
                        <p className={cn("px-4 py-3 text-sm whitespace-pre-wrap break-words", !u.author_email && "text-muted-foreground")} data-update={u.author_email ? "tech-lead" : "monita"}>
                          {u.message}
                        </p>
                      </article>
                    </li>
                  ))}
                </ol>
              )}
            </section>
            <aside className="flex flex-col gap-5 lg:sticky lg:top-[calc(var(--bar-h)+24px)]">
              <Side title="Affects">
                <ul className="flex flex-col gap-1 text-sm">
                  {d.items.map((it) => (
                    <li key={it}>{it}</li>
                  ))}
                </ul>
              </Side>
              {i.cause && (
                <Side title="Cause">
                  <p className="font-mono text-[13px] break-words">{i.cause}</p>
                </Side>
              )}
              <Side title="Dates">
                <dl className="grid grid-cols-[max-content_minmax(0,1fr)] gap-x-3 gap-y-1.5 text-sm">
                  <dt className="text-muted-foreground">Started</dt>
                  <dd>{stamp(i.started_at)}</dd>
                  <dt className="text-muted-foreground">Ended</dt>
                  <dd>{i.resolved_at ? stamp(i.resolved_at) : "Still open"}</dd>
                  <dt className="text-muted-foreground">Lasted</dt>
                  <dd>{duration((i.resolved_at ?? now) - i.started_at)}</dd>
                </dl>
              </Side>
              <Side title="How it closes">
                <p className="text-sm text-muted-foreground">
                  {i.origin === "auto"
                    ? "Monita closes it on the next passing check or heartbeat ping. You can also resolve it yourself, such as when you retire the job."
                    : "It stays open until you post an update labelled Resolved."}
                </p>
              </Side>
            </aside>
          </div>
        </>
      )}
    </>
  );
}
