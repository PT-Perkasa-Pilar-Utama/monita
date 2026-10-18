import { useState } from "react";
import type { FormEvent, ReactElement } from "react";
import { Link, useOutletContext } from "react-router-dom";
import { CheckCircle } from "@/components/icons";
import { PageHeader } from "@/components/layout/Page";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardFooter } from "@/components/ui/card";
import { Field, FieldError, FieldGroup, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Spinner } from "@/components/ui/spinner";
import { Textarea } from "@/components/ui/textarea";
import { api, ApiError, errorText } from "@/lib/api";
import type { PublicSite } from "@/lib/types";

/** The reporter's bug form. A ref ID from the project's own system proves who reports. */
export function Report(): ReactElement {
  const project = useOutletContext<PublicSite>();
  const [form, setForm] = useState({ ref_id: "", title: "", description: "", page_url: "" });
  const [error, setError] = useState<ApiError | null>(null);
  const [busy, setBusy] = useState(false);
  const [made, setMade] = useState<number | null>(null);
  const set = (k: keyof typeof form, v: string): void => setForm((f) => ({ ...f, [k]: v }));
  const err = (k: string): string | undefined => error?.fields[k];

  const submit = async (e: FormEvent): Promise<void> => {
    e.preventDefault();
    setBusy(true);
    setError(null);
    try {
      const res = await api<{ number: number }>(`/public/${project.slug}/report`, "POST", form);
      setMade(res.number);
    } catch (err) {
      setError(err instanceof ApiError ? err : new ApiError(0, "error", errorText(err)));
    } finally {
      setBusy(false);
    }
  };
  const formError = error && Object.keys(error.fields).length === 0 ? error.message : undefined;

  return (
    <div className="mx-auto flex max-w-2xl flex-col">
      <PageHeader
        title="Report a bug"
        meta={
          <span>
            Tell the {project.name} team what broke. Your ref ID is on your account page in {project.name}.
          </span>
        }
      />
      {made !== null ? (
        <Alert role="status">
          <CheckCircle />
          <AlertTitle>Thanks. Your report is ticket #{made}.</AlertTitle>
          {project.pages.mine && (
            <AlertDescription>
              <span>
                Track it with your ref ID on the{" "}
                <Link to={`/status/${project.slug}/mine`} className="text-link underline-offset-4 hover:underline">
                  My tickets
                </Link>{" "}
                page.
              </span>
            </AlertDescription>
          )}
        </Alert>
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
                <Field data-invalid={err("ref_id") ? true : undefined}>
                  <FieldLabel htmlFor="r-ref" required>
                    Your ref ID
                  </FieldLabel>
                  <Input id="r-ref" autoComplete="off" spellCheck={false} className="font-mono" aria-required aria-invalid={err("ref_id") ? true : undefined} value={form.ref_id} onChange={(e) => set("ref_id", e.target.value)} />
                  {err("ref_id") && <FieldError>{err("ref_id")}</FieldError>}
                </Field>
                <Field data-invalid={err("title") ? true : undefined}>
                  <FieldLabel htmlFor="r-title" required>
                    What went wrong
                  </FieldLabel>
                  <Input id="r-title" maxLength={120} aria-required aria-invalid={err("title") ? true : undefined} value={form.title} onChange={(e) => set("title", e.target.value)} />
                  {err("title") && <FieldError>{err("title")}</FieldError>}
                </Field>
                <Field data-invalid={err("description") ? true : undefined}>
                  <FieldLabel htmlFor="r-description" required>
                    What happened
                  </FieldLabel>
                  <Textarea
                    id="r-description"
                    rows={6}
                    aria-required
                    aria-invalid={err("description") ? true : undefined}
                    placeholder="What you did, what you expected, what happened."
                    value={form.description}
                    onChange={(e) => set("description", e.target.value)}
                  />
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
      )}
    </div>
  );
}
