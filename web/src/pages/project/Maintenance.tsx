import { PausedNow } from "@/components/app/PausedNow";
import { useState } from "react";
import type { FormEvent, ReactElement } from "react";
import { useNavigate, useSearchParams } from "react-router-dom";
import { Link } from "react-router-dom";
import { ConfirmDialog } from "@/components/app/ConfirmDialog";
import { FormSheet, FormSheetBody } from "@/components/app/FormSheet";
import { TableCard } from "@/components/app/TableCard";
import { Plus, Trash2 } from "@/components/icons";
import { PageHeader } from "@/components/layout/Page";
import { Pager } from "@/components/list/Pager";
import { TableEmpty } from "@/components/list/TableEmpty";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Field, FieldError, FieldGroup, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { Spinner } from "@/components/ui/spinner";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Textarea } from "@/components/ui/textarea";
import { api, ApiError, dropField, errorText } from "@/lib/api";
import { useProject } from "@/lib/project";
import { Writable } from "@/lib/session";
import { between } from "@/lib/status";
import { browserOffset, fromInputValue, inputValue, nowSecs, offset, windowHint, zone } from "@/lib/time";
import type { IncidentItem, MaintenancePage, MaintenanceWindow, WindowState } from "@/lib/types";
import { useApi } from "@/lib/useApi";
import { ItemPicker } from "@/pages/project/IncidentForm";
import { toast } from "sonner";

const STATE: Record<WindowState, [string, "outline" | "neutral" | "unknown"]> = {
  scheduled: ["Scheduled", "outline"],
  active: ["Active", "neutral"],
  ended: ["Ended", "unknown"],
};

/** Delete while scheduled, End now while active; an ended window is a record. */
function WindowAction({ w, base, onDone }: { w: MaintenanceWindow; base: string; onDone: () => void }): ReactElement | null {
  if (w.state === "ended") return null;
  const scheduled = w.state === "scheduled";
  return (
    <ConfirmDialog
      trigger={
        <Button type="button" variant={scheduled ? "danger" : "outline"} size="sm">
          {scheduled && <Trash2 data-icon="inline-start" />}
          {scheduled ? "Delete" : "End now"}
        </Button>
      }
      title={scheduled ? `Delete "${w.title}"?` : `End "${w.title}" now?`}
      description={scheduled ? "The window will not start, and you cannot undo this." : "Failures open incidents and send alerts again from now on."}
      action={scheduled ? "Delete window" : "End now"}
      onConfirm={async () => {
        await (scheduled ? api(`${base}/${w.id}`, "DELETE") : api(`${base}/${w.id}/end`, "POST"));
        toast.success(scheduled ? "Window deleted." : "Window ended.");
        onDone();
      }}
    />
  );
}

/** Plan a window: a title, a start and an end in the admin's time zone, the items, and a note for visitors. */
function WindowForm({ base, onClose, onSaved }: { base: string; onClose: () => void; onSaved: () => void }): ReactElement {
  const { project: p } = useProject();
  const choices = useApi<IncidentItem[]>(`/projects/${p.slug}/incident-items`);
  const [title, setTitle] = useState("");
  const [starts, setStarts] = useState(() => inputValue(nowSecs()));
  const [ends, setEnds] = useState(() => inputValue(nowSecs() + 3600));
  // "Plan maintenance" on a monitor or heartbeat opens the form with it picked: ?item=monitor:3.
  const [params] = useSearchParams();
  const [items, setItems] = useState<string[]>(() => params.getAll("item"));
  const [message, setMessage] = useState("");
  const [error, setError] = useState<ApiError | null>(null);
  const [busy, setBusy] = useState(false);
  const err = (k: string): string | undefined => error?.fields[k];
  const clear = (k: string): void => setError((e) => e && dropField(e, k));

  const submit = async (_e: FormEvent): Promise<void> => {
    setBusy(true);
    setError(null);
    try {
      await api(base, "POST", { title, message, starts_at: fromInputValue(starts), ends_at: fromInputValue(ends), items });
      toast.success("Maintenance planned.");
      onSaved();
    } catch (e) {
      setError(e instanceof ApiError ? e : new ApiError(0, "error", errorText(e)));
    } finally {
      setBusy(false);
    }
  };
  const formError = error && Object.keys(error.fields).length === 0 ? error.message : undefined;
  const hint = windowHint(fromInputValue(starts), fromInputValue(ends), nowSecs(), offset(), browserOffset());

  return (
    <FormSheetBody
      title="Plan maintenance"
      onSubmit={(e) => void submit(e)}
      footer={
        <div className="ml-auto flex gap-2">
          <Button type="button" variant="ghost" onClick={onClose}>
            Cancel
          </Button>
          <Button type="submit" disabled={busy}>
            {busy && <Spinner data-icon="inline-start" />}
            Plan maintenance
          </Button>
        </div>
      }
    >
      {!choices.data ? (
        <Skeleton className="h-[480px] rounded-xl" />
      ) : (
        <FieldGroup className="gap-5">
          {formError && (
            <p role="alert" className="text-sm text-destructive">
              {formError}
            </p>
          )}
          <Field data-invalid={err("title") ? true : undefined}>
            <FieldLabel htmlFor="w-title" required>
              Title
            </FieldLabel>
            <Input id="w-title" autoFocus maxLength={120} aria-required aria-invalid={err("title") ? true : undefined} placeholder="Database upgrade" value={title} onChange={(e) => {
              setTitle(e.target.value);
              clear("title");
            }} />
            {err("title") && <FieldError>{err("title")}</FieldError>}
          </Field>
          <div className="grid gap-5 sm:grid-cols-2">
            <Field data-invalid={err("starts_at") ? true : undefined}>
              <FieldLabel htmlFor="w-starts" required>
                Starts, {zone()}
              </FieldLabel>
              <Input id="w-starts" type="datetime-local" aria-required aria-invalid={err("starts_at") ? true : undefined} value={starts} onChange={(e) => {
                setStarts(e.target.value);
                clear("starts_at");
              }} />
              {err("starts_at") && <FieldError>{err("starts_at")}</FieldError>}
            </Field>
            <Field data-invalid={err("ends_at") ? true : undefined}>
              <FieldLabel htmlFor="w-ends" required>
                Ends, {zone()}
              </FieldLabel>
              <Input id="w-ends" type="datetime-local" aria-required aria-invalid={err("ends_at") ? true : undefined} value={ends} onChange={(e) => {
                setEnds(e.target.value);
                clear("ends_at");
              }} />
              {err("ends_at") && <FieldError>{err("ends_at")}</FieldError>}
            </Field>
            {hint && <p className="text-sm text-muted-foreground sm:col-span-2">{hint}</p>}
          </div>
          <ItemPicker
            choices={choices.data}
            chosen={items}
            onToggle={(value, on) => {
              setItems((xs) => (on ? [...xs, value] : xs.filter((x) => x !== value)));
              clear("items");
            }}
            error={err("items")}
          />
          <Field data-invalid={err("message") ? true : undefined}>
            <FieldLabel htmlFor="w-message">Message on the status page</FieldLabel>
            <Textarea id="w-message" rows={4} maxLength={1000} aria-invalid={err("message") ? true : undefined} placeholder="What visitors may notice, and when it is back." value={message} onChange={(e) => {
              setMessage(e.target.value);
              clear("message");
            }} />
            {err("message") && <FieldError>{err("message")}</FieldError>}
          </Field>
        </FieldGroup>
      )}
    </FormSheetBody>
  );
}

