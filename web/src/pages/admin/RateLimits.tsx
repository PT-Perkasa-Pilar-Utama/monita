import { useState } from "react";
import { Writable } from "@/lib/session";
import type { FormEvent, ReactElement } from "react";
import { SettingsSection } from "@/components/app/SettingsSection";
import { PageHeader } from "@/components/layout/Page";
import { Button } from "@/components/ui/button";
import { Field, FieldError, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { Spinner } from "@/components/ui/spinner";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { api, ApiError, errorText } from "@/lib/api";
import type { LimitRow, SystemView } from "@/lib/types";
import { useApi } from "@/lib/useApi";
import { cn } from "@/lib/utils";
import { toast } from "sonner";

/** One limit as a GitHub settings row: what it is on the left, the number on the right. */
function LimitField({ l, value, error, onChange }: { l: LimitRow; value: string; error: string | undefined; onChange: (v: string) => void }): ReactElement {
  return (
    <Field data-invalid={error ? true : undefined} className="grid gap-x-8 gap-y-2 px-6 py-4 sm:grid-cols-[minmax(0,1fr)_auto] sm:items-center">
      <div className="flex min-w-0 flex-col gap-0.5">
        <FieldLabel htmlFor={`limit-${l.key}`}>{l.label}</FieldLabel>
        <p className="text-[13px] text-muted-foreground">{l.help}</p>
        {error && <FieldError>{error}</FieldError>}
      </div>
      <div className="flex items-center gap-3 sm:flex-col sm:items-end sm:gap-1">
        <Input
          id={`limit-${l.key}`}
          type="number"
          min={1}
          max={l.max}
          aria-required
          aria-invalid={error ? true : undefined}
          className="w-28 text-right font-mono tabular-nums"
          value={value}
          onChange={(e) => onChange(e.target.value)}
        />
        <span className={cn("text-xs text-muted-foreground", value !== String(l.default) && "text-foreground")}>
          Default {l.default.toLocaleString()}
        </span>
      </div>
    </Field>
  );
}

function LimitsForm({ d, onSaved }: { d: SystemView; onSaved: () => void }): ReactElement {
  const initial = Object.fromEntries(d.limits.map((l) => [l.key, String(l.value)]));
  const [values, setValues] = useState<Record<string, string>>(initial);
  const [error, setError] = useState<ApiError | null>(null);
  const [busy, setBusy] = useState(false);
  const changed = d.limits.some((l) => values[l.key] !== initial[l.key]);
  const save = async (e: FormEvent): Promise<void> => {
    e.preventDefault();
    setBusy(true);
    setError(null);
    try {
      await api("/admin/limits", "PUT", values);
      toast.success("Limits saved.");
      onSaved();
    } catch (err) {
      setError(err instanceof ApiError ? err : new ApiError(0, "error", errorText(err)));
    } finally {
      setBusy(false);
    }
  };
  return (
    <form onSubmit={(e) => void save(e)} noValidate className="flex flex-col gap-6">
      {d.groups.map((g) => (
        <SettingsSection key={g} title={g} flush>
          <div className="divide-y">
            {d.limits
              .filter((l) => l.group === g)
              .map((l) => (
                <LimitField key={l.key} l={l} value={values[l.key] ?? ""} error={error?.fields[l.key]} onChange={(v) => setValues((s) => ({ ...s, [l.key]: v }))} />
              ))}
          </div>
        </SettingsSection>
      ))}
      <Writable>
        <div className="flex items-center justify-end gap-3">
          {error && Object.keys(error.fields).length === 0 && <p className="mr-auto text-sm text-destructive">{error.message}</p>}
          <Button type="button" variant="ghost" disabled={!changed || busy} onClick={() => setValues(initial)}>
            Discard changes
          </Button>
          <Button type="submit" disabled={!changed || busy}>
            {busy && <Spinner data-icon="inline-start" />}
            Save limits
          </Button>
        </div>
      </Writable>
    </form>
  );
}

/** Admin, Rate limits: the limits a super admin can change, and the maximums built into Monita. */
export function RateLimits(): ReactElement {
  const res = useApi<SystemView>("/admin/system");
  const d = res.data;
  return (
    <>
      <PageHeader title="Rate limits" />
      <p className="-mt-2 mb-6 max-w-3xl text-sm text-muted-foreground">
        Limits keep Monita fast and stop one visitor or app from overloading it. The defaults suit most teams. Change one only when something hits it.
      </p>
      {!d ? (
        <Skeleton className="h-96 rounded-xl" />
      ) : (
        <div className="flex flex-col gap-6">
          <LimitsForm key={d.limits.map((l) => l.value).join()} d={d} onSaved={res.reload} />
          <SettingsSection title="Built-in maximums" description="Monita stops at these numbers to stay quick. You cannot change them here. When a list reaches its maximum, adding one more is refused." flush>
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead className="pl-6">What</TableHead>
                      <TableHead className="text-right">In use</TableHead>
                      <TableHead className="pr-6 text-right">Maximum</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {d.caps.map((c) => (
                      <TableRow key={c.label}>
                        <TableCell className="pl-6">{c.label}</TableCell>
                        <TableCell className="text-right tabular-nums">{c.used === null ? "–" : c.used.toLocaleString()}</TableCell>
                        <TableCell className="pr-6 text-right tabular-nums">{c.max.toLocaleString()}</TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              </SettingsSection>
        </div>
      )}
    </>
  );
}
