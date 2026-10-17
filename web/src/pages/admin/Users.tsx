import { useState } from "react";
import type { FormEvent, ReactElement } from "react";
import { ConfirmDialog } from "@/components/app/ConfirmDialog";
import { CopyField } from "@/components/app/CopyField";
import { TableCard } from "@/components/app/TableCard";
import { StateBadge } from "@/components/app/Tone";
import { Plus } from "@/components/icons";
import { PageHeader } from "@/components/layout/Page";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Field, FieldError, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { Spinner } from "@/components/ui/spinner";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { api, ApiError, errorText } from "@/lib/api";
import { utc } from "@/lib/time";
import type { AdminUserRow, Invited } from "@/lib/types";
import { useApi } from "@/lib/useApi";
import { toast } from "sonner";

const STATUS = {
  active: ["up", "Active"],
  invited: ["issue", "Invited"],
  disabled: ["unknown", "Disabled"],
} as const;

/** Admin, Users: every account, invites for tech leads, and disable. */
export function Users(): ReactElement {
  const users = useApi<AdminUserRow[]>("/admin/users");
  const [open, setOpen] = useState(false);
  const [email, setEmail] = useState("");
  const [error, setError] = useState<ApiError | null>(null);
  const [busy, setBusy] = useState(false);
  const [invited, setInvited] = useState<Invited | null>(null);

  const invite = async (e: FormEvent): Promise<void> => {
    e.preventDefault();
    setBusy(true);
    setError(null);
    try {
      setInvited(await api<Invited>("/admin/users", "POST", { email }));
      setEmail("");
      setOpen(false);
      users.reload();
    } catch (err) {
      setError(err instanceof ApiError ? err : new ApiError(0, "error", errorText(err)));
    } finally {
      setBusy(false);
    }
  };
  const list = users.data ?? [];
  const message = error?.fields.email ?? error?.message;

  return (
    <>
      <PageHeader
        title="Users"
        actions={
          <Button onClick={() => setOpen(true)}>
            <Plus data-icon="inline-start" />
            Invite tech lead
          </Button>
        }
      />
      {invited && (
        <Alert role="status" className="mb-5">
          <AlertTitle>Invite queued for {invited.email}. The link works once and expires in 72 hours.</AlertTitle>
          <AlertDescription className="mt-2">
            <CopyField id="invite-link" value={invited.link} />
          </AlertDescription>
        </Alert>
      )}
      <TableCard title={users.data ? `${list.length} ${list.length === 1 ? "user" : "users"}` : "Users"}>
        {!users.data ? (
          <Skeleton className="m-5 h-24" />
        ) : (
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Email</TableHead>
                <TableHead>Role</TableHead>
                <TableHead>Status</TableHead>
                <TableHead className="max-md:hidden">Joined</TableHead>
                <TableHead className="text-right">Actions</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {list.map((u) => (
                <TableRow key={u.id}>
                  <TableCell className="max-w-56 truncate font-medium">{u.email}</TableCell>
                  <TableCell>{u.role === "super_admin" ? "Super admin" : "Tech lead"}</TableCell>
                  <TableCell>
                    <StateBadge kind={STATUS[u.status][0]} label={STATUS[u.status][1]} />
                  </TableCell>
                  <TableCell className="font-mono text-[13px] max-md:hidden">{utc(u.created_at)}</TableCell>
                  <TableCell className="text-right">
                    {u.role === "tl" && u.status !== "disabled" && (
                      <span className="inline-flex gap-2">
                        <Button
                          variant="secondary"
                          size="sm"
                          onClick={() => {
                            api<Invited>(`/admin/users/${u.id}/invite`, "POST")
                              .then((i) => {
                                setInvited(i);
                                users.reload();
                              })
                              .catch((err: unknown) => toast.error(errorText(err)));
                          }}
                        >
                          New invite
                        </Button>
                        <ConfirmDialog
                          trigger={
                            <Button variant="danger" size="sm">
                              Disable
                            </Button>
                          }
                          title={`Disable ${u.email}?`}
                          description="Monita signs them out now. They can no longer sign in, and their open invites stop working. You cannot undo this."
                          action="Disable"
                          onConfirm={async () => {
                            await api(`/admin/users/${u.id}/disable`, "POST");
                            users.reload();
                          }}
                        />
                      </span>
                    )}
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        )}
      </TableCard>
      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent>
          <form onSubmit={(e) => void invite(e)} noValidate className="flex flex-col gap-6">
            <DialogHeader>
              <DialogTitle>Invite a tech lead</DialogTitle>
              <DialogDescription>Monita emails a sign-up link that works once and expires in 72 hours.</DialogDescription>
            </DialogHeader>
            <Field data-invalid={message ? true : undefined}>
              <FieldLabel htmlFor="invite-email" required>
                Email
              </FieldLabel>
              <Input id="invite-email" type="email" autoFocus aria-required aria-invalid={message ? true : undefined} value={email} onChange={(e) => setEmail(e.target.value)} />
              {message && <FieldError>{message}</FieldError>}
            </Field>
            <DialogFooter>
              <DialogClose asChild>
                <Button type="button" variant="ghost">
                  Cancel
                </Button>
              </DialogClose>
              <Button type="submit" disabled={busy}>
                {busy && <Spinner data-icon="inline-start" />}
                Send invite
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>
    </>
  );
}