/** Project, Maintenance: planned work, then everything paused. While a window is active its items
 * still get checked, but a failure opens no incident and sends no alert. */
export function Maintenance({ creating = false }: { creating?: boolean }): ReactElement {
  const navigate = useNavigate();
  const { project: p } = useProject();
  const base = `/projects/${p.slug}/maintenance`;
  const here = `/projects/${p.slug}/maintenance`;
  const [page, setPage] = useState(1);
  const list = useApi<MaintenancePage>(`${base}?page=${page}`);
  const d = list.data;

  return (
    <>
      <PageHeader
        title="Maintenance"
        actions={
          <>
            <Writable>
              <Button asChild>
                <Link to={`${here}/new`}>
                  <Plus data-icon="inline-start" />
                  Plan maintenance
                </Link>
              </Button>
            </Writable>
          </>
        }
      />
      <div className="flex flex-col gap-6 pb-4">
        <TableCard title={d ? `${d.total} ${d.total === 1 ? "window" : "windows"}` : "Windows"}>
          {!d ? (
            <Skeleton className="m-5 h-24" />
          ) : d.windows.length === 0 ? (
            <TableEmpty
              title="Plan maintenance before planned work"
              description="During a window Monita still checks the items, but a failure opens no incident and sends no alert, and the status page shows them as Maintenance."
            />
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Window</TableHead>
                  <TableHead className="max-md:hidden">When</TableHead>
                  <TableHead>State</TableHead>
                  <TableHead className="text-right">
                    <span className="sr-only">Action</span>
                  </TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {d.windows.map((w) => (
                  <TableRow key={w.id} data-window={w.id}>
                    <TableCell className="max-w-56 whitespace-normal sm:max-w-96">
                      <div className="font-medium">{w.title}</div>
                      <div className="text-xs text-muted-foreground">{w.items.map((i) => i.name).join(", ") || "Its items were deleted."}</div>
                    </TableCell>
                    <TableCell className="font-mono text-[13px] text-muted-foreground max-md:hidden">{between(w.starts_at, w.ends_at)}</TableCell>
                    <TableCell>
                      <Badge variant={STATE[w.state][1]}>{STATE[w.state][0]}</Badge>
                    </TableCell>
                    <TableCell className="text-right">
                      <WindowAction w={w} base={base} onDone={list.reload} />
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </TableCard>
        {d && <Pager page={page} pageSize={d.page_size} total={d.total} noun="window" onPage={setPage} disabled={list.loading} />}
        <PausedNow slug={p.slug} />
      </div>
      <FormSheet open={creating} onClose={() => navigate(here)}>
        <WindowForm
          base={base}
          onClose={() => navigate(here)}
          onSaved={() => {
            list.reload();
            navigate(here);
          }}
        />
      </FormSheet>
    </>
  );
}
