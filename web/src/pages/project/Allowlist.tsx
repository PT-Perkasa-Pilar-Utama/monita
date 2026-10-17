import { useState } from "react";
import type { FormEvent, ReactElement } from "react";
import { PageHeader } from "@/components/layout/Page";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardAction, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Field, FieldError, FieldLabel } from "@/components/ui/field";
import { Spinner } from "@/components/ui/spinner";
import { Textarea } from "@/components/ui/textarea";
import { api, ApiError, errorText } from "@/lib/api";
import { useProject } from "@/lib/project";
import { toast } from "sonner";

/** Settings, Allowlist: the addresses that may use this project's API keys. */
export function Allowlist(): ReactElement {
  const { project: p, reload } = useProject();
  const [cidrs, setCidrs] = useState((p.ip_allowlist ?? "").replaceAll(",", "\n"));
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const save = async (e: FormEvent): Promise<void> => {
    e.preventDefault();
    setBusy(true);
    setError("");
    try {
      await api(`/projects/${p.slug}/allowlist`, "PUT", { cidrs });
      reload();
      toast.success(cidrs.trim() ? "Allowlist saved." : "Allowlist turned off.");
    } catch (err) {
      setError(err instanceof ApiError ? (err.fields.cidrs ?? err.message) : errorText(err));
    } finally {
      setBusy(false);
    }
  };
  return (
    <>
      <PageHeader title="Allowlist" />
      <Card className="max-w-3xl">
        <CardHeader className="border-b">
          <CardTitle>Allowed IP addresses</CardTitle>
          <CardDescription>Only these addresses may use this project's API keys. Leave the list empty to allow any address.</CardDescription>
          <CardAction>
            <Badge variant={p.ip_allowlist ? "up" : "unknown"}>{p.ip_allowlist ? "On" : "Off"}</Badge>
          </CardAction>
        </CardHeader>
        <CardContent>
          <form onSubmit={(e) => void save(e)} className="flex flex-col gap-5">
            <Field data-invalid={error ? true : undefined}>
              <FieldLabel htmlFor="cidrs">One address or CIDR range per line</FieldLabel>
              <Textarea
                id="cidrs"
                rows={5}
                className="font-mono"
                placeholder={"10.20.0.0/16\n192.168.1.5"}
                aria-invalid={error ? true : undefined}
                value={cidrs}
                onChange={(e) => setCidrs(e.target.value)}
              />
              {error && <FieldError>{error}</FieldError>}
            </Field>
            <div>
              <Button type="submit" disabled={busy}>
                {busy && <Spinner data-icon="inline-start" />}
                Save allowlist
              </Button>
            </div>
          </form>
        </CardContent>
      </Card>
    </>
  );
}
