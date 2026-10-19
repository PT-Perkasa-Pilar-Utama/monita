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
import { api, ApiError, errorText } from "@/lib/api";
import type { MyTicket, PublicSite } from "@/lib/types";

const asError = (e: unknown): ApiError => (e instanceof ApiError ? e : new ApiError(0, "error", errorText(e)));
const EMPTY = { title: "", description: "", page_url: "" };

/** Report a bug: a reporter enters their ref ID once, then files a bug and follows their own
 * tickets on the same page. The ref ID stays in memory only, never in the URL or storage. */
export function Report(): ReactElement {
  const project = useOutletContext<PublicSite>();
  const [draft, setDraft] = useState("");
  const [refId, setRefId] = useState<string | null>(null);
  const [tickets, setTickets] = useState<MyTicket[]>([]);
  const [form, setForm] = useState(EMPTY);
  const [error, setError] = useState<ApiError | null>(null);
  const [busy, setBusy] = useState(false);
  const [made, setMade] = useState<number | null>(null);
  const set = (k: keyof typeof form, v: string): void => setForm((f) => ({ ...f, [k]: v }));
  const err = (k: string): string | undefined => error?.fields[k];
  const load = (id: string): Promise<{ tickets: MyTicket[] }> => api(`/public/${project.slug}/mine`, "POST", { ref_id: id });

  const open = async (e: FormEvent): Promise<void> => {
    e.preventDefault();
    setBusy(true);
    setError(null);
    try {
      const id = draft.trim();
      setTickets((await load(id)).tickets);
      setRefId(id);
    } catch (e) {
      setError(asError(e));
    } finally {
      setBusy(false);
    }
  };

  const send = async (e: FormEvent): Promise<void> => {
    e.preventDefault();
    if (!refId) return;
    setBusy(true);
    setError(null);
    try {
      const res = await api<{ number: number }>(`/public/${project.slug}/report`, "POST", { ref_id: refId, ...form });
      setMade(res.number);
      setForm(EMPTY);
      setTickets((await load(refId)).tickets);
    } catch (e) {
      setError(asError(e));
    } finally {
      setBusy(false);
    }
  };

  const leave = (): void => {
    setRefId(null);
    setDraft("");
    setTickets([]);
    setMade(null);
    setError(null);
  };
  const formError = error && Object.keys(error.fields).length === 0 ? error.message : undefined;

  if (refId === null) {
    const message = error?.fields.ref_id ?? error?.message;
    return (
      <div className="mx-auto flex max-w-2xl flex-col">
        <PageHeader title="Report a bug" meta={<span>Your ref ID is on your account page in {project.name}. It lets you file bugs and follow them here.</span>} />
        <Card>
          <CardContent>
            <form onSubmit={(e) => void open(e)} noValidate className="flex flex-wrap items-end gap-3">
              <Field data-invalid={message ? true : undefined} className="min-w-0 flex-1 basis-64">
                <FieldLabel htmlFor="r-ref" required>
                  Your ref ID
                </FieldLabel>
                <Input id="r-ref" autoFocus autoComplete="off" spellCheck={false} className="font-mono" placeholder="rf_…" aria-required aria-invalid={message ? true : undefined} value={draft} onChange={(e) => setDraft(e.target.value)} />
                {message && <FieldError>{message}</FieldError>}
              </Field>
              <Button type="submit" disabled={busy || draft.trim() === ""} className={message ? "mb-7" : undefined}>
                {busy && <Spinner data-icon="inline-start" />}
                Continue
              </Button>
            </form>
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <>
      <PageHeader
        title="Report a bug"
        meta={
          <span className="flex flex-wrap items-center gap-x-3 gap-y-1">
            <span>
              Ref ID <code className="rounded-md bg-muted px-1.5 py-0.5 font-mono text-[13px]">rf_…{refId.slice(-4)}</code>
            </span>
            <button type="button" onClick={leave} className="text-link underline-offset-4 hover:underline">
              Use another ref ID
            </button>
          </span>
        }
      />
      <div className="grid items-start gap-6 lg:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle>New report</CardTitle>
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
              <span className="text-sm text-muted-foreground">Only the team reads your report. Please leave out passwords.</span>
              <Button type="submit" disabled={busy}>
                {busy && <Spinner data-icon="inline-start" />}
                Send report
              </Button>
            </CardFooter>
          </form>
        </Card>
        <MyTickets tickets={tickets} />
      </div>
    </>
  );
}
