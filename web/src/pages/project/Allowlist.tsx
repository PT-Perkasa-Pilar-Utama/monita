import { useState } from "react";
import type { ReactElement } from "react";
import { SettingsPage, SettingsSection } from "@/components/app/SettingsSection";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
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
  const saved = (p.ip_allowlist ?? "").replaceAll(",", "\n");
  const save = async (): Promise<void> => {
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
    <SettingsPage title="Allowlist">
      <SettingsSection
        title="Allowed IP addresses"
        description="Only these addresses may use this project's API keys, and an empty list allows any address."
        action={<Badge variant={p.ip_allowlist ? "up" : "unknown"}>{p.ip_allowlist ? "On" : "Off"}</Badge>}
        onSubmit={() => void save()}
        footer={
          <Button type="submit" disabled={busy || cidrs.trim() === saved}>
            {busy && <Spinner data-icon="inline-start" />}
            Save changes
          </Button>
        }
      >
        <Field data-invalid={error ? true : undefined}>
          <FieldLabel htmlFor="cidrs">Allowed addresses</FieldLabel>
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
      </SettingsSection>
    </SettingsPage>
  );
}
