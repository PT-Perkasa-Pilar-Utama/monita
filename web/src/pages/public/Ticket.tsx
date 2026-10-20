import { useState } from "react";
import type { FormEvent, ReactElement } from "react";
import { useOutletContext } from "react-router-dom";
import { MyTickets } from "@/components/app/MyTickets";
import { CheckCircle } from "@/components/icons";
import { PageHeader } from "@/components/layout/Page";
import { Alert, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardFooter, CardHeader, CardTitle } from "@/components/ui/card";
import { Field, FieldError, FieldGroup, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Spinner } from "@/components/ui/spinner";
import { Textarea } from "@/components/ui/textarea";
import { api, ApiError, dropField, errorText } from "@/lib/api";
import type { MyTicket, PublicSite } from "@/lib/types";

const asError = (e: unknown): ApiError => (e instanceof ApiError ? e : new ApiError(0, "error", errorText(e)));
const EMPTY = { ref_id: "", title: "", description: "", page_url: "" };

/** A ref ID input. The value is a credential, so the browser neither fills nor checks it. */
function RefIdInput({ id, value, onChange, invalid }: { id: string; value: string; onChange: (v: string) => void; invalid: boolean }): ReactElement {
  return <Input id={id} autoComplete="off" spellCheck={false} className="font-mono" placeholder="rf_…" aria-required aria-invalid={invalid ? true : undefined} value={value} onChange={(e) => onChange(e.target.value)} />;
}

/** The public Ticket tab: report a bug on the left, check your tickets on the right. Both take
 * the reporter's ref ID. A sent report shows the reporter's tickets on the right at once. */
export function Ticket(): ReactElement {
  const project = useOutletContext<PublicSite>();
  const [form, setForm] = useState(EMPTY);
  const [sendError, setSendError] = useState<ApiError | null>(null);
  const [sending, setSending] = useState(false);
  const [made, setMade] = useState<number | null>(null);
  const [lookup, setLookup] = useState("");
  const [tickets, setTickets] = useState<MyTicket[] | null>(null);
  const [lookError, setLookError] = useState<ApiError | null>(null);
  const [looking, setLooking] = useState(false);
  const set = (k: keyof typeof form, v: string): void => {
    setForm((f) => ({ ...f, [k]: v }));
    setSendError((e) => e && dropField(e, k));
  };
  const err = (k: string): string | undefined => sendError?.fields[k];

  const show = async (refId: string): Promise<void> => {
    setLooking(true);
    setLookError(null);
    try {
      setTickets((await api<{ tickets: MyTicket[] }>(`/public/${project.slug}/mine`, "POST", { ref_id: refId.trim() })).tickets);
    } catch (e) {
      setTickets(null);
      setLookError(asError(e));
    } finally {
      setLooking(false);
    }
  };

  const send = async (e: FormEvent): Promise<void> => {
    e.preventDefault();
    setSending(true);
    setSendError(null);
    setMade(null);
    try {
      const res = await api<{ number: number }>(`/public/${project.slug}/report`, "POST", form);
      setMade(res.number);
      setLookup(form.ref_id);
      setForm({ ...EMPTY, ref_id: form.ref_id });
      void show(form.ref_id);
    } catch (e) {
      setSendError(asError(e));
    } finally {
      setSending(false);
    }
  };

  const look = (e: FormEvent): void => {
    e.preventDefault();
    void show(lookup);
  };
  const formError = sendError && Object.keys(sendError.fields).length === 0 ? sendError.message : undefined;
  const lookMessage = lookError?.fields.ref_id ?? lookError?.message;

  return (
    <>
      <PageHeader title={`${project.name} tickets`} />
      <div className="grid items-start gap-6 lg:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle>Report a bug</CardTitle>
          </CardHeader>
          <form onSubmit={(e) => void send(e)} noValidate className="flex flex-col gap-6">
            <CardContent>
              <FieldGroup className="gap-5">
                {made !== null && (
                  <Alert role="status">
                    <CheckCircle />
                    <AlertTitle>Thanks. Your report is ticket #{made}.</AlertTitle>
                  </Alert>
                )}
                {formError && (
                  <p role="alert" className="text-sm text-destructive">
                    {formError}
                  </p>
                )}
                <Field data-invalid={err("ref_id") ? true : undefined}>
                  <FieldLabel htmlFor="r-ref" required>
                    Your ref ID
                  </FieldLabel>
                  <RefIdInput id="r-ref" value={form.ref_id} onChange={(v) => set("ref_id", v)} invalid={!!err("ref_id")} />
                  {err("ref_id") && <FieldError>{err("ref_id")}</FieldError>}
                </Field>
                <Field data-invalid={err("title") ? true : undefined}>
                  <FieldLabel htmlFor="r-title" required>
                    What went wrong
                  </FieldLabel>
                  <Input id="r-title" maxLength={120} aria-required aria-invalid={err("title") ? true : undefined} value={form.title} onChange={(e) => set("title", e.target.value)} placeholder="The Export button stays grey" />
                  {err("title") && <FieldError>{err("title")}</FieldError>}
                </Field>
                <Field data-invalid={err("description") ? true : undefined}>
                  <FieldLabel htmlFor="r-description" required>
                    What happened
                  </FieldLabel>
                  <Textarea id="r-description" rows={6} aria-required aria-invalid={err("description") ? true : undefined} placeholder="What you did, what you expected, what happened." value={form.description} onChange={(e) => set("description", e.target.value)} />
                  {err("description") && <FieldError>{err("description")}</FieldError>}
                </Field>
                <Field data-invalid={err("page_url") ? true : undefined}>
                  <FieldLabel htmlFor="r-url">Page where it happened</FieldLabel>
                  <Input id="r-url" type="url" className="font-mono" aria-invalid={err("page_url") ? true : undefined} value={form.page_url} onChange={(e) => set("page_url", e.target.value)} placeholder="https://" />
                  {err("page_url") && <FieldError>{err("page_url")}</FieldError>}
                </Field>
              </FieldGroup>
            </CardContent>
            <CardFooter className="flex-wrap justify-between gap-3 border-t">
              <span className="text-sm text-muted-foreground">Do not include sensitive information.</span>
              <Button type="submit" disabled={sending}>
                {sending && <Spinner data-icon="inline-start" />}
                Send report
              </Button>
            </CardFooter>
          </form>
        </Card>
        <div className="flex flex-col gap-6">
          <Card>
            <CardHeader>
              <CardTitle>Check your tickets</CardTitle>
            </CardHeader>
            <CardContent>
              <form onSubmit={look} noValidate className="flex flex-wrap items-end gap-3">
                <Field data-invalid={lookMessage ? true : undefined} className="min-w-0 flex-1 basis-56">
                  <FieldLabel htmlFor="m-ref" required>
                    Your ref ID
                  </FieldLabel>
                  <RefIdInput id="m-ref" value={lookup} onChange={(v) => {
                    setLookup(v);
                    setLookError((e) => e && dropField(e, "ref_id"));
                  }} invalid={!!lookMessage} />
                  {lookMessage && <FieldError>{lookMessage}</FieldError>}
                </Field>
                <Button type="submit" variant="secondary" disabled={looking || lookup.trim() === ""} className={lookMessage ? "mb-7" : undefined}>
                  {looking && <Spinner data-icon="inline-start" />}
                  Show my tickets
                </Button>
              </form>
            </CardContent>
          </Card>
          {tickets !== null && <MyTickets tickets={tickets} />}
        </div>
      </div>
    </>
  );
}
