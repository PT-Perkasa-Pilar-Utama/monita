import { useState } from "react";
import type { FormEvent, ReactElement } from "react";
import { toast } from "sonner";
import { ChevronDown, FileDown } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import { Field, FieldError, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Spinner } from "@/components/ui/spinner";
import { apiFile, errorText } from "@/lib/api";

const PERIODS = [
  { period: "7d", label: "Last 7 days" },
  { period: "month", label: "Last month" },
  { period: "quarter", label: "Last quarter" },
] as const;

/** The project's summary report as a PDF (v1.2 Q59): a fixed period from the menu, or days picked
 * in a dialog. The server checks the range, so a refused one shows the server's message. */
export function ReportMenu({ slug }: { slug: string }): ReactElement {
  const base = `/projects/${slug}/report`;
  const [busy, setBusy] = useState(false);
  const [picking, setPicking] = useState(false);
  const [from, setFrom] = useState("");
  const [to, setTo] = useState("");
  const [error, setError] = useState<string | null>(null);

  const get = async (query: string): Promise<boolean> => {
    setBusy(true);
    try {
      await apiFile(`${base}?${query}`);
      return true;
    } catch (e) {
      if (picking) setError(errorText(e));
      else toast.error(errorText(e));
      return false;
    } finally {
      setBusy(false);
    }
  };

  const submit = async (e: FormEvent): Promise<void> => {
    e.preventDefault();
    if (await get(new URLSearchParams({ from, to }).toString())) setPicking(false);
  };

  return (
    <>
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <Button variant="outline" disabled={busy}>
            {busy && !picking ? <Spinner data-icon="inline-start" /> : <FileDown data-icon="inline-start" />}
            Download report
            <ChevronDown data-icon="inline-end" />
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end">
          {PERIODS.map((p) => (
            <DropdownMenuItem key={p.period} onSelect={() => void get(`period=${p.period}`)}>
              {p.label}
            </DropdownMenuItem>
          ))}
          <DropdownMenuItem
            onSelect={() => {
              setError(null);
              setPicking(true);
            }}
          >
            Pick the days
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
      <Dialog open={picking} onOpenChange={setPicking}>
        <DialogContent>
          <form onSubmit={(e) => void submit(e)} className="flex flex-col gap-4">
            <DialogHeader>
              <DialogTitle>Download a report</DialogTitle>
              <DialogDescription>The report covers both days and every day between them, up to 366 days.</DialogDescription>
            </DialogHeader>
            <div className="flex flex-wrap gap-4">
              <Field className="min-w-0 flex-1">
                <FieldLabel htmlFor="report-from">First day</FieldLabel>
                <Input id="report-from" type="date" required value={from} max={to || undefined} onChange={(e) => (setFrom(e.target.value), setError(null))} />
              </Field>
              <Field className="min-w-0 flex-1">
                <FieldLabel htmlFor="report-to">Last day</FieldLabel>
                <Input id="report-to" type="date" required value={to} min={from || undefined} onChange={(e) => (setTo(e.target.value), setError(null))} />
              </Field>
            </div>
            {error && <FieldError>{error}</FieldError>}
            <DialogFooter>
              <Button type="submit" disabled={busy}>
                {busy ? <Spinner data-icon="inline-start" /> : <FileDown data-icon="inline-start" />}
                Download
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>
    </>
  );
}

/** One incident's report as a PDF, post-mortem included. `path` is the incident's page path.
 * The post-mortem section offers the same file under its own label. */
export function IncidentReportButton({ path, label, size }: { path: string; label: string; size?: "sm" }): ReactElement {
  const [busy, setBusy] = useState(false);
  const get = async (): Promise<void> => {
    setBusy(true);
    try {
      await apiFile(`${path}/report`);
    } catch (e) {
      toast.error(errorText(e));
    } finally {
      setBusy(false);
    }
  };
  return (
    <Button variant="outline" size={size} disabled={busy} onClick={() => void get()}>
      {busy ? <Spinner data-icon="inline-start" /> : <FileDown data-icon="inline-start" />}
      {label}
    </Button>
  );
}
