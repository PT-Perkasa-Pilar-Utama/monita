import type { ReactElement } from "react";
import { Link } from "react-router-dom";
import { ChoiceField } from "@/components/app/CheckForm";
import { Plus, X } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { Field, FieldError, FieldLabel, FieldLegend, FieldSet } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Select, SelectContent, SelectGroup, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Textarea } from "@/components/ui/textarea";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { AUTH_KINDS, CHECK_KINDS, targetField } from "@/lib/checks";
import type { AuthKind, HeaderRow } from "@/lib/checks";
import type { CheckKind, Choice, Secret } from "@/lib/types";

/** The part of the monitor form that says what one check sends. */
export type RequestPart = {
  kind: CheckKind;
  method: string;
  url: string;
  interval_s: number;
  timeout_s: string;
  body: string;
  auth_kind: AuthKind;
  auth_secret_id: number | null;
  auth_user: string;
  auth_header: string;
  header_rows: HeaderRow[];
};

type Props<F extends RequestPart> = {
  form: F;
  set: <K extends keyof F>(k: K, v: F[K]) => void;
  err: (k: string) => string | undefined;
  intervals: Choice[];
  /** The project's secrets; null while they load. */
  secrets: Secret[] | null;
  secretsHref: string;
};

const METHODS = ["GET", "HEAD", "POST", "PUT"];
export const sendsBody = (method: string): boolean => method === "POST" || method === "PUT";

function Pick({ id, value, items, onChange, mono, invalid, placeholder = "Pick one" }: { id: string; value: string; items: { value: string; label: string }[]; onChange: (v: string) => void; mono?: boolean; invalid?: boolean; placeholder?: string }): ReactElement {
  return (
    <Select value={value} onValueChange={onChange}>
      <SelectTrigger id={id} className={mono ? "font-mono" : undefined} aria-invalid={invalid ? true : undefined}>
        <SelectValue placeholder={placeholder} />
      </SelectTrigger>
      <SelectContent>
        <SelectGroup>
          {items.map((x) => (
            <SelectItem key={x.value} value={x.value} className={mono ? "font-mono" : undefined}>
              {x.label}
            </SelectItem>
          ))}
        </SelectGroup>
      </SelectContent>
    </Select>
  );
}

/** What to check, the target, how often and how long, and for HTTP the method and the body. */
export function RequestFields<F extends RequestPart>({ form, set, err, intervals }: Props<F>): ReactElement {
  const http = form.kind === "http";
  const target = targetField(form.kind);
  return (
    <FieldSet className="gap-5">
      <FieldLegend>Check</FieldLegend>
      <Field data-invalid={err("kind") ? true : undefined}>
        <FieldLabel id="m-kind-label">What it checks</FieldLabel>
        <ToggleGroup type="single" variant="outline" aria-labelledby="m-kind-label" value={form.kind} onValueChange={(v) => v && set("kind", v as CheckKind)} className="flex-wrap">
          {CHECK_KINDS.map((k) => (
            <ToggleGroupItem key={k.value} value={k.value} className="px-3">
              {k.label}
            </ToggleGroupItem>
          ))}
        </ToggleGroup>
        {err("kind") && <FieldError>{err("kind")}</FieldError>}
      </Field>
      <div className={http ? "grid gap-5 sm:grid-cols-[8rem_minmax(0,1fr)]" : "grid gap-5"}>
        {http && (
          <Field data-invalid={err("method") ? true : undefined}>
            <FieldLabel htmlFor="m-method">Method</FieldLabel>
            <Pick id="m-method" mono value={form.method} items={METHODS.map((m) => ({ value: m, label: m }))} onChange={(v) => set("method", v)} />
            {err("method") && <FieldError>{err("method")}</FieldError>}
          </Field>
        )}
        <Field data-invalid={err("url") ? true : undefined}>
          <FieldLabel htmlFor="m-url" required>
            {target.label}
          </FieldLabel>
          <Input id="m-url" type={http ? "url" : "text"} spellCheck={false} aria-required aria-invalid={err("url") ? true : undefined} className="font-mono" value={form.url} onChange={(e) => set("url", e.target.value)} placeholder={target.placeholder} />
          {err("url") && <FieldError>{err("url")}</FieldError>}
        </Field>
      </div>
      {http && sendsBody(form.method) && (
        <Field data-invalid={err("body") ? true : undefined}>
          <FieldLabel htmlFor="m-body">Request body</FieldLabel>
          <Textarea id="m-body" rows={4} spellCheck={false} aria-invalid={err("body") ? true : undefined} className="font-mono text-[13px]" value={form.body} onChange={(e) => set("body", e.target.value)} placeholder='{"ping": true}' />
          {err("body") && <FieldError>{err("body")}</FieldError>}
        </Field>
      )}
      <div className="grid gap-5 sm:grid-cols-2">
        <ChoiceField id="m-interval" label="Check every" value={form.interval_s} choices={intervals} onChange={(v) => set("interval_s", v)} error={err("interval_s")} />
        <Field data-invalid={err("timeout_s") ? true : undefined}>
          <FieldLabel htmlFor="m-timeout" required>
            Timeout
          </FieldLabel>
          <div className="flex items-center gap-3">
            <Input id="m-timeout" inputMode="numeric" className="w-24" aria-required aria-invalid={err("timeout_s") ? true : undefined} value={form.timeout_s} onChange={(e) => set("timeout_s", e.target.value)} placeholder="10" />
            <span className="text-sm text-muted-foreground">seconds</span>
          </div>
          {err("timeout_s") && <FieldError>{err("timeout_s")}</FieldError>}
        </Field>
      </div>
    </FieldSet>
  );
}

