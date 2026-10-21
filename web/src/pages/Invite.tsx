import { useState } from "react";
import type { FormEvent, ReactElement } from "react";
import { useParams } from "react-router-dom";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Field, FieldError, FieldGroup, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Spinner } from "@/components/ui/spinner";
import { BareLayout } from "@/layouts/BareLayout";
import { api, ApiError, dropField, errorText } from "@/lib/api";
import { useApi } from "@/lib/useApi";

/** An invite link: set a password, then land on the projects. */
export function Invite(): ReactElement {
  const { token = "" } = useParams();
  const link = useApi<{ usable: boolean }>(`/invites/${encodeURIComponent(token)}`);
  const [password, setPassword] = useState("");
  const [confirm, setConfirm] = useState("");
  const [error, setError] = useState<ApiError | null>(null);
  const [busy, setBusy] = useState(false);
  const clear = (field: string): void => setError((e) => e && dropField(e, field));
  const dead = link.error?.status === 410 || error?.status === 410;

  const submit = async (e: FormEvent): Promise<void> => {
    e.preventDefault();
    setBusy(true);
    setError(null);
    try {
      await api(`/invites/${encodeURIComponent(token)}`, "POST", { password, password_confirm: confirm });
      window.location.assign("/");
    } catch (err) {
      setError(err instanceof ApiError ? err : new ApiError(0, "error", errorText(err)));
      setBusy(false);
    }
  };

  if (dead) {
    return (
      <BareLayout>
        <Card>
          <CardHeader>
            <CardTitle>
            <h1 className="text-xl">Link not valid</h1>
          </CardTitle>
            <CardDescription>This link is invalid, expired or already used. Ask a super admin for a new invite.</CardDescription>
          </CardHeader>
        </Card>
      </BareLayout>
    );
  }
  return (
    <BareLayout>
      <Card>
        <CardHeader>
          <CardTitle>
            <h1 className="text-xl">Set your password</h1>
          </CardTitle>
          <CardDescription>Use at least 8 characters. You sign in with your email and this password.</CardDescription>
        </CardHeader>
        <CardContent>
          <form onSubmit={(e) => void submit(e)} className="flex flex-col gap-6">
            {error && !error.fields.password && !error.fields.password_confirm && (
              <Alert variant="destructive">
                <AlertDescription>{error.message}</AlertDescription>
              </Alert>
            )}
            <FieldGroup className="gap-5">
              <Field data-invalid={error?.fields.password ? true : undefined}>
                <FieldLabel htmlFor="password" required>
                  New password
                </FieldLabel>
                <Input
                  id="password"
                  type="password"
                  autoComplete="new-password"
                  aria-required
                  aria-invalid={error?.fields.password ? true : undefined}
                  value={password}
                  onChange={(e) => {
                    setPassword(e.target.value);
                    clear("password");
                  }}
                />
                {error?.fields.password && <FieldError>{error.fields.password}</FieldError>}
              </Field>
              <Field data-invalid={error?.fields.password_confirm ? true : undefined}>
                <FieldLabel htmlFor="password_confirm" required>
                  Repeat password
                </FieldLabel>
                <Input
                  id="password_confirm"
                  type="password"
                  autoComplete="new-password"
                  aria-required
                  aria-invalid={error?.fields.password_confirm ? true : undefined}
                  value={confirm}
                  onChange={(e) => {
                    setConfirm(e.target.value);
                    clear("password_confirm");
                  }}
                />
                {error?.fields.password_confirm && <FieldError>{error.fields.password_confirm}</FieldError>}
              </Field>
            </FieldGroup>
            <Button type="submit" disabled={busy || link.loading} className="w-full">
              {busy && <Spinner data-icon="inline-start" />}
              Save password
            </Button>
          </form>
        </CardContent>
      </Card>
    </BareLayout>
  );
}
