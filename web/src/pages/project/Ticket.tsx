import { useEffect, useState } from "react";
import type { FormEvent, ReactElement } from "react";
import { useParams } from "react-router-dom";
import { ConfirmDialog } from "@/components/app/ConfirmDialog";
import { Facts } from "@/components/app/Facts";
import { Crumbs, PageHeader } from "@/components/layout/Page";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from "@/components/ui/card";
import { Field, FieldError, FieldGroup, FieldLabel, FieldLegend, FieldSet } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Separator } from "@/components/ui/separator";
import { Skeleton } from "@/components/ui/skeleton";
import { Spinner } from "@/components/ui/spinner";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { api, ApiError, errorText } from "@/lib/api";
import { useProject } from "@/lib/project";
import { REASONS, STAGES, stageLabel } from "@/lib/tickets";
import { utc } from "@/lib/time";
import type { CloseReason, Stage, TicketRow } from "@/lib/types";
import { useApi } from "@/lib/useApi";
import { NotFound } from "@/pages/NotFound";
import { toast } from "sonner";

const asError = (e: unknown): ApiError => (e instanceof ApiError ? e : new ApiError(0, "error", errorText(e)));

/** Where the ticket sits: its column, a close reason for Done, and the title the public sees. */
function BoardCard({ t, path, onSaved }: { t: TicketRow; path: string; onSaved: () => void }): ReactElement {
  const [stage, setStage] = useState<Stage>(t.stage);
  const [reason, setReason] = useState<CloseReason | "">(t.close_reason ?? "");
  const [title, setTitle] = useState(t.public_title ?? "");
  const [error, setError] = useState<ApiError | null>(null);
  const [busy, setBusy] = useState<"stage" | "title" | null>(null);
  useEffect(() => {
    setStage(t.stage);
    setReason(t.close_reason ?? "");
    setTitle(t.public_title ?? "");
  }, [t]);

  const save = async (e: FormEvent, what: "stage" | "title"): Promise<void> => {
    e.preventDefault();
    setBusy(what);
    setError(null);
    try {
      if (what === "stage") await api(`${path}/stage`, "PUT", { stage, close_reason: stage === "done" ? reason : null });
      else await api(`${path}/public-title`, "PUT", { public_title: title });
      toast.success(what === "stage" ? `Moved to ${stageLabel(stage)}.` : "Public title saved.");
      onSaved();
    } catch (err) {
      setError(asError(err));
    } finally {
      setBusy(null);
    }
  };
  const err = (k: string): string | undefined => error?.fields[k];
  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-[15px]">Board</CardTitle>
        <CardDescription>
          Outside Triage the public board shows it as <strong className="text-foreground">{t.public_title ?? t.title}</strong>.
        </CardDescription>
      </CardHeader>
      <form onSubmit={(e) => void save(e, "stage")} noValidate>
        <CardContent>
          <FieldGroup className="gap-5">
            <FieldSet data-invalid={err("stage") ? true : undefined}>
              <FieldLegend variant="label">Column</FieldLegend>
              <ToggleGroup type="single" variant="outline" value={stage} onValueChange={(v) => v && setStage(v as Stage)} className="flex-wrap">
                {STAGES.map(([k, l]) => (
                  <ToggleGroupItem key={k} value={k} className="px-3">
                    {l}
                  </ToggleGroupItem>
                ))}
              </ToggleGroup>
              {err("stage") && <FieldError>{err("stage")}</FieldError>}
            </FieldSet>
            {stage === "done" && (
              <FieldSet data-invalid={err("close_reason") ? true : undefined}>
                <FieldLegend variant="label">
                  Close reason <span aria-hidden="true" className="text-destructive">*</span>
                </FieldLegend>
                <ToggleGroup type="single" variant="outline" value={reason} onValueChange={(v) => setReason(v as CloseReason | "")} className="flex-wrap">
                  {REASONS.map(([k, l]) => (
                    <ToggleGroupItem key={k} value={k} className="px-3">
                      {l}
                    </ToggleGroupItem>
                  ))}
                </ToggleGroup>
                {err("close_reason") && <FieldError>{err("close_reason")}</FieldError>}
              </FieldSet>
            )}
          </FieldGroup>
        </CardContent>
        <CardFooter className="mt-5 justify-end">
          <Button type="submit" disabled={busy !== null}>
            {busy === "stage" && <Spinner data-icon="inline-start" />}
            Move ticket
          </Button>
        </CardFooter>
      </form>
      <Separator />
      <form onSubmit={(e) => void save(e, "title")} noValidate>
        <CardContent>
          <Field data-invalid={err("public_title") ? true : undefined}>
            <FieldLabel htmlFor="public-title">Public title, when the reporter's title says too much</FieldLabel>
            <Input id="public-title" maxLength={120} aria-invalid={err("public_title") ? true : undefined} value={title} onChange={(e) => setTitle(e.target.value)} />
            {err("public_title") && <FieldError>{err("public_title")}</FieldError>}
          </Field>
        </CardContent>
        <CardFooter className="mt-5 justify-end">
          <Button type="submit" variant="secondary" disabled={busy !== null}>
            {busy === "title" && <Spinner data-icon="inline-start" />}
            Save public title
          </Button>
        </CardFooter>
      </form>
    </Card>
  );
}

/** One ticket: the private report, and the board controls. */
export function Ticket(): ReactElement {
  const { project: p, reload: reloadProject } = useProject();
  const { number = "" } = useParams();
  const base = `/projects/${p.slug}/tickets`;
  const path = `${base}/${encodeURIComponent(number)}`;
  const res = useApi<TicketRow>(path);
  if (res.error?.status === 404) return <NotFound />;
  const t = res.data;
  const saved = (): void => {
    res.reload();
    reloadProject();
  };
  return (
    <>
      <Crumbs items={[{ label: "Tickets", to: base }, { label: t ? `#${t.number}` : "Ticket" }]} />
      {!t ? (
        <Skeleton className="h-96 rounded-xl" />
      ) : (
        <>
          <PageHeader
            title={t.title}
            meta={
              <>
                <Badge variant="neutral">{stageLabel(t.stage)}</Badge>
                <span className="font-mono text-sm">#{t.number}</span>
              </>
            }
          />
          <div className="grid items-start gap-6 lg:grid-cols-2">
            <Card>
              <CardHeader>
                <CardTitle className="text-[15px]">Report</CardTitle>
                <CardDescription>Only your team sees the report and its reporter.</CardDescription>
              </CardHeader>
              <CardContent className="flex flex-col gap-5">
                <p className="whitespace-pre-wrap break-words">{t.description}</p>
                <Facts
                  rows={[
                    ...(t.page_url ? ([["Page", <span className="font-mono text-[13px] break-all">{t.page_url}</span>]] as [string, ReactElement][]) : []),
                    ["Reported", utc(t.created_at)],
                    ["Updated", utc(t.updated_at)],
                    ["Ref ID", t.ref_revoked ? "Revoked" : "Active"],
                  ]}
                />
              </CardContent>
              {!t.ref_revoked && (
                <CardFooter>
                  <ConfirmDialog
                    trigger={
                      <Button variant="danger" size="sm">
                        Revoke this reporter's ref ID
                      </Button>
                    }
                    title="Revoke this ref ID?"
                    description="The reporter can no longer file tickets with it. You cannot undo this."
                    action="Revoke"
                    onConfirm={async () => {
                      await api(`${path}/revoke-ref`, "POST");
                      res.reload();
                    }}
                  />
                </CardFooter>
              )}
            </Card>
            <BoardCard t={t} path={path} onSaved={saved} />
          </div>
        </>
      )}
    </>
  );
}