/** How the check signs in: a kind, a sealed secret of the project, and the kind's own field. */
export function AuthFields<F extends RequestPart>({ form, set, err, secrets, secretsHref }: Props<F>): ReactElement {
  const kind = form.auth_kind;
  return (
    <FieldSet className="gap-5">
      <FieldLegend>Authentication</FieldLegend>
      <div className="grid gap-5 sm:grid-cols-2">
        <Field data-invalid={err("auth_kind") ? true : undefined}>
          <FieldLabel htmlFor="m-auth">Sign in with</FieldLabel>
          <Pick id="m-auth" value={kind} items={[...AUTH_KINDS]} onChange={(v) => set("auth_kind", v as AuthKind)} />
          {err("auth_kind") && <FieldError>{err("auth_kind")}</FieldError>}
        </Field>
        {kind !== "none" && secrets && secrets.length > 0 && (
          <Field data-invalid={err("auth_secret_id") ? true : undefined}>
            <FieldLabel htmlFor="m-secret" required>
              {kind === "basic" ? "Password" : "Secret"}
            </FieldLabel>
            <Pick
              id="m-secret"
              mono
              placeholder="Pick a secret"
              invalid={!!err("auth_secret_id")}
              value={form.auth_secret_id === null ? "" : String(form.auth_secret_id)}
              items={secrets.map((s) => ({ value: String(s.id), label: s.name }))}
              onChange={(v) => set("auth_secret_id", Number(v))}
            />
            {err("auth_secret_id") && <FieldError>{err("auth_secret_id")}</FieldError>}
          </Field>
        )}
        {kind === "basic" && (
          <Field data-invalid={err("auth_user") ? true : undefined}>
            <FieldLabel htmlFor="m-user" required>
              User name
            </FieldLabel>
            <Input id="m-user" aria-required aria-invalid={err("auth_user") ? true : undefined} className="font-mono" value={form.auth_user} onChange={(e) => set("auth_user", e.target.value)} />
            {err("auth_user") && <FieldError>{err("auth_user")}</FieldError>}
          </Field>
        )}
        {kind === "header" && (
          <Field data-invalid={err("auth_header") ? true : undefined}>
            <FieldLabel htmlFor="m-key-header" required>
              Header name
            </FieldLabel>
            <Input id="m-key-header" aria-required aria-invalid={err("auth_header") ? true : undefined} className="font-mono" value={form.auth_header} onChange={(e) => set("auth_header", e.target.value)} placeholder="X-Api-Key" />
            {err("auth_header") && <FieldError>{err("auth_header")}</FieldError>}
          </Field>
        )}
      </div>
      {kind !== "none" && secrets?.length === 0 && (
        <p className="text-sm text-muted-foreground">
          This project has no secrets yet, so add one in{" "}
          <Link to={secretsHref} className="text-link underline-offset-4 hover:underline">
            Settings, Secrets
          </Link>
          , then pick it here.
        </p>
      )}
    </FieldSet>
  );
}

/** Plain headers as name and value rows. A token belongs under Authentication instead. */
export function HeaderFields<F extends RequestPart>({ form, set, err }: Props<F>): ReactElement {
  const rows = form.header_rows;
  const put = (i: number, part: keyof HeaderRow, v: string): void => set("header_rows", rows.map((r, j) => (j === i ? { ...r, [part]: v } : r)));
  return (
    <FieldSet className="gap-3" data-invalid={err("headers") ? true : undefined}>
      <FieldLegend>Headers</FieldLegend>
      {rows.length === 0 ? (
        <p className="text-sm text-muted-foreground">No headers yet, and a token belongs under Authentication so it stays sealed.</p>
      ) : (
        <div className="flex flex-col gap-2">
          {rows.map((r, i) => (
            <div key={i} className="grid grid-cols-[minmax(0,2fr)_minmax(0,3fr)_auto] gap-2">
              <Input aria-label={`Header ${i + 1} name`} className="font-mono text-[13px]" value={r.name} onChange={(e) => put(i, "name", e.target.value)} placeholder="X-Env" />
              <Input aria-label={`Header ${i + 1} value`} className="font-mono text-[13px]" value={r.value} onChange={(e) => put(i, "value", e.target.value)} placeholder="production" />
              <Button type="button" variant="ghost" size="icon" aria-label={`Remove header ${i + 1}`} onClick={() => set("header_rows", rows.filter((_, j) => j !== i))}>
                <X />
              </Button>
            </div>
          ))}
        </div>
      )}
      {err("headers") && <FieldError>{err("headers")}</FieldError>}
      <div>
        <Button type="button" variant="outline" size="sm" onClick={() => set("header_rows", [...rows, { name: "", value: "" }])}>
          <Plus data-icon="inline-start" />
          Add header
        </Button>
      </div>
    </FieldSet>
  );
}
