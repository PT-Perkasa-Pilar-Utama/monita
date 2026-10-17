import { useState } from "react";
import type { FormEvent, ReactElement } from "react";
import { Link, useNavigate } from "react-router-dom";
import { FormAside, SeverityField } from "@/components/app/CheckForm";
import { Crumbs, PageHeader } from "@/components/layout/Page";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardFooter } from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
import { Field, FieldError, FieldGroup, FieldLabel, FieldLegend, FieldSet } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { Spinner } from "@/components/ui/spinner";
import { Textarea } from "@/components/ui/textarea";
import { api, ApiError, errorText } from "@/lib/api";
import { useProject } from "@/lib/project";
import type { IncidentItem, Severity } from "@/lib/types";
import { useApi } from "@/lib/useApi";

const STEPS = [
  "Report a problem that no check catches, such as a slow sign-in for some users.",
  "The items you pick show as Issue on the status page, not as down. Uptime does not change.",
  "The first update appears on the status page at once, without your name.",
  "A reported incident stays open until you post an update labelled Resolved.",
];

/** Report an incident by hand: a title, the items it affects, a severity and the first update. */
export function IncidentForm(): ReactElement {
  const { project: p, reload } = useProject();
  const navigate = useNavigate();
  const base = `/projects/${p.slug}/incidents`;
  const choices = useApi<IncidentItem[]>(`/projects/${p.slug}/incident-items`);
  const [title, setTitle] = useState("");
  const [items, setItems] = useState<string[]>([]);
  const [severity, setSeverity] = useState<Severity>("warning");
  const [message, setMessage] = useState("");
  const [error, setError] = useState<ApiError | null>(null);
  const [busy, setBusy] = useState(false);
  const err = (k: string): string | undefined => error?.fields[k];

  const submit = async (e: FormEvent): Promise<void> => {
    e.preventDefault();
    setBusy(true);
    setError(null);
    try {
      const made = await api<{ id: number }>(base, "POST", { title, items, severity, message });
      reload();
      navigate(`${base}/${made.id}`);
    } catch (err) {
      setError(err instanceof ApiError ? err : new ApiError(0, "error", errorText(err)));
    } finally {
      setBusy(false);
    }
  };
  const toggle = (value: string, on: boolean): void => setItems((xs) => (on ? [...xs, value] : xs.filter((x) => x !== value)));
  const formError = error && Object.keys(error.fields).length === 0 ? error.message : undefined;

  return (
    <>
      <Crumbs items={[{ label: "Incidents", to: base }, { label: "Report" }]} />
      <PageHeader title="Report an incident" />
      <div className="grid items-start gap-6 xl:grid-cols-[minmax(0,2fr)_minmax(280px,1fr)]">
        {!choices.data ? (
          <Skeleton className="h-[480px] rounded-xl" />
        ) : (
          <Card>
            <form onSubmit={(e) => void submit(e)} noValidate className="flex flex-col gap-6">
              <CardContent>
                <FieldGroup className="gap-5">
                  {formError && (
                    <p role="alert" className="text-sm text-destructive">
                      {formError}
                    </p>
                  )}
                  <Field data-invalid={err("title") ? true : undefined}>
                    <FieldLabel htmlFor="i-title" required>
                      Title
                    </FieldLabel>
                    <Input id="i-title" autoFocus maxLength={120} aria-required aria-invalid={err("title") ? true : undefined} value={title} onChange={(e) => setTitle(e.target.value)} />
                    {err("title") && <FieldError>{err("title")}</FieldError>}
                  </Field>
                  <FieldSet data-invalid={err("items") ? true : undefined}>
                    <FieldLegend variant="label">
                      Affected monitors and heartbeats <span aria-hidden="true" className="text-destructive">*</span>
                    </FieldLegend>
                    {choices.data.length === 0 ? (
                      <p className="text-sm text-muted-foreground">Add a monitor or heartbeat first.</p>
                    ) : (
                      <div className="grid gap-3 sm:grid-cols-2">
                        {choices.data.map((c, n) => (
                          <Field key={c.value} orientation="horizontal">
                            <Checkbox id={`item-${n}`} checked={items.includes(c.value)} onCheckedChange={(v) => toggle(c.value, v === true)} />
                            <FieldLabel htmlFor={`item-${n}`} className="font-normal">
                              {c.name}
                            </FieldLabel>
                          </Field>
                        ))}
                      </div>
                    )}
                    {err("items") && <FieldError>{err("items")}</FieldError>}
                  </FieldSet>
                  <SeverityField value={severity} onChange={setSeverity} />
                  <Field data-invalid={err("message") ? true : undefined}>
                    <FieldLabel htmlFor="i-message" required>
                      First update
                    </FieldLabel>
                    <Textarea
                      id="i-message"
                      rows={5}
                      aria-required
                      aria-invalid={err("message") ? true : undefined}
                      placeholder="What users see, and what you know so far."
                      value={message}
                      onChange={(e) => setMessage(e.target.value)}
                    />
                    {err("message") && <FieldError>{err("message")}</FieldError>}
                  </Field>
                </FieldGroup>
              </CardContent>
              <CardFooter className="justify-end gap-2 border-t">
                <Button asChild variant="ghost">
                  <Link to={base}>Cancel</Link>
                </Button>
                <Button type="submit" disabled={busy}>
                  {busy && <Spinner data-icon="inline-start" />}
                  Open incident
                </Button>
              </CardFooter>
            </form>
          </Card>
        )}
        <FormAside title="When to report an incident" steps={STEPS} note="A reported incident never counts as downtime." />
      </div>
    </>
  );
}
