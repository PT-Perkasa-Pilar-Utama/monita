import { useState } from "react";
import type { FormEvent, ReactElement } from "react";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Field, FieldGroup, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Spinner } from "@/components/ui/spinner";
import { BareLayout } from "@/layouts/BareLayout";
import { PeekingMoni, Playground } from "@/components/brand/Playground";
import { api, ApiError, errorText } from "@/lib/api";

/** Where to go after sign-in: `?next=` only when it stays on this site. The parsed path is
 * checked too: dot segments turn `/.//evil.example` into `//evil.example`, which the browser
 * reads as another host. */
export function nextPath(next: string | null, origin: string): string {
  if (!next?.startsWith("/")) return "/";
  try {
    const url = new URL(next, origin);
    const path = url.pathname + url.search + url.hash;
    return url.origin === origin && !path.startsWith("//") ? path : "/";
  } catch {
    return "/";
  }
}

const goOn = (): void => window.location.assign(nextPath(new URLSearchParams(window.location.search).get("next"), window.location.origin));

export function Login(): ReactElement {
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  // Set after a right password while two-factor is on: the page then asks for the code.
  const [challenge, setChallenge] = useState("");
  const [code, setCode] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);

  const submit = async (e: FormEvent): Promise<void> => {
    e.preventDefault();
    setBusy(true);
    setError("");
    try {
      if (challenge) {
        await api("/login/code", "POST", { challenge, code });
        goOn();
        return;
      }
      const step = await api<{ two_factor: true; challenge: string } | undefined>("/login", "POST", { email, password });
      if (!step) {
        goOn();
        return;
      }
      setChallenge(step.challenge);
      setBusy(false);
    } catch (err) {
      // 410: five wrong codes or five minutes. Start again from the password.
      if (err instanceof ApiError && err.status === 410) {
        setChallenge("");
        setCode("");
      }
      setError(errorText(err));
      setBusy(false);
    }
  };

  return (
    <BareLayout scene={<Playground />}>
      <Card className="relative mt-4">
        <PeekingMoni className="absolute -top-[44px] right-8 h-[46px] w-[100px]" />
        <CardHeader>
          <CardTitle>
            <h1 className="text-xl">{challenge ? "Enter your code" : "Sign in"}</h1>
          </CardTitle>
        </CardHeader>
        <CardContent>
          <form onSubmit={(e) => void submit(e)} className="flex flex-col gap-6">
            {error && (
              <Alert variant="destructive">
                <AlertDescription>{error}</AlertDescription>
              </Alert>
            )}
            {challenge ? (
              <Field>
                <FieldLabel htmlFor="code" required>
                  Code
                </FieldLabel>
                <Input
                  id="code"
                  autoFocus
                  autoComplete="one-time-code"
                  aria-required
                  className="font-mono"
                  placeholder="The 6 digits in your authenticator app, or a recovery code"
                  value={code}
                  onChange={(e) => setCode(e.target.value)}
                />
              </Field>
            ) : (
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
            )}
            <Button type="submit" disabled={busy} className="w-full">
              {busy && <Spinner data-icon="inline-start" />}
              {challenge ? "Verify" : "Sign in"}
            </Button>
          </form>
        </CardContent>
      </Card>
    </BareLayout>
  );
}
