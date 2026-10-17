import { useState } from "react";
import type { FormEvent, ReactElement } from "react";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Field, FieldGroup, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Spinner } from "@/components/ui/spinner";
import { BareLayout } from "@/layouts/BareLayout";
import { api, errorText } from "@/lib/api";

/** Where to go after signing in: a path on this site only. */
function nextPath(): string {
  const next = new URLSearchParams(window.location.search).get("next") ?? "/";
  return next.startsWith("/") && !next.startsWith("//") ? next : "/";
}

export function Login(): ReactElement {
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);

  const submit = async (e: FormEvent): Promise<void> => {
    e.preventDefault();
    setBusy(true);
    setError("");
    try {
      await api("/login", "POST", { email, password });
      window.location.assign(nextPath());
    } catch (err) {
      setError(errorText(err));
      setBusy(false);
    }
  };

  return (
    <BareLayout>
      <Card>
        <CardHeader>
          <CardTitle>
            <h1 className="text-xl">Sign in</h1>
          </CardTitle>
        </CardHeader>
        <CardContent>
          <form onSubmit={(e) => void submit(e)} className="flex flex-col gap-6">
            {error && (
              <Alert variant="destructive">
                <AlertDescription>{error}</AlertDescription>
              </Alert>
            )}
            <FieldGroup className="gap-5">
              <Field>
                <FieldLabel htmlFor="email" required>
                  Email
                </FieldLabel>
                <Input id="email" type="email" autoComplete="username" aria-required value={email} onChange={(e) => setEmail(e.target.value)} />
              </Field>
              <Field>
                <FieldLabel htmlFor="password" required>
                  Password
                </FieldLabel>
                <Input
                  id="password"
                  type="password"
                  autoComplete="current-password"
                  aria-required
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                />
              </Field>
            </FieldGroup>
            <Button type="submit" disabled={busy} className="w-full">
              {busy && <Spinner data-icon="inline-start" />}
              Sign in
            </Button>
          </form>
        </CardContent>
      </Card>
    </BareLayout>
  );
}
