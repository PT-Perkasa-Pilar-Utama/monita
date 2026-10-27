// The frontend's one door to the server: JSON under /api/ui, in the api-specs 01 envelope.

import { BASE } from "@/lib/base";

export type FieldErrors = Record<string, string>;

export class ApiError extends Error {
  readonly status: number;
  readonly code: string;
  readonly fields: FieldErrors;

  constructor(status: number, code: string, message: string, fields: FieldErrors = {}) {
    super(message);
    this.status = status;
    this.code = code;
    this.fields = fields;
  }
}

/** The error once the user changes `field`: that field's message goes, and with no field left
 * the whole error goes, since a form shows the top message under a field when none is named. */
export function dropField(error: ApiError, field: string): ApiError | null {
  if (!(field in error.fields)) return error;
  const { [field]: _, ...rest } = error.fields;
  return Object.keys(rest).length === 0 ? null : new ApiError(error.status, error.code, error.message, rest);
}

type Method = "GET" | "POST" | "PUT" | "PATCH" | "DELETE";

/** Calls `/api/ui${path}` and returns `data`. A 401 sends the reader to sign in. */
export async function api<T>(path: string, method: Method = "GET", body?: unknown, cache?: RequestCache): Promise<T> {
  // The viewer's clock, so the server lays out chart labels and day bars in the viewer's time.
  const clock = { "x-utc-offset": String(-new Date().getTimezoneOffset()) };
  const init: RequestInit = { method, credentials: "same-origin", headers: { accept: "application/json", ...clock } };
  if (cache) init.cache = cache;
  if (body !== undefined) {
    init.headers = { accept: "application/json", "content-type": "application/json", ...clock };
    init.body = JSON.stringify(body);
  }
  const res = await fetch(`/api/ui${path}`, init);
  if (res.status === 204) return undefined as T;
  const json: unknown = await res.json().catch(() => null);
  if (!res.ok) {
    const e = (json as { error?: { code?: string; message?: string; fields?: FieldErrors } } | null)?.error;
    if (res.status === 401 && path !== "/login" && window.location.pathname !== `${BASE}/login`) {
      const next = window.location.pathname + window.location.search;
      window.location.assign(`${BASE}/login?next=${encodeURIComponent(next)}`);
    }
    throw new ApiError(res.status, e?.code ?? "error", e?.message ?? `The server answered ${res.status}.`, e?.fields ?? {});
  }
  return (json as { data: T }).data;
}

/** The message to show for a failed call. */
export function errorText(e: unknown): string {
  return e instanceof Error ? e.message : "Something went wrong, so try again.";
}
