import { useState } from "react";
import type { FormEvent, ReactElement } from "react";
import { useOutletContext } from "react-router-dom";
import { TableCard } from "@/components/app/TableCard";
import { PageHeader } from "@/components/layout/Page";
import { TableEmpty } from "@/components/list/TableEmpty";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Field, FieldError, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Spinner } from "@/components/ui/spinner";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { api, ApiError, errorText } from "@/lib/api";
import { reasonLabel, stageLabel } from "@/lib/tickets";
import { stamp } from "@/lib/time";
import type { MyTicket, PublicProject } from "@/lib/types";

/** A reporter's own tickets, found by the ref ID they reported with. */
export function Mine(): ReactElement {
  const project = useOutletContext<PublicProject>();
  const [refId, setRefId] = useState("");
  const [tickets, setTickets] = useState<MyTicket[] | null>(null);
  const [error, setError] = useState<ApiError | null>(null);
  const [busy, setBusy] = useState(false);

  const submit = async (e: FormEvent): Promise<void> => {
    e.preventDefault();
    setBusy(true);
    setError(null);
    try {
      const res = await api<{ tickets: MyTicket[] }>(`/public/${project.slug}/mine`, "POST", { ref_id: refId });
      setTickets(res.tickets);
    } catch (err) {
      setTickets(null);
      setError(err instanceof ApiError ? err : new ApiError(0, "error", errorText(err)));
    } finally {
      setBusy(false);
    }
  };
  const message = error?.fields.ref_id ?? error?.message;

  return (
    <div className="mx-auto flex max-w-3xl flex-col gap-6">
      <PageHeader title={`Your tickets in ${project.name}`} className="mb-0" />
      <Card>
        <CardContent>
          <form onSubmit={(e) => void submit(e)} noValidate className="flex flex-wrap items-end gap-3">
            <Field data-invalid={message ? true : undefined} className="min-w-0 flex-1 basis-64">
              <FieldLabel htmlFor="m-ref" required>
                Ref ID
              </FieldLabel>
              <Input id="m-ref" autoComplete="off" spellCheck={false} className="font-mono" aria-required aria-invalid={message ? true : undefined} value={refId} onChange={(e) => setRefId(e.target.value)} />
              {message && <FieldError>{message}</FieldError>}
            </Field>
            <Button type="submit" disabled={busy} className={message ? "mb-7" : undefined}>
              {busy && <Spinner data-icon="inline-start" />}
              Show my tickets
            </Button>
          </form>
        </CardContent>
      </Card>
      {tickets !== null && (
        <TableCard title={`${tickets.length} ${tickets.length === 1 ? "ticket" : "tickets"}`}>
          {tickets.length === 0 ? (
            <TableEmpty title="You have not reported a bug with this ref ID yet." />
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Ticket</TableHead>
                  <TableHead>Column</TableHead>
                  <TableHead className="text-right max-sm:hidden">Updated</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {tickets.map((t) => (
                  <TableRow key={t.number}>
                    <TableCell className="max-w-44 whitespace-normal sm:max-w-96">
                      <span className="font-medium">{t.title}</span>
                      <div className="font-mono text-xs text-muted-foreground">#{t.number}</div>
                    </TableCell>
                    <TableCell>
                      <span className="flex flex-wrap gap-1.5">
                        <Badge variant="neutral">{stageLabel(t.stage)}</Badge>
                        {t.close_reason && <Badge variant="outline">{reasonLabel(t.close_reason)}</Badge>}
                      </span>
                    </TableCell>
                    <TableCell className="text-right font-mono text-[13px] text-muted-foreground max-sm:hidden">{stamp(t.updated_at)}</TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </TableCard>
      )}
    </div>
  );
}
